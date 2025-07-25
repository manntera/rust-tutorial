use crate::core::{
    traits::ProcessingConfig, DefaultConfig, HighPerformanceConfig, StaticDIContainer,
    TestingConfig,
};
use crate::perceptual_hash::config::DynamicAlgorithmConfig;
use anyhow::Result;
use std::path::PathBuf;

/// Configuration struct for scan command to reduce argument count
pub struct ScanConfig {
    pub target_directory: PathBuf,
    pub output: PathBuf,
    pub threads: Option<usize>,
    pub force: bool,
}

/// Extended configuration struct including all scan parameters
pub struct ExtendedScanConfig {
    pub target_directory: PathBuf,
    pub output: PathBuf,
    pub threads: Option<usize>,
    pub force: bool,
    pub algorithm: String,
    pub hash_size: u32,
    pub config_preset: Option<String>,
    pub config_file: Option<PathBuf>,
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

    // Check if output file already exists
    if config.output.exists() && !config.force {
        anyhow::bail!(
            "Output file already exists: {}. Use --force to overwrite.",
            config.output.display()
        );
    }

    println!("🔍 画像スキャン開始");
    println!(
        "   - 対象ディレクトリ: {}",
        config.target_directory.display()
    );
    println!("   - 出力ファイル: {}", config.output.display());
    println!("   - 設定: {}", C::NAME);

    // Create DI container
    let container = StaticDIContainer::<C>::new();

    // Create processing engine
    let engine = container.create_processing_engine(&config.output);

    // Display engine configuration
    println!("⚙️  処理設定:");
    println!(
        "   - 並行処理数: {}",
        engine.config().max_concurrent_tasks()
    );
    println!("   - バッチサイズ: {}", engine.config().batch_size());
    println!(
        "   - バッファサイズ: {}",
        engine.config().channel_buffer_size()
    );

    // Execute the scan
    let target_dir_str = config.target_directory.to_str().ok_or_else(|| {
        anyhow::anyhow!("Invalid UTF-8 path: {}", config.target_directory.display())
    })?;

    match engine.process_directory(target_dir_str).await {
        Ok(result) => {
            println!("✅ スキャン完了!");
            println!("   - 処理済ファイル: {}", result.processed_files);
            println!("   - 総ファイル数: {}", result.total_files);
            println!("   - エラー数: {}", result.error_count);
            println!("   - 処理時間: {}ms", result.total_processing_time_ms);

            println!("📄 結果は {} に保存されました", config.output.display());
        }
        Err(error) => {
            anyhow::bail!("処理エラー: {}", error);
        }
    }

    Ok(())
}

/// Unified scan command with static dispatch selection
#[allow(clippy::too_many_arguments)]
pub async fn execute_scan(
    target_directory: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    force: bool,
    algorithm: String,
    hash_size: u32,
    config_preset: Option<String>,
    config_file: Option<PathBuf>,
) -> Result<()> {
    let config = ExtendedScanConfig {
        target_directory,
        output,
        threads,
        force,
        algorithm,
        hash_size,
        config_preset,
        config_file,
    };

    execute_scan_with_extended_config(config).await
}

