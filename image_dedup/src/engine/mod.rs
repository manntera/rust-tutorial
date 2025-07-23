// エンジン層 - 並列処理とオーケストレーション
// サービス層を組み合わせて高レベルな処理を提供

pub mod api;
pub mod consumer;
mod pipeline;
pub mod processing_engine;
pub mod producer; // ProcessingEngine内部でのみ使用

// 公開API - 主要エンジンクラス
pub use api::*;
pub use processing_engine::ProcessingEngine;
