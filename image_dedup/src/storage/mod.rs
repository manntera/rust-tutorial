use anyhow::Result;
use async_trait::async_trait;

pub mod local;

/// ストレージ内のアイテムを表す構造体
#[derive(Debug, Clone)]
pub struct StorageItem {
    /// アイテムの識別子（ローカルならパス、S3ならオブジェクトキー）
    pub id: String,
    /// アイテム名（ファイル名）
    pub name: String,
    /// アイテムのサイズ（バイト）
    pub size: u64,
    /// アイテムがディレクトリかどうか
    pub is_directory: bool,
    /// 拡張子（あれば）
    pub extension: Option<String>,
}

/// ストレージバックエンドのトレイト
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// ストレージ内のアイテムをリストする
    async fn list_items(&self, prefix: &str) -> Result<Vec<StorageItem>>;

    /// アイテムのデータを読み込む
    async fn read_item(&self, id: &str) -> Result<Vec<u8>>;

    /// アイテムが存在するかチェック
    async fn exists(&self, id: &str) -> Result<bool>;

    /// アイテムを削除する
    async fn delete_item(&self, id: &str) -> Result<()>;

    /// 画像ファイルかどうかを判定
    fn is_image_file(&self, item: &StorageItem) -> bool {
        if item.is_directory {
            return false;
        }

        if let Some(ext) = &item.extension {
            let ext_lower = ext.to_lowercase();
            matches!(
                ext_lower.as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp"
            )
        } else {
            false
        }
    }
}
