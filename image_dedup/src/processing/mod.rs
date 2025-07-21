// 並列処理システム - 機能別整理された構造
// 機能的な目的に基づいてコードを分離し、保守性と可読性を向上

// 基盤モジュール
pub mod types;                  // データ構造定義
pub mod traits;                 // トレイト定義
pub mod tests;                  // テストユーティリティ
pub mod api;                    // 高レベル公開API

// 機能別モジュール
pub mod image_processing;       // 画像処理機能
pub mod parallel_execution;     // 並列実行機能
pub mod progress_monitoring;    // 進捗監視機能
pub mod data_persistence;       // データ永続化機能
pub mod configuration;          // 設定管理機能

// 公開API - トレイト
pub use traits::{
    ProcessingConfig,
    ProgressReporter,
    HashPersistence,
    ParallelProcessor,
};

// 公開API - データ型
pub use types::*;

// 公開API - 具象実装
pub use configuration::DefaultProcessingConfig;
pub use progress_monitoring::{ConsoleProgressReporter, NoOpProgressReporter};
pub use data_persistence::MemoryHashPersistence;

// 公開API - コア機能
pub use parallel_execution::ProcessingEngine;
pub use image_processing::process_single_file;

// 内部API - 後方互換性のみ（直接使用非推奨）
// ProcessingPipelineはProcessingEngine内部で使用されるため、
// クレート外に公開はするが使用は推奨しない
pub use parallel_execution::ProcessingPipeline;

// 公開API - 高レベル便利関数
pub use api::{
    // レガシーAPI（後方互換性）
    process_directory_parallel,
    process_directory_with_config,
    process_files_parallel,
    // 新しいDI対応API
    process_directory_with_engine,
    process_files_with_engine,
    create_default_processing_engine,
    create_quiet_processing_engine,
};

// 公開API - テストユーティリティ
pub use tests::*;