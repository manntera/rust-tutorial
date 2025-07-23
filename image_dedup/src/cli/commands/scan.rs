use crate::core::{DependencyContainer, DependencyContainerBuilder};
use anyhow::Result;
use std::path::PathBuf;

/// Configuration struct for scan command to reduce argument count
pub struct ScanConfig {
    pub target_directory: PathBuf,
    pub output: PathBuf,
    pub threads: Option<usize>,
    pub force: bool,
}

/// Unified scan command using DI container
pub async fn execute_scan_with_container(
    config: ScanConfig,
    container: DependencyContainer,
) -> Result<()> {
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

    // Resolve all dependencies from container
    let dependencies = container.resolve_all_dependencies(&config.output)?;
    
    // Build processing engine using resolved dependencies
    let engine = dependencies.create_processing_engine();

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

/// Unified scan command with dynamic dependency injection
pub async fn execute_scan(
    target_directory: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    force: bool,
    algorithm: String,
    hash_size: u32,
    config_file: Option<PathBuf>,
) -> Result<()> {
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

    // DIコンテナを構築（アルゴリズムとパラメータを指定）
    let thread_count = threads.unwrap_or_else(num_cpus::get);
    
    let quality_factor = if algorithm == "dct" { 1.0 } else { 0.0 };
    
    let container = DependencyContainerBuilder::new()
        .with_image_loader("standard", serde_json::json!({
            "max_dimension": 512
        }))
        .with_perceptual_hash(&algorithm, serde_json::json!({
            "size": hash_size,
            "quality_factor": quality_factor
        }))
        .with_storage("local", serde_json::json!({}))
        .with_processing_config("default", serde_json::json!({
            "max_concurrent": thread_count * 2,
            "buffer_size": 100,
            "batch_size": 50,
            "enable_progress": true
        }))
        .with_progress_reporter("console", serde_json::json!({
            "quiet": false
        }))
        .with_hash_persistence("streaming_json", serde_json::json!({
            "buffer_size": 100
        }))
        .build();

    execute_scan_with_container(scan_config, container).await
}


/// 設定ファイルから読み込んでスキャンを実行
async fn execute_scan_from_config_file(config: ScanConfig, config_path: PathBuf) -> Result<()> {
    println!("📄 設定ファイル: {}", config_path.display());

    // DIコンテナを設定ファイルから作成
    let container = DependencyContainer::from_config_file(&config_path)
        .map_err(|e| anyhow::anyhow!("設定ファイルからのDIコンテナ作成エラー: {}", e))?;

    println!("✅ 設定ファイルから依存関係を正常に読み込みました");
    println!("🔧 ImageLoader: {}", container.config().image_loader.implementation);
    println!("🔧 PerceptualHash: {}", container.config().perceptual_hash.implementation);
    println!("🔧 Storage: {}", container.config().storage.implementation);
    println!("🔧 ProcessingConfig: {}", container.config().processing_config.implementation);
    println!("🔧 ProgressReporter: {}", container.config().progress_reporter.implementation);
    println!("🔧 HashPersistence: {}", container.config().hash_persistence.implementation);

    execute_scan_with_container(config, container).await
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

        let result = execute_scan(
            nonexistent_dir,
            output,
            None,
            false,
            "dct".to_string(),
            8,
            None,
        )
        .await;
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

        let result = execute_scan(
            target_dir.path().to_path_buf(),
            output,
            None,
            false,
            "dct".to_string(),
            8,
            None,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_scan_with_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");
        let output_path = temp_dir.path().join("output.json");
        let target_dir = TempDir::new().unwrap();

        // テスト用設定ファイルを作成
        let test_config = crate::core::DependencyConfig::for_testing();
        let config_json = serde_json::to_string_pretty(&test_config).unwrap();
        fs::write(&config_path, config_json).unwrap();

        let scan_config = ScanConfig {
            target_directory: target_dir.path().to_path_buf(),
            output: output_path,
            threads: Some(1),
            force: true,
        };

        let result = execute_scan_from_config_file(scan_config, config_path).await;
        // 空のディレクトリなので処理は成功するはず
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_scan_with_container() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.json");
        let target_dir = TempDir::new().unwrap();

        let scan_config = ScanConfig {
            target_directory: target_dir.path().to_path_buf(),
            output: output_path,
            threads: Some(1),
            force: true,
        };

        let container = DependencyContainer::with_preset("testing").unwrap();
        let result = execute_scan_with_container(scan_config, container).await;
        assert!(result.is_ok());
    }
}
