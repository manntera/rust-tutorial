// データ永続化のトレイト定義

use crate::processing::types::ProcessingMetadata;

/// ハッシュ値の永続化を抽象化するトレイト
#[async_trait::async_trait]
pub trait HashPersistence: Send + Sync {
    /// 単一ハッシュ値の保存
    async fn store_hash(&self, file_path: &str, hash: &str, metadata: &ProcessingMetadata) -> anyhow::Result<()>;
    
    /// バッチでの保存（効率化）
    async fn store_batch(&self, results: &[(String, String, ProcessingMetadata)]) -> anyhow::Result<()>;
    
    /// 永続化処理の完了
    async fn finalize(&self) -> anyhow::Result<()>;
}