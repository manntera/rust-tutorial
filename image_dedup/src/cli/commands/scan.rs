use crate::{
    core::{HashPersistence, ProcessingConfig, ProgressReporter},
    engine::ProcessingEngine,
    image_loader::ImageLoaderBackend,
    image_loader::standard::StandardImageLoader,
    perceptual_hash::PerceptualHashBackend,
    perceptual_hash::config::{AlgorithmConfig, DynamicAlgorithmConfig},
    services::{ConsoleProgressReporter, DefaultProcessingConfig, StreamingJsonHashPersistence},
    storage::StorageBackend,
    storage::local::LocalStorageBackend,
};
use anyhow::Result;
use std::path::PathBuf;

/// Configuration struct for scan command to reduce argument count
pub struct ScanConfig {
    pub target_directory: PathBuf,
    pub output: PathBuf,
    pub threads: Option<usize>,
    pub force: bool,
}

/// Dependencies struct for scan command to reduce argument count
pub struct ScanDependencies<L, H, S, C, R, P> {
    pub loader: L,
    pub hasher: H,
    pub storage: S,
    pub config: C,
    pub reporter: R,
    pub persistence: P,
}

/// Zero-cost abstraction scan command with trait bounds
pub async fn execute_scan_generic<L, H, S, C, R, P>(
    config: ScanConfig,
    deps: ScanDependencies<L, H, S, C, R, P>,
) -> Result<()>
where
    L: ImageLoaderBackend + Send + Sync + Clone + 'static,
    H: PerceptualHashBackend + Send + Sync + Clone + 'static,
    S: StorageBackend + Send + Sync + 'static,
    C: ProcessingConfig + Send + Sync,
    R: ProgressReporter + Send + Sync + Clone + 'static,
    P: HashPersistence + Send + Sync + Clone + 'static,
{
    // Validate target directory
    if !config.target_directory.exists() {
        anyhow::bail!(
            "Target directory does not exist: {}",
            config.target_directory.display()
        );
    }

    if !config.target_directory.is_dir() {
        anyhow::bail!(
            "Target path is not a directory: {}",
            config.target_directory.display()
        );
    }

    // Check if output file exists and handle --force flag
    if config.output.exists() && !config.force {
        anyhow::bail!(
            "Output file already exists: {}. Use --force to overwrite.",
            config.output.display()
        );
    }

    // Create output directory if it doesn't exist
    if let Some(parent) = config.output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Determine thread count
    let thread_count = config.threads.unwrap_or_else(num_cpus::get);

    println!("🚀 画像重複検出ツール - scanコマンド");
    println!("📂 対象ディレクトリ: {}", config.target_directory.display());
    println!("📄 出力ファイル: {}", config.output.display());
    println!("🧵 使用スレッド数: {thread_count}");

    // Build processing engine using injected dependencies
    let engine = ProcessingEngine::new(
        deps.loader,
        deps.hasher,
        deps.storage,
        deps.config,
        deps.reporter,
        deps.persistence,
    );

    println!("⚙️  設定:");
    println!(
        "   - 最大並列数: {}",
        engine.config().max_concurrent_tasks()
    );
    println!("   - バッチサイズ: {}", engine.config().batch_size());
    println!(
        "   - バッファサイズ: {}",
        engine.config().channel_buffer_size()
    );

    // Execute processing
    let start_time = std::time::Instant::now();

    let target_str = config.target_directory.to_string_lossy();
    match engine.process_directory(&target_str).await {
        Ok(summary) => {
            let elapsed = start_time.elapsed();

            println!("\n✅ 処理完了!");
            println!("📊 処理結果:");
            println!("   - 対象ファイル数: {}", summary.total_files);
            println!("   - 成功処理数: {}", summary.processed_files);
            println!("   - エラー数: {}", summary.error_count);
            println!("   - 総処理時間: {:.2}秒", elapsed.as_secs_f64());
            println!(
                "   - 平均処理時間: {:.2}ms/ファイル",
                summary.average_time_per_file_ms
            );

            if summary.error_count > 0 {
                println!(
                    "⚠️  {}個のファイルでエラーが発生しました",
                    summary.error_count
                );
            }

            println!("📄 結果は {} に保存されました", config.output.display());
        }
        Err(error) => {
            anyhow::bail!("処理エラー: {}", error);
        }
    }

    Ok(())
}

