// 進捗報告と監視機能
//
// このモジュールは進捗報告機能を提供し、将来的には以下の拡張が想定される：
// - reporting/console.rs   - コンソール出力実装
// - reporting/file.rs      - ファイル出力実装
// - reporting/json.rs      - JSON形式ログ出力
// - reporting/metrics.rs   - Prometheusメトリクス送信
// - reporting/gui.rs       - GUI進捗表示
// - reporting/webhook.rs   - Webhook通知

pub mod traits;

#[cfg(test)]
pub mod test_mocks;

// 公開API
pub use traits::*;

// テストモック（テスト時のみ）
#[cfg(test)]
pub use test_mocks::*;