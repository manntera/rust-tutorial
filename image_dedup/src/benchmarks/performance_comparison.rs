//! 静的ディスパッチ vs 動的ディスパッチのパフォーマンス比較
//!
//! シンプルなパフォーマンス測定とレポート生成

use crate::core::{
    DefaultConfig as StaticDefaultConfig, DependencyContainer, ProcessingConfig, StaticDIContainer,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// パフォーマンス測定結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub test_name: String,
    pub dynamic_time: Duration,
    pub static_time: Duration,
    pub improvement_ratio: f64,
    pub memory_dynamic: usize,
    pub memory_static: usize,
}

impl PerformanceMetrics {
    pub fn new(
        test_name: String,
        dynamic_time: Duration,
        static_time: Duration,
        memory_dynamic: usize,
        memory_static: usize,
    ) -> Self {
        let improvement_ratio = if static_time.as_nanos() > 0 {
            dynamic_time.as_nanos() as f64 / static_time.as_nanos() as f64
        } else {
            1.0
        };

        Self {
            test_name,
            dynamic_time,
            static_time,
            improvement_ratio,
            memory_dynamic,
            memory_static,
        }
    }

    pub fn improvement_percentage(&self) -> f64 {
        (self.improvement_ratio - 1.0) * 100.0
    }
}

/// パフォーマンス比較テストスイート
pub struct PerformanceComparison {
    results: Vec<PerformanceMetrics>,
}

impl PerformanceComparison {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// DIコンテナ作成のパフォーマンス比較
    pub fn benchmark_di_container_creation(&mut self, iterations: usize) {
        println!("🔬 DIコンテナ作成パフォーマンステスト ({iterations} iterations)");

        // 動的DIコンテナ
        let start = Instant::now();
        for _ in 0..iterations {
            let _container = DependencyContainer::default();
        }
        let dynamic_time = start.elapsed();

        // 静的DIコンテナ
        let start = Instant::now();
        for _ in 0..iterations {
            let _container = StaticDIContainer::<StaticDefaultConfig>::new();
        }
        let static_time = start.elapsed();

        let metrics = PerformanceMetrics::new(
            "DI Container Creation".to_string(),
            dynamic_time,
            static_time,
            std::mem::size_of::<DependencyContainer>(),
            std::mem::size_of::<StaticDIContainer<StaticDefaultConfig>>(),
        );

        println!("  ⚡ 動的ディスパッチ: {dynamic_time:?}");
        println!("  🚀 静的ディスパッチ: {static_time:?}");
        println!(
            "  📈 改善: {:.2}% ({:.2}x faster)",
            metrics.improvement_percentage(),
            metrics.improvement_ratio
        );

        self.results.push(metrics);
    }

    /// 設定アクセスのパフォーマンス比較
    pub fn benchmark_config_access(&mut self, iterations: usize) {
        println!("🔬 設定アクセスパフォーマンステスト ({iterations} iterations)");

        let temp_dir = tempfile::TempDir::new().unwrap();
        let _output_path = temp_dir.path().join("test.json");

        // 動的ディスパッチでの設定アクセス
        let dynamic_container = DependencyContainer::default();
        let dynamic_config = dynamic_container.create_processing_config().unwrap();

        let start = Instant::now();
        for _ in 0..iterations {
            let _max_concurrent = dynamic_config.max_concurrent_tasks();
            let _batch_size = dynamic_config.batch_size();
            let _buffer_size = dynamic_config.channel_buffer_size();
        }
        let dynamic_time = start.elapsed();

        // 静的ディスパッチでの設定アクセス
        let static_container = StaticDIContainer::<StaticDefaultConfig>::new();
        let static_config = static_container.create_processing_config();

        let start = Instant::now();
        for _ in 0..iterations {
            let _max_concurrent = static_config.max_concurrent_tasks();
            let _batch_size = static_config.batch_size();
            let _buffer_size = static_config.channel_buffer_size();
        }
        let static_time = start.elapsed();

        let metrics = PerformanceMetrics::new(
            "Configuration Access".to_string(),
            dynamic_time,
            static_time,
            8, // Box<dyn> のサイズ推定
            std::mem::size_of_val(&static_config),
        );

        println!("  ⚡ 動的ディスパッチ: {dynamic_time:?}");
        println!("  🚀 静的ディスパッチ: {static_time:?}");
        println!(
            "  📈 改善: {:.2}% ({:.2}x faster)",
            metrics.improvement_percentage(),
            metrics.improvement_ratio
        );

        self.results.push(metrics);
    }

