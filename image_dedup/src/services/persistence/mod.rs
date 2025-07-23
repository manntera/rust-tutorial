// データ永続化機能
// ハッシュデータの保存、バッチ処理、結果収集

pub mod collector;
pub mod implementations;

// 公開API
pub use collector::spawn_result_collector;
pub use implementations::{
    JsonHashPersistence, MemoryHashPersistence, StreamingJsonHashPersistence,
};
