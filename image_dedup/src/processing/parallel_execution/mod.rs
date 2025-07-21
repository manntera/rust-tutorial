// 並列実行機能
// Producer-Consumer パターンによる並列処理とオーケストレーション

pub mod producer;
pub mod consumer;
pub mod engine;
pub mod pipeline; // ProcessingEngine内部で使用するため公開が必要

// 公開API - 主要エンジン（推奨）
pub use engine::ProcessingEngine;

// 内部実装API - クレート内部でのみ使用（非推奨、後方互換性のみ）
pub use producer::spawn_producer;
pub use consumer::{spawn_single_consumer, spawn_consumers};
pub use pipeline::ProcessingPipeline; // 内部実装詳細（公開は必要だが使用は非推奨）