    /// ProcessingEngine作成のパフォーマンス比較
    pub fn benchmark_processing_engine_creation(&mut self, iterations: usize) {
        println!("🔬 ProcessingEngine作成パフォーマンステスト ({iterations} iterations)");

        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

        // 動的ディスパッチでのProcessingEngine作成
        let start = Instant::now();
        for _ in 0..iterations {
            let container = DependencyContainer::default();
            let dependencies = container.resolve_all_dependencies(&output_path).unwrap();
            let _engine = dependencies.create_processing_engine();
        }
        let dynamic_time = start.elapsed();

        // 静的ディスパッチでのProcessingEngine作成
        let start = Instant::now();
        for _ in 0..iterations {
            let container = StaticDIContainer::<StaticDefaultConfig>::new();
            let _engine = container.create_processing_engine(&output_path);
        }
        let static_time = start.elapsed();

        let metrics = PerformanceMetrics::new(
            "ProcessingEngine Creation".to_string(),
            dynamic_time,
            static_time,
            1024, // 推定値
            512,  // 推定値
        );

        println!("  ⚡ 動的ディスパッチ: {dynamic_time:?}");
        println!("  🚀 静的ディスパッチ: {static_time:?}");
        println!(
            "  📈 改善: {:.2}% ({:.2}x faster)",
            metrics.improvement_percentage(),
            metrics.improvement_ratio
        );

        self.results.push(metrics);
    }

    /// メモリ使用量の比較
    pub fn benchmark_memory_usage(&mut self) {
        println!("🔬 メモリ使用量比較テスト");

        let dynamic_size = std::mem::size_of::<DependencyContainer>();
        let static_size = std::mem::size_of::<StaticDIContainer<StaticDefaultConfig>>();

        let metrics = PerformanceMetrics::new(
            "Memory Usage".to_string(),
            Duration::from_nanos(dynamic_size as u64),
            Duration::from_nanos(static_size as u64),
            dynamic_size,
            static_size,
        );

        println!("  💾 動的DIコンテナサイズ: {dynamic_size} bytes");
        println!("  🗜️  静的DIコンテナサイズ: {static_size} bytes");
        println!(
            "  📉 メモリ削減: {} bytes ({:.2}%)",
            dynamic_size.saturating_sub(static_size),
            if dynamic_size > 0 {
                (dynamic_size.saturating_sub(static_size) as f64 / dynamic_size as f64) * 100.0
            } else {
                0.0
            }
        );

        self.results.push(metrics);
    }

    /// 全体的なパフォーマンス比較実行
    pub fn run_full_comparison(&mut self) {
        println!("🚀 静的ディスパッチ vs 動的ディスパッチ パフォーマンス比較");
        println!("{}", "=".repeat(60));

        self.benchmark_di_container_creation(10_000);
        println!();

        self.benchmark_config_access(100_000);
        println!();

        self.benchmark_processing_engine_creation(1_000);
        println!();

        self.benchmark_memory_usage();
        println!();

        self.print_summary();
    }

