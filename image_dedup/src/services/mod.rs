// サービス層 - 機能別のビジネスロジック
// 各サービスは特定の責任を持ち、疎結合で設計されている

pub mod config;
pub mod monitoring;
pub mod persistence;
pub mod processing;

// 公開API - 各サービスの主要機能を明示的にエクスポート
pub use config::DefaultProcessingConfig;
pub use monitoring::{ConsoleProgressReporter, NoOpProgressReporter};
pub use persistence::{
    JsonHashPersistence, MemoryHashPersistence, StreamingJsonHashPersistence,
    spawn_result_collector,
};
pub use processing::process_single_file;