/// Default convenience function for scan command
pub async fn execute_scan(
    target_directory: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    force: bool,
    algorithm: String,
    hash_size: u32,
    config_file: Option<PathBuf>,
) -> Result<()> {
    let _thread_count = threads.unwrap_or_else(num_cpus::get);

    let scan_config = ScanConfig {
        target_directory,
        output: output.clone(),
        threads,
        force,
    };

    // 設定ファイルが指定されている場合は設定ファイルから読み込み
    if let Some(config_path) = config_file {
        return execute_scan_from_config_file(scan_config, config_path).await;
    }

    // ハッシャーを作成（具体的な型で分岐）
    match algorithm.as_str() {
        "dct" => {
            let config = crate::perceptual_hash::dct_config::DctConfig {
                size: hash_size,
                quality_factor: 1.0,
            };
            config.validate()?;
            let hasher = config.create_hasher()?;
            execute_scan_with_dct_hasher(scan_config, hasher).await
        }
        "average" => {
            let config = crate::perceptual_hash::average_config::AverageConfig { size: hash_size };
            config.validate()?;
            let hasher = config.create_hasher()?;
            execute_scan_with_average_hasher(scan_config, hasher).await
        }
        "difference" => {
            let config =
                crate::perceptual_hash::difference_config::DifferenceConfig { size: hash_size };
            config.validate()?;
            let hasher = config.create_hasher()?;
            execute_scan_with_difference_hasher(scan_config, hasher).await
        }
        _ => {
            anyhow::bail!(
                "Unsupported algorithm: {}. Available algorithms: dct, average, difference",
                algorithm
            );
        }
    }
}

/// DCTハッシャー用の専用scan実装
async fn execute_scan_with_dct_hasher(
    config: ScanConfig,
    hasher: crate::perceptual_hash::dct_hash::DctHasher,
) -> Result<()> {
    let thread_count = config.threads.unwrap_or_else(num_cpus::get);
    let output = &config.output;

    let persistence = StreamingJsonHashPersistence::new(output);

    // DCT設定情報を設定
    let dct_params = serde_json::json!({
        "size": hasher.get_size(),
        "quality_factor": hasher.get_quality_factor()
    });
    persistence
        .set_scan_info("dct".to_string(), dct_params)
        .await?;

    let scan_deps = ScanDependencies {
        loader: StandardImageLoader::with_max_dimension(512),
        hasher,
        storage: LocalStorageBackend::new(),
        config: DefaultProcessingConfig::new(thread_count)
            .with_max_concurrent(thread_count * 2)
            .with_batch_size(50)
            .with_progress_reporting(true),
        reporter: ConsoleProgressReporter::new(),
        persistence,
    };

    execute_scan_generic(config, scan_deps).await
}

/// Averageハッシャー用の専用scan実装
async fn execute_scan_with_average_hasher(
    config: ScanConfig,
    hasher: crate::perceptual_hash::average_hash::AverageHasher,
) -> Result<()> {
    let thread_count = config.threads.unwrap_or_else(num_cpus::get);
    let output = &config.output;

    let persistence = StreamingJsonHashPersistence::new(output);

    // Average設定情報を設定
    let avg_params = serde_json::json!({
        "size": hasher.get_size()
    });
    persistence
        .set_scan_info("average".to_string(), avg_params)
        .await?;

    let scan_deps = ScanDependencies {
        loader: StandardImageLoader::with_max_dimension(512),
        hasher,
        storage: LocalStorageBackend::new(),
        config: DefaultProcessingConfig::new(thread_count)
            .with_max_concurrent(thread_count * 2)
            .with_batch_size(50)
            .with_progress_reporting(true),
        reporter: ConsoleProgressReporter::new(),
        persistence,
    };

    execute_scan_generic(config, scan_deps).await
}

