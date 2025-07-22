// 進捗監視機能
// 処理進捗の報告、エラー通知、完了通知

pub mod implementations;

// 公開API
pub use implementations::{ConsoleProgressReporter, NoOpProgressReporter};