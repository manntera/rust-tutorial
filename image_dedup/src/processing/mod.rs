// 並列処理システムのモジュール
// 機能別フォルダ構造によるアーキテクチャ
//
// この設計は将来の拡張性を考慮しています。
// 各機能モジュールは独立して拡張可能です。

// コアモジュール
pub mod types;        // データ構造定義

// 機能モジュール (抜象化レベル別)
pub mod config;       // 設定管理 (将来: default.rs, file.rs, env.rs)
pub mod reporting;    // 進捗報告・監視 (将来: console.rs, file.rs, metrics.rs)
pub mod persistence;  // データ永続化 (将来: json.rs, sqlite.rs, csv.rs)
pub mod processor;    // 並列処理制御 (将来: engine.rs, pipeline.rs, worker.rs)

// 公開API - 各機能から再エクスポート
pub use types::*;
pub use config::ProcessingConfig;
pub use reporting::ProgressReporter;
pub use persistence::HashPersistence;
pub use processor::ParallelProcessor;