/// Differenceハッシャー用の専用scan実装  
async fn execute_scan_with_difference_hasher(
    config: ScanConfig,
    hasher: crate::perceptual_hash::average_hash::DifferenceHasher,
) -> Result<()> {
    let thread_count = config.threads.unwrap_or_else(num_cpus::get);
    let output = &config.output;

    let persistence = StreamingJsonHashPersistence::new(output);

    // Difference設定情報を設定
    let diff_params = serde_json::json!({
        "size": hasher.get_size()
    });
    persistence
        .set_scan_info("difference".to_string(), diff_params)
        .await?;

    let scan_deps = ScanDependencies {
        loader: StandardImageLoader::with_max_dimension(512),
        hasher,
        storage: LocalStorageBackend::new(),
        config: DefaultProcessingConfig::new(thread_count)
            .with_max_concurrent(thread_count * 2)
            .with_batch_size(50)
            .with_progress_reporting(true),
        reporter: ConsoleProgressReporter::new(),
        persistence,
    };

    execute_scan_generic(config, scan_deps).await
}

/// 設定ファイルから読み込んでスキャンを実行
async fn execute_scan_from_config_file(config: ScanConfig, config_path: PathBuf) -> Result<()> {
    // 設定ファイルを読み込み
    let config_json = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("設定ファイルの読み込みエラー: {}", e))?;

    // JSONを解析
    let dynamic_config: DynamicAlgorithmConfig = serde_json::from_str(&config_json)
        .map_err(|e| anyhow::anyhow!("設定ファイルの解析エラー: {}", e))?;

    println!("📄 設定ファイル: {}", config_path.display());
    println!("🔧 アルゴリズム: {}", dynamic_config.algorithm);
    println!(
        "⚙️  パラメータ: {}",
        serde_json::to_string_pretty(&dynamic_config.parameters)?
    );

    // アルゴリズムに応じて適切な関数を呼び出し
    match dynamic_config.algorithm.as_str() {
        "dct" => {
            let dct_config: crate::perceptual_hash::dct_config::DctConfig =
                serde_json::from_value(dynamic_config.parameters)
                    .map_err(|e| anyhow::anyhow!("DCT設定の解析エラー: {}", e))?;
            dct_config.validate()?;
            let hasher = dct_config.create_hasher()?;
            execute_scan_with_dct_hasher(config, hasher).await
        }
        "average" => {
            let avg_config: crate::perceptual_hash::average_config::AverageConfig =
                serde_json::from_value(dynamic_config.parameters)
                    .map_err(|e| anyhow::anyhow!("Average設定の解析エラー: {}", e))?;
            avg_config.validate()?;
            let hasher = avg_config.create_hasher()?;
            execute_scan_with_average_hasher(config, hasher).await
        }
        "difference" => {
            let diff_config: crate::perceptual_hash::difference_config::DifferenceConfig =
                serde_json::from_value(dynamic_config.parameters)
                    .map_err(|e| anyhow::anyhow!("Difference設定の解析エラー: {}", e))?;
            diff_config.validate()?;
            let hasher = diff_config.create_hasher()?;
            execute_scan_with_difference_hasher(config, hasher).await
        }
        _ => {
            anyhow::bail!(
                "サポートされていないアルゴリズム: {}. 利用可能: dct, average, difference",
                dynamic_config.algorithm
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_nonexistent_directory() {
        let nonexistent_dir = PathBuf::from("nonexistent_directory");
        let output = PathBuf::from("output.json");

        let result = execute_scan(nonexistent_dir, output, None, false, "dct".to_string(), 8, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_scan_file_instead_of_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        fs::write(&file_path, "test content").unwrap();

        let output = PathBuf::from("output.json");

        let result = execute_scan(file_path, output, None, false, "dct".to_string(), 8, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[tokio::test]
    async fn test_scan_existing_output_without_force() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("existing_output.json");
        fs::write(&output, "existing content").unwrap();

        let target_dir = TempDir::new().unwrap();

        let result = execute_scan(target_dir.path().to_path_buf(), output, None, false, "dct".to_string(), 8, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}
