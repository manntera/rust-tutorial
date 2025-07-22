// エンジン層 - 並列処理とオーケストレーション
// サービス層を組み合わせて高レベルな処理を提供

pub mod producer;
pub mod consumer;
pub mod processing_engine;
pub mod api;
mod pipeline; // ProcessingEngine内部でのみ使用

// 公開API - 主要エンジンクラス
pub use processing_engine::ProcessingEngine;
pub use api::*;