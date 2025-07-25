// 設定管理機能
// 並列処理設定、バッファサイズ、バッチサイズなどの設定管理

pub mod implementations;

// 公開API
pub use implementations::DefaultProcessingConfig;
