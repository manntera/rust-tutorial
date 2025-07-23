//! パフォーマンス測定とベンチマークモジュール
//! 
//! 静的ディスパッチと動的ディスパッチの性能比較を提供

pub mod performance_comparison;

pub use performance_comparison::{PerformanceComparison, PerformanceMetrics};