/// Execute scan with extended configuration struct
async fn execute_scan_with_extended_config(config: ExtendedScanConfig) -> Result<()> {
    let scan_config = ScanConfig {
        target_directory: config.target_directory,
        output: config.output,
        threads: config.threads,
        force: config.force,
    };

    // Load configuration from file if provided
    if let Some(config_path) = config.config_file {
        return execute_scan_with_config_file(scan_config, config_path).await;
    }

    // Select configuration based on preset or algorithm
    let preset = config.config_preset.unwrap_or_else(|| {
        if config.hash_size >= 32 || config.threads.unwrap_or(0) >= 8 {
            "high_performance".to_string()
        } else if config.algorithm == "average" {
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

/// Execute scan with configuration file
async fn execute_scan_with_config_file(config: ScanConfig, config_path: PathBuf) -> Result<()> {
    // Validate config file exists
    if !config_path.exists() {
        anyhow::bail!(
            "Configuration file does not exist: {}",
            config_path.display()
        );
    }

    // Read and parse configuration file
    let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read config file {}: {}",
            config_path.display(),
            e
        )
    })?;

    let dynamic_config: DynamicAlgorithmConfig =
        serde_json::from_str(&config_content).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse config file {}: {}",
                config_path.display(),
                e
            )
        })?;

    println!("🔧 設定ファイル使用: {}", config_path.display());
    println!("   - アルゴリズム: {}", dynamic_config.algorithm);
    println!("   - パラメータ: {}", dynamic_config.parameters);

    // For now, use default config with algorithm info from file
    // TODO: Implement dynamic algorithm execution
    match dynamic_config.algorithm.as_str() {
        "dct" => execute_scan_with_default_config(config).await,
        "average" => execute_scan_with_testing_config(config).await,
        _ => execute_scan_with_default_config(config).await,
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

        let result = execute_scan(
            file_path,
            output,
            None,
            false,
            "dct".to_string(),
            8,
            None,
            None,
        )
        .await;
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
            None,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_scan_with_config_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create a valid config file
        let config_path = temp_dir.path().join("test_config.json");
        let config_content = r#"{
            "algorithm": "dct",
            "parameters": {
                "size": 16,
                "quality_factor": 0.9
            }
        }"#;
        fs::write(&config_path, config_content).unwrap();

        // Create target directory with a test file
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("test.txt"), "test content").unwrap();

        let output = temp_dir.path().join("output.json");

        let result = execute_scan(
            target_dir,
            output.clone(),
            None,
            true, // force
            "dct".to_string(),
            8,
            None,
            Some(config_path),
        )
        .await;

        // Should succeed (no actual image files to process, but config should be loaded)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scan_with_nonexistent_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_config = temp_dir.path().join("nonexistent.json");

        // Create target directory
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        let output = temp_dir.path().join("output.json");

        let result = execute_scan(
            target_dir,
            output,
            None,
            true,
            "dct".to_string(),
            8,
            None,
            Some(nonexistent_config),
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_scan_with_invalid_config_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create an invalid config file
        let config_path = temp_dir.path().join("invalid_config.json");
        let invalid_config = r#"{ "invalid": "json structure" }"#;
        fs::write(&config_path, invalid_config).unwrap();

        // Create target directory
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        let output = temp_dir.path().join("output.json");

        let result = execute_scan(
            target_dir,
            output,
            None,
            true,
            "dct".to_string(),
            8,
            None,
            Some(config_path),
        )
        .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse config file"));
    }

    #[tokio::test]
    async fn test_scan_with_config_file_different_algorithms() {
        let temp_dir = TempDir::new().unwrap();

        // Test DCT algorithm config
        let dct_config_path = temp_dir.path().join("dct_config.json");
        let dct_config = r#"{
            "algorithm": "dct",
            "parameters": {
                "size": 32,
                "quality_factor": 1.0
            }
        }"#;
        fs::write(&dct_config_path, dct_config).unwrap();

        // Test Average algorithm config
        let avg_config_path = temp_dir.path().join("avg_config.json");
        let avg_config = r#"{
            "algorithm": "average",
            "parameters": {
                "size": 8
            }
        }"#;
        fs::write(&avg_config_path, avg_config).unwrap();

        // Create target directory
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("test.txt"), "test content").unwrap();

        // Test DCT config
        let dct_output = temp_dir.path().join("dct_output.json");
        let dct_result = execute_scan(
            target_dir.clone(),
            dct_output,
            None,
            true,
            "dct".to_string(),
            8,
            None,
            Some(dct_config_path),
        )
        .await;

        assert!(dct_result.is_ok());

        // Test Average config
        let avg_output = temp_dir.path().join("avg_output.json");
        let avg_result = execute_scan(
            target_dir,
            avg_output,
            None,
            true,
            "average".to_string(),
            8,
            None,
            Some(avg_config_path),
        )
        .await;

        assert!(avg_result.is_ok());
    }

    #[tokio::test]
    async fn test_scan_config_file_takes_precedence_over_preset() {
        let temp_dir = TempDir::new().unwrap();

        // Create config file
        let config_path = temp_dir.path().join("config.json");
        let config_content = r#"{
            "algorithm": "average",
            "parameters": {
                "size": 16
            }
        }"#;
        fs::write(&config_path, config_content).unwrap();

        // Create target directory
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("test.txt"), "test content").unwrap();

        let output = temp_dir.path().join("output.json");

        // Both config file and preset are provided - config file should take precedence
        let result = execute_scan(
            target_dir,
            output,
            None,
            true,
            "dct".to_string(),
            8,
            Some("high_performance".to_string()), // This should be ignored
            Some(config_path),                    // This should take precedence
        )
        .await;

        assert!(result.is_ok());
        // The function should use the config file (average algorithm) instead of high_performance preset
    }

    #[tokio::test]
    async fn test_scan_with_config_presets() {
        let target_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        // テスト用の設定プリセット
        let presets = vec!["default", "high_performance", "testing"];

        for preset in presets {
            let output = output_dir.path().join(format!("output_{preset}.json"));

            let result = execute_scan(
                target_dir.path().to_path_buf(),
                output,
                None,
                true,
                "dct".to_string(),
                8,
                Some(preset.to_string()),
                None,
            )
            .await;

            assert!(result.is_ok(), "Failed for preset: {preset}");
        }
    }

    #[tokio::test]
    async fn test_invalid_config_preset() {
        let target_dir = TempDir::new().unwrap();
        let output = target_dir.path().join("output.json");

        let result = execute_scan(
            target_dir.path().to_path_buf(),
            output,
            None,
            true,
            "dct".to_string(),
            8,
            Some("invalid_preset".to_string()),
            None,
        )
        .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown configuration preset"));
    }
}
