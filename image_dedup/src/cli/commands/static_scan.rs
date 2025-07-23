//! 静的ディスパッチ版スキャンコマンド
//!
//! コンパイル時に型が確定する高性能スキャン：
//! - 静的ディスパッチによるゼロコスト抽象化
//! - コンパイル時設定検証
//! - 型安全な依存関係注入

use crate::core::{
    DefaultConfig, HighPerformanceConfig, ProcessingConfig, ProcessingEngineFactory,
    ProcessingEngineVariant, StaticDIContainer, StaticDependencyProvider, StaticProcessingEngine,
    TestingConfig,
};
use anyhow::Result;
use std::path::PathBuf;

/// 静的スキャン設定
#[derive(Clone)]
pub struct StaticScanConfig {
    pub target_directory: PathBuf,
    pub output: PathBuf,
    pub threads: Option<usize>,
    pub force: bool,
}

/// 静的ディスパッチによる統一スキャン実行
pub async fn execute_static_scan<P: StaticDependencyProvider>(
    config: StaticScanConfig,
    _container: StaticDIContainer<P>,
) -> Result<()> {
    // 入力検証
    validate_scan_input(&config)?;

    // スレッド数決定
    let thread_count = config.threads.unwrap_or_else(num_cpus::get);

    // 静的DIコンテナから処理エンジンを作成（コンパイル時型確定）
    let container = StaticDIContainer::<P>::new();
    let engine = container.create_processing_engine(&config.output);

    // 実行情報表示
    print_execution_info::<P>(&config, thread_count, &engine);

    // 処理実行
    execute_processing::<P>(&engine, &config).await
}

/// デフォルト設定でスキャン実行
pub async fn execute_default_scan(config: StaticScanConfig) -> Result<()> {
    let container = StaticDIContainer::<DefaultConfig>::new();
    execute_static_scan(config, container).await
}

/// 高性能設定でスキャン実行
pub async fn execute_high_performance_scan(config: StaticScanConfig) -> Result<()> {
    let container = StaticDIContainer::<HighPerformanceConfig>::new();
    execute_static_scan(config, container).await
}

/// テスト設定でスキャン実行
pub async fn execute_testing_scan(config: StaticScanConfig) -> Result<()> {
    let container = StaticDIContainer::<TestingConfig>::new();
    execute_static_scan(config, container).await
}

/// プリセット名による動的スキャン実行
///
/// 実行時にプリセットを選択する場合に使用
/// 内部的には適切な静的ディスパッチ版を呼び出す
pub async fn execute_scan_by_preset(config: StaticScanConfig, preset: &str) -> Result<()> {
    match preset {
        "default" => execute_default_scan(config).await,
        "high_performance" => execute_high_performance_scan(config).await,
        "testing" => execute_testing_scan(config).await,
        _ => anyhow::bail!(
            "無効なプリセット: {}. 利用可能: default, high_performance, testing",
            preset
        ),
    }
}

/// アルゴリズム・パラメータ指定による動的スキャン実行
///
/// 後方互換性のため、従来のAPIも提供
pub async fn execute_parametric_scan(
    target_directory: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    force: bool,
    algorithm: String,
    hash_size: u32,
) -> Result<()> {
    let config = StaticScanConfig {
        target_directory,
        output,
        threads,
        force,
    };

    // パラメータに基づいて最適なプリセットを選択
    let preset = match (algorithm.as_str(), hash_size) {
        ("dct", 8) => "default",
        ("dct", 32) => "high_performance",
        ("average", 8) => "testing",
        _ => "default", // フォールバック
    };

    execute_scan_by_preset(config, preset).await
}

