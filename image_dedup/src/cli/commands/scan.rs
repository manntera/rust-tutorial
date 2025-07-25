use crate::core::{
    traits::ProcessingConfig, DefaultConfig, HighPerformanceConfig, StaticDIContainer,
    TestingConfig,
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

/// Execute scan command with DefaultConfig
pub async fn execute_scan_with_default_config(config: ScanConfig) -> Result<()> {
    execute_scan_with_static_config::<DefaultConfig>(config).await
}

/// Execute scan command with HighPerformanceConfig
pub async fn execute_scan_with_high_performance_config(config: ScanConfig) -> Result<()> {
    execute_scan_with_static_config::<HighPerformanceConfig>(config).await
}

/// Execute scan command with TestingConfig
pub async fn execute_scan_with_testing_config(config: ScanConfig) -> Result<()> {
    execute_scan_with_static_config::<TestingConfig>(config).await
}

/// Generic scan execution with static dispatch
async fn execute_scan_with_static_config<C>(config: ScanConfig) -> Result<()>
where
    C: crate::core::StaticDependencyProvider + crate::core::static_config::TypeConfig,
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

    println!("🚀 画像重複検出ツール - scanコマンド");
    println!("📂 対象ディレクトリ: {}", config.target_directory.display());
    println!("📄 出力ファイル: {}", config.output.display());
    println!("⚙️  設定: {}", C::NAME);
    println!("📋 説明: {}", C::DESCRIPTION);

    // Create static DI container and processing engine
    let container = StaticDIContainer::<C>::new();
    let engine = container.create_processing_engine(&config.output);

    println!("⚙️  エンジン設定:");
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

/// Unified scan command with static dispatch selection
pub async fn execute_scan(
    target_directory: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    force: bool,
    algorithm: String,
    hash_size: u32,
    config_preset: Option<String>,
) -> Result<()> {
    let scan_config = ScanConfig {
        target_directory,
        output,
        threads,
        force,
    };

    // Select configuration based on preset or algorithm
    let preset = config_preset.unwrap_or_else(|| {
        if hash_size >= 32 || threads.unwrap_or(0) >= 8 {
            "high_performance".to_string()
        } else if algorithm == "average" {
            "testing".to_string()
        } else {
            "default".to_string()
        }
    });

    match preset.as_str() {
        "default" => execute_scan_with_default_config(scan_config).await,
        "high_performance" => execute_scan_with_high_performance_config(scan_config).await,
        "testing" => execute_scan_with_testing_config(scan_config).await,
        _ => {
            anyhow::bail!(
                "Unknown configuration preset: {}. Available: default, high_performance, testing",
                preset
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
    async fn test_scan_with_config_presets() {
        let target_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        // テスト用の設定プリセット
        let configs = ["default", "high_performance", "testing"];

        for config_preset in configs {
            let output = output_dir
                .path()
                .join(format!("output_{config_preset}.json"));
            let scan_config = ScanConfig {
                target_directory: target_dir.path().to_path_buf(),
                output,
                threads: Some(1),
                force: true,
            };

            let result = match config_preset {
                "default" => execute_scan_with_default_config(scan_config).await,
                "high_performance" => execute_scan_with_high_performance_config(scan_config).await,
                "testing" => execute_scan_with_testing_config(scan_config).await,
                _ => unreachable!("All config presets are defined in the array above"),
            };

            // 空のディレクトリなので処理は成功するはず
            assert!(result.is_ok(), "Config {config_preset} should work");
        }
    }
}
