// 並列実行機能
// Producer-Consumer パターンによる並列処理とオーケストレーション

pub mod producer;
pub mod consumer;
pub mod pipeline;
pub mod engine;

// 公開API
pub use producer::spawn_producer;
pub use consumer::{spawn_single_consumer, spawn_consumers};
pub use pipeline::ProcessingPipeline;
pub use engine::ParallelProcessingEngine;