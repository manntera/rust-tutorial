// エンジン層 - 並列処理とオーケストレーション
// サービス層を組み合わせて高レベルな処理を提供

pub mod api;
pub mod consumer;
mod pipeline;
pub mod processing_engine;
pub mod producer; // ProcessingEngine内部でのみ使用

// 公開API - 主要エンジンクラス
pub use api::{
    create_default_processing_engine, create_quiet_processing_engine,
    process_directory_with_engine, process_files_with_engine,
};
pub use processing_engine::ProcessingEngine;