/// 入力検証
fn validate_scan_input(config: &StaticScanConfig) -> Result<()> {
    // ターゲットディレクトリ検証
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

    // 出力ファイル検証
    if config.output.exists() && !config.force {
        anyhow::bail!(
            "Output file already exists: {}. Use --force to overwrite.",
            config.output.display()
        );
    }

    // 出力ディレクトリ作成
    if let Some(parent) = config.output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

/// 実行情報表示
fn print_execution_info<P: StaticDependencyProvider>(
    config: &StaticScanConfig,
    thread_count: usize,
    engine: &StaticProcessingEngine<P>,
) {
    println!("🚀 静的ディスパッチ画像重複検出ツール");
    println!("📂 対象ディレクトリ: {}", config.target_directory.display());
    println!("📄 出力ファイル: {}", config.output.display());
    println!("🧵 使用スレッド数: {thread_count}");
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
}

/// 処理実行と結果表示
async fn execute_processing<P: StaticDependencyProvider>(
    engine: &StaticProcessingEngine<P>,
    config: &StaticScanConfig,
) -> Result<()> {
    let start_time = std::time::Instant::now();
    let target_str = config.target_directory.to_string_lossy();

    match engine.process_directory(&target_str).await {
        Ok(summary) => {
            let elapsed = start_time.elapsed();
            print_success_summary(&summary, elapsed, &config.output);
            Ok(())
        }
        Err(error) => {
            anyhow::bail!("処理エラー: {}", error);
        }
    }
}

/// 成功時のサマリー表示
fn print_success_summary(
    summary: &crate::core::ProcessingSummary,
    elapsed: std::time::Duration,
    output_path: &std::path::Path,
) {
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

    println!("📄 結果は {} に保存されました", output_path.display());
}

/// 設定ファイル対応スキャン
pub async fn execute_scan_from_static_config_file(
    config: StaticScanConfig,
    config_path: PathBuf,
) -> Result<()> {
    println!("📄 設定ファイル: {}", config_path.display());

    // 設定ファイルから適切なプリセットを判定
    // 実装の簡素化のため、ファイル名ベースで判定
    let preset = if config_path.to_string_lossy().contains("high_performance") {
        "high_performance"
    } else if config_path.to_string_lossy().contains("test") {
        "testing"
    } else {
        "default"
    };

    println!("🔧 検出されたプリセット: {preset}");
    execute_scan_by_preset(config, preset).await
}

/// 統一DI APIを使用した次世代スキャン実行
///
/// 最新の統一DIシステムを使用した高レベルAPI
/// 動的・静的ディスパッチを自動選択し、最適なパフォーマンスを提供
pub async fn execute_unified_scan(
    config: StaticScanConfig,
    prefer_performance: bool,
) -> Result<()> {
    // 入力検証
    validate_scan_input(&config)?;

    // 統一DIファクトリーで最適なエンジンを作成
    let engine =
        ProcessingEngineFactory::create_optimal("default", &config.output, prefer_performance)
            .map_err(|e| anyhow::anyhow!("エンジン作成エラー: {e}"))?;

    // エンジン情報表示
    print_unified_execution_info(&config, &engine);

    // 処理実行
    execute_unified_processing(&engine, &config).await
}

/// 統一DI APIによる高性能スキャン
pub async fn execute_high_performance_unified_scan(config: StaticScanConfig) -> Result<()> {
    validate_scan_input(&config)?;

    let engine = ProcessingEngineFactory::create_high_performance(&config.output)
        .map_err(|e| anyhow::anyhow!("高性能エンジン作成エラー: {e}"))?;

    print_unified_execution_info(&config, &engine);
    execute_unified_processing(&engine, &config).await
}

/// 統一DI APIによるテストスキャン
pub async fn execute_testing_unified_scan(config: StaticScanConfig) -> Result<()> {
    validate_scan_input(&config)?;

    let engine = ProcessingEngineFactory::create_testing(&config.output)
        .map_err(|e| anyhow::anyhow!("テストエンジン作成エラー: {e}"))?;

    print_unified_execution_info(&config, &engine);
    execute_unified_processing(&engine, &config).await
}

/// 柔軟性重視の統一スキャン
pub async fn execute_flexible_unified_scan(config: StaticScanConfig, preset: &str) -> Result<()> {
    validate_scan_input(&config)?;

    let engine = ProcessingEngineFactory::create_flexible(preset, &config.output)
        .map_err(|e| anyhow::anyhow!("柔軟性エンジン作成エラー: {e}"))?;

    print_unified_execution_info(&config, &engine);
    execute_unified_processing(&engine, &config).await
}

/// 統一エンジンの実行情報表示
fn print_unified_execution_info(config: &StaticScanConfig, engine: &ProcessingEngineVariant) {
    let characteristics = engine.performance_characteristics();

    println!("🚀 次世代統一DI画像重複検出ツール");
    println!("📂 対象ディレクトリ: {}", config.target_directory.display());
    println!("📄 出力ファイル: {}", config.output.display());
    println!("⚙️  エンジン情報:");
    println!("   - 種類: {}", engine.engine_type());
    println!("   - ディスパッチ: {}", characteristics.dispatch_type());
    println!(
        "   - パフォーマンス: {}",
        characteristics.performance_level()
    );
    println!(
        "   - 推定オーバーヘッド: {}レベル",
        characteristics.estimated_overhead()
    );

    if let Some(threads) = config.threads {
        println!("🧵 使用スレッド数: {threads}");
    }
}

/// 統一エンジンでの処理実行
async fn execute_unified_processing(
    engine: &ProcessingEngineVariant,
    config: &StaticScanConfig,
) -> Result<()> {
    let start_time = std::time::Instant::now();
    let target_str = config.target_directory.to_string_lossy();

    match engine.process_directory(&target_str).await {
        Ok(summary) => {
            let elapsed = start_time.elapsed();
            print_unified_success_summary(&summary, elapsed, &config.output, engine);
            Ok(())
        }
        Err(error) => {
            anyhow::bail!("統一DI処理エラー: {}", error);
        }
    }
}

/// 統一DI版成功サマリー表示
fn print_unified_success_summary(
    summary: &crate::core::ProcessingSummary,
    elapsed: std::time::Duration,
    output_path: &std::path::Path,
    engine: &ProcessingEngineVariant,
) {
    let characteristics = engine.performance_characteristics();

    println!("\n✅ 統一DI処理完了!");
    println!("📊 処理結果:");
    println!("   - エンジン: {}", engine.engine_type());
    println!("   - ディスパッチ: {}", characteristics.dispatch_type());
    println!("   - 対象ファイル数: {}", summary.total_files);
    println!("   - 成功処理数: {}", summary.processed_files);
    println!("   - エラー数: {}", summary.error_count);
    println!("   - 総処理時間: {:.2}秒", elapsed.as_secs_f64());
    println!(
        "   - 平均処理時間: {:.2}ms/ファイル",
        summary.average_time_per_file_ms
    );

    if summary.processed_files > 0 {
        let throughput = summary.processed_files as f64 / elapsed.as_secs_f64();
        println!("   - スループット: {throughput:.2}ファイル/秒");
    }

    if summary.error_count > 0 {
        println!(
            "⚠️  {}個のファイルでエラーが発生しました",
            summary.error_count
        );
    }

    println!("📄 結果は {} に保存されました", output_path.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_static_scan_nonexistent_directory() {
        let nonexistent_dir = PathBuf::from("nonexistent_directory");
        let output = PathBuf::from("output.json");
        let config = StaticScanConfig {
            target_directory: nonexistent_dir,
            output,
            threads: None,
            force: false,
        };

        let result = execute_default_scan(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_static_scan_file_instead_of_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        std::fs::write(&file_path, "test content").unwrap();

        let output = PathBuf::from("output.json");
        let config = StaticScanConfig {
            target_directory: file_path,
            output,
            threads: None,
            force: false,
        };

        let result = execute_default_scan(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[tokio::test]
    async fn test_static_scan_existing_output_without_force() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("existing_output.json");
        std::fs::write(&output, "existing content").unwrap();

        let target_dir = TempDir::new().unwrap();
        let config = StaticScanConfig {
            target_directory: target_dir.path().to_path_buf(),
            output,
            threads: None,
            force: false,
        };

        let result = execute_default_scan(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_static_scan_with_different_presets() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        for preset in ["default", "high_performance", "testing"] {
            let output = temp_dir.path().join(format!("output_{preset}.json"));
            let config = StaticScanConfig {
                target_directory: target_dir.path().to_path_buf(),
                output,
                threads: Some(1),
                force: true,
            };

            let result = execute_scan_by_preset(config, preset).await;
            assert!(result.is_ok(), "Failed with preset: {preset}");
        }
    }

    #[tokio::test]
    async fn test_static_scan_parametric_backward_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("parametric_output.json");

        let result = execute_parametric_scan(
            target_dir.path().to_path_buf(),
            output,
            Some(1),
            true,
            "dct".to_string(),
            8,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_static_scan_invalid_preset() {
        let temp_dir = TempDir::new().unwrap();
        let config = StaticScanConfig {
            target_directory: temp_dir.path().to_path_buf(),
            output: temp_dir.path().join("output.json"),
            threads: None,
            force: true,
        };

        let result = execute_scan_by_preset(config, "invalid_preset").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("無効なプリセット"));
    }

    #[test]
    fn test_static_scan_config_creation() {
        let config = StaticScanConfig {
            target_directory: PathBuf::from("/test"),
            output: PathBuf::from("output.json"),
            threads: Some(4),
            force: true,
        };

        assert_eq!(config.target_directory, PathBuf::from("/test"));
        assert_eq!(config.output, PathBuf::from("output.json"));
        assert_eq!(config.threads, Some(4));
        assert!(config.force);
    }

    #[tokio::test]
    async fn test_unified_scan_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = StaticScanConfig {
            target_directory: temp_dir.path().to_path_buf(),
            output: temp_dir.path().join("output.json"),
            threads: None,
            force: true,
        };

        // 性能重視（静的ディスパッチ）
        let result = execute_unified_scan(config.clone(), true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_high_performance_unified_scan() {
        let temp_dir = TempDir::new().unwrap();
        let config = StaticScanConfig {
            target_directory: temp_dir.path().to_path_buf(),
            output: temp_dir.path().join("hp_output.json"),
            threads: Some(2),
            force: true,
        };

        let result = execute_high_performance_unified_scan(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_testing_unified_scan() {
        let temp_dir = TempDir::new().unwrap();
        let config = StaticScanConfig {
            target_directory: temp_dir.path().to_path_buf(),
            output: temp_dir.path().join("test_output.json"),
            threads: Some(1),
            force: true,
        };

        let result = execute_testing_unified_scan(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_flexible_unified_scan() {
        let temp_dir = TempDir::new().unwrap();
        let config = StaticScanConfig {
            target_directory: temp_dir.path().to_path_buf(),
            output: temp_dir.path().join("flexible_output.json"),
            threads: Some(1),
            force: true,
        };

        for preset in ["default", "high_performance", "testing"] {
            let result = execute_flexible_unified_scan(config.clone(), preset).await;
            assert!(result.is_ok(), "Failed with flexible preset: {preset}");
        }
    }
}
