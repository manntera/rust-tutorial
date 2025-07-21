// 並列処理の設定管理
//
// このモジュールは設定管理機能を提供し、将来的には以下の拡張が想定される：
// - config/default.rs     - デフォルト設定実装
// - config/file.rs        - ファイルベース設定
// - config/env.rs         - 環境変数ベース設定
// - config/runtime.rs     - 実行時動的設定

pub mod traits;

#[cfg(test)]
pub mod test_mocks;

// 公開API
pub use traits::*;

// テストモック（テスト時のみ）
#[cfg(test)]
pub use test_mocks::*;