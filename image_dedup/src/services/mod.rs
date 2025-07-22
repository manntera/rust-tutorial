// サービス層 - 機能別のビジネスロジック
// 各サービスは特定の責任を持ち、疎結合で設計されている

pub mod persistence;
pub mod monitoring; 
pub mod config;
pub mod processing;

// 公開API - 各サービスの主要機能を明示的にエクスポート
pub use persistence::{
    spawn_result_collector, MemoryHashPersistence, JsonHashPersistence, StreamingJsonHashPersistence
};
pub use monitoring::{ConsoleProgressReporter, NoOpProgressReporter};
pub use config::DefaultProcessingConfig;
pub use processing::process_single_file;