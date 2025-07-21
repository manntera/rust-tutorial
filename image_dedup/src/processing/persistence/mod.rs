// データ永続化機能
//
// このモジュールはデータ永続化機能を提供し、将来的には以下の拡張が想定される：
// - persistence/json.rs    - JSON形式での永続化
// - persistence/sqlite.rs  - SQLiteデータベース永続化
// - persistence/csv.rs     - CSV形式での永続化
// - persistence/redis.rs   - Redisキャッシュ永続化
// - persistence/postgres.rs - PostgreSQL永続化
// - persistence/memory.rs  - インメモリ永続化（テスト用）

pub mod traits;

#[cfg(test)]
pub mod test_mocks;

// 公開API
pub use traits::*;

// テストモック（テスト時のみ）
#[cfg(test)]
pub use test_mocks::*;