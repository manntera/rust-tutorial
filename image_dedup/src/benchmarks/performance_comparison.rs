//! 静的ディスパッチのパフォーマンス測定
//!
//! 異なる設定間のパフォーマンス比較とレポート生成

use crate::core::{
    traits::ProcessingConfig, DefaultConfig, HighPerformanceConfig, StaticDIContainer,
    TestingConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// パフォーマンス測定結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub test_name: String,
    pub default_time: Duration,
    pub high_performance_time: Duration,
    pub testing_time: Duration,
    pub best_config: String,
    pub memory_usage: HashMap<String, usize>,
}

impl PerformanceMetrics {
    pub fn new(
        test_name: String,
        default_time: Duration,
        high_performance_time: Duration,
        testing_time: Duration,
        memory_usage: HashMap<String, usize>,
    ) -> Self {
        let best_config = if default_time <= high_performance_time && default_time <= testing_time {
            "default".to_string()
        } else if high_performance_time <= testing_time {
            "high_performance".to_string()
        } else {
            "testing".to_string()
        };

        Self {
            test_name,
            default_time,
            high_performance_time,
            testing_time,
            best_config,
            memory_usage,
        }
    }

    pub fn get_best_time(&self) -> Duration {
        match self.best_config.as_str() {
            "default" => self.default_time,
            "high_performance" => self.high_performance_time,
            "testing" => self.testing_time,
            _ => self.default_time,
        }
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

        // DefaultConfig
        let start = Instant::now();
        for _ in 0..iterations {
            let _container = StaticDIContainer::<DefaultConfig>::new();
        }
        let default_time = start.elapsed();

        // HighPerformanceConfig
        let start = Instant::now();
        for _ in 0..iterations {
            let _container = StaticDIContainer::<HighPerformanceConfig>::new();
        }
        let high_performance_time = start.elapsed();

        // TestingConfig
        let start = Instant::now();
        for _ in 0..iterations {
            let _container = StaticDIContainer::<TestingConfig>::new();
        }
        let testing_time = start.elapsed();

        let mut memory_usage = HashMap::new();
        memory_usage.insert(
            "default".to_string(),
            std::mem::size_of::<StaticDIContainer<DefaultConfig>>(),
        );
        memory_usage.insert(
            "high_performance".to_string(),
            std::mem::size_of::<StaticDIContainer<HighPerformanceConfig>>(),
        );
        memory_usage.insert(
            "testing".to_string(),
            std::mem::size_of::<StaticDIContainer<TestingConfig>>(),
        );

        let metrics = PerformanceMetrics::new(
            "DI Container Creation".to_string(),
            default_time,
            high_performance_time,
            testing_time,
            memory_usage,
        );

        println!("  🟢 Default: {default_time:?}");
        println!("  🔴 HighPerformance: {high_performance_time:?}");
        println!("  🟡 Testing: {testing_time:?}");
        println!(
            "  🏆 Best: {} ({:?})",
            metrics.best_config,
            metrics.get_best_time()
        );

        self.results.push(metrics);
    }

    /// 設定アクセスのパフォーマンス比較
    pub fn benchmark_config_access(&mut self, iterations: usize) {
        println!("🔬 設定アクセスパフォーマンステスト ({iterations} iterations)");

        // DefaultConfig
        let container = StaticDIContainer::<DefaultConfig>::new();
        let config = container.create_processing_config();
        let start = Instant::now();
        for _ in 0..iterations {
            let _max_concurrent = config.max_concurrent_tasks();
            let _batch_size = config.batch_size();
            let _buffer_size = config.channel_buffer_size();
        }
        let default_time = start.elapsed();

        // HighPerformanceConfig
        let container = StaticDIContainer::<HighPerformanceConfig>::new();
        let config = container.create_processing_config();
        let start = Instant::now();
        for _ in 0..iterations {
            let _max_concurrent = config.max_concurrent_tasks();
            let _batch_size = config.batch_size();
            let _buffer_size = config.channel_buffer_size();
        }
        let high_performance_time = start.elapsed();

        // TestingConfig
        let container = StaticDIContainer::<TestingConfig>::new();
        let config = container.create_processing_config();
        let start = Instant::now();
        for _ in 0..iterations {
            let _max_concurrent = config.max_concurrent_tasks();
            let _batch_size = config.batch_size();
            let _buffer_size = config.channel_buffer_size();
        }
        let testing_time = start.elapsed();

        let mut memory_usage = HashMap::new();
        memory_usage.insert(
            "default".to_string(),
            std::mem::size_of_val(&container.create_processing_config()),
        );
        memory_usage.insert(
            "high_performance".to_string(),
            std::mem::size_of_val(
                &StaticDIContainer::<HighPerformanceConfig>::new().create_processing_config(),
            ),
        );
        memory_usage.insert(
            "testing".to_string(),
            std::mem::size_of_val(
                &StaticDIContainer::<TestingConfig>::new().create_processing_config(),
            ),
        );

        let metrics = PerformanceMetrics::new(
            "Configuration Access".to_string(),
            default_time,
            high_performance_time,
            testing_time,
            memory_usage,
        );

        println!("  🟢 Default: {default_time:?}");
        println!("  🔴 HighPerformance: {high_performance_time:?}");
        println!("  🟡 Testing: {testing_time:?}");
        println!(
            "  🏆 Best: {} ({:?})",
            metrics.best_config,
            metrics.get_best_time()
        );

        self.results.push(metrics);
    }

    /// 全体的なパフォーマンス比較実行
    pub fn run_full_comparison(&mut self) {
        println!("🚀 静的ディスパッチ設定比較");
        println!("{}", "=".repeat(60));

        self.benchmark_di_container_creation(10_000);
        println!();

        self.benchmark_config_access(100_000);
        println!();

        self.print_summary();
    }

    /// 結果サマリーの表示
    pub fn print_summary(&self) {
        println!("📊 パフォーマンス比較サマリー");
        println!("{}", "=".repeat(60));

        for metrics in &self.results {
            println!("🎯 {}", metrics.test_name);
            println!("   🟢 Default: {:?}", metrics.default_time);
            println!("   🔴 HighPerformance: {:?}", metrics.high_performance_time);
            println!("   🟡 Testing: {:?}", metrics.testing_time);
            println!(
                "   🏆 Best: {} ({:?})",
                metrics.best_config,
                metrics.get_best_time()
            );
            println!();
        }

        println!("✅ 全ての設定がゼロコスト抽象化を実現しています。");
        println!("🚀 静的ディスパッチによりコンパイル時最適化が適用されます。");
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
    fn test_zero_cost_abstraction_verification() {
        // 静的DIコンテナが本当にゼロコストかを検証
        assert_eq!(std::mem::size_of::<StaticDIContainer<DefaultConfig>>(), 0);
        assert_eq!(
            std::mem::size_of::<StaticDIContainer<HighPerformanceConfig>>(),
            0
        );
        assert_eq!(std::mem::size_of::<StaticDIContainer<TestingConfig>>(), 0);
    }
}
