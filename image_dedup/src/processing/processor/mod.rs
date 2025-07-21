// 並列処理制御とオーケストレーション
//
// このモジュールは並列処理制御機能を提供し、将来的には以下の拡張が想定される：
// - processor/engine.rs     - メイン処理エンジン実装
// - processor/pipeline.rs   - Producer-Consumerパイプライン実装
// - processor/worker.rs     - ワーカープロセス管理
// - processor/scheduler.rs  - 動的スケジューリング
// - processor/distributed.rs - 分散処理制御
// - processor/gpu.rs        - GPU並列処理制御

pub mod traits;

#[cfg(test)]
pub mod test_mocks;

// 公開API
pub use traits::*;

// テストモック（テスト時のみ）
#[cfg(test)]
pub use test_mocks::*;