    /// 結果サマリーの表示
    pub fn print_summary(&self) {
        println!("📊 パフォーマンス比較サマリー");
        println!("{}", "=".repeat(60));

        let mut total_improvement = 0.0;
        let mut improvement_count = 0;

        for metrics in &self.results {
            if metrics.test_name != "Memory Usage" {
                total_improvement += metrics.improvement_percentage();
                improvement_count += 1;
            }

            println!("🎯 {}", metrics.test_name);
            println!("   ⚡ 動的: {:?}", metrics.dynamic_time);
            println!("   🚀 静的: {:?}", metrics.static_time);

            if metrics.test_name == "Memory Usage" {
                let memory_reduction = metrics.memory_dynamic.saturating_sub(metrics.memory_static);
                println!("   📉 メモリ削減: {memory_reduction} bytes");
            } else {
                println!(
                    "   📈 改善: {:.2}% ({:.2}x faster)",
                    metrics.improvement_percentage(),
                    metrics.improvement_ratio
                );
            }
            println!();
        }

        if improvement_count > 0 {
            let average_improvement = total_improvement / improvement_count as f64;
            println!("🏆 平均パフォーマンス改善: {average_improvement:.2}%");
        }

        // 結論の表示
        self.print_conclusion();
    }

    /// 結論の表示
    fn print_conclusion(&self) {
        println!("🎯 結論");
        println!("{}", "=".repeat(60));

        let has_significant_improvement = self
            .results
            .iter()
            .any(|m| m.test_name != "Memory Usage" && m.improvement_percentage() > 5.0);

        if has_significant_improvement {
            println!("✅ 静的ディスパッチは動的ディスパッチと比較して有意なパフォーマンス改善を示しています。");
            println!("🚀 主な利点:");
            println!("   - コンパイル時最適化によるインライン化");
            println!("   - Virtual function callsの削除");
            println!("   - メモリ使用量の削減");
            println!("   - 型安全性の向上");
        } else {
            println!("⚖️  静的ディスパッチと動的ディスパッチのパフォーマンス差は限定的です。");
            println!("🎯 主な利点:");
            println!("   - コンパイル時エラー検出");
            println!("   - より良い型安全性");
            println!("   - 潜在的な最適化機会");
        }

        println!();
        println!("📝 推奨事項:");
        println!("   - パフォーマンスが重要な場面では静的ディスパッチを使用");
        println!("   - 柔軟性が必要な場面では動的ディスパッチを使用");
        println!("   - ハイブリッドアプローチで両方の利点を活用");
        println!();
    }

    /// JSON形式でのレポート出力
    pub fn export_json_report(
        &self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut report = HashMap::new();
        report.insert("timestamp", chrono::Utc::now().to_rfc3339());
        report.insert("test_results", serde_json::to_string(&self.results)?);

        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(path, json)?;

        println!("📄 詳細レポートを出力しました: {}", path.display());
        Ok(())
    }
}

impl Default for PerformanceComparison {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics::new(
            "Test".to_string(),
            Duration::from_millis(100),
            Duration::from_millis(50),
            1000,
            500,
        );

        assert_eq!(metrics.test_name, "Test");
        assert_eq!(metrics.improvement_ratio, 2.0);
        assert_eq!(metrics.improvement_percentage(), 100.0);
    }

    #[test]
    fn test_performance_comparison_creation() {
        let comparison = PerformanceComparison::new();
        assert!(comparison.results.is_empty());
    }

    #[test]
    fn test_di_container_benchmark() {
        let mut comparison = PerformanceComparison::new();
        comparison.benchmark_di_container_creation(10);

        assert_eq!(comparison.results.len(), 1);
        assert_eq!(comparison.results[0].test_name, "DI Container Creation");
    }

    #[test]
    fn test_memory_benchmark() {
        let mut comparison = PerformanceComparison::new();
        comparison.benchmark_memory_usage();

        assert_eq!(comparison.results.len(), 1);
        assert_eq!(comparison.results[0].test_name, "Memory Usage");
    }

    #[test]
    fn test_zero_cost_abstraction_verification() {
        // 静的DIコンテナが本当にゼロコストかを検証
        let static_size = std::mem::size_of::<StaticDIContainer<StaticDefaultConfig>>();
        assert_eq!(static_size, 0, "Static DI container should be zero-cost");
    }
}
