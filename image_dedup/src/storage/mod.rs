use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;

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
#[automock]
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

// StorageBackend for Box<dyn StorageBackend>
#[async_trait]
impl StorageBackend for Box<dyn StorageBackend> {
    async fn list_items(&self, prefix: &str) -> Result<Vec<StorageItem>> {
        self.as_ref().list_items(prefix).await
    }

    async fn read_item(&self, id: &str) -> Result<Vec<u8>> {
        self.as_ref().read_item(id).await
    }

    async fn exists(&self, id: &str) -> Result<bool> {
        self.as_ref().exists(id).await
    }

    async fn delete_item(&self, id: &str) -> Result<()> {
        self.as_ref().delete_item(id).await
    }

    fn is_image_file(&self, item: &StorageItem) -> bool {
        self.as_ref().is_image_file(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_item_creation() {
        let item = StorageItem {
            id: "path/to/file.jpg".to_string(),
            name: "file.jpg".to_string(),
            size: 1024,
            is_directory: false,
            extension: Some("jpg".to_string()),
        };

        assert_eq!(item.id, "path/to/file.jpg");
        assert_eq!(item.name, "file.jpg");
        assert_eq!(item.size, 1024);
        assert!(!item.is_directory);
        assert_eq!(item.extension, Some("jpg".to_string()));
    }

    #[test]
    fn test_is_image_file_valid_extensions() {
        let backend = crate::storage::local::LocalStorageBackend::new();

        // Test valid image extensions
        let valid_extensions = vec!["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"];

        for ext in valid_extensions {
            let item = StorageItem {
                id: format!("file.{ext}"),
                name: format!("file.{ext}"),
                size: 1000,
                is_directory: false,
                extension: Some(ext.to_string()),
            };
            assert!(
                backend.is_image_file(&item),
                "Extension {ext} should be recognized as image"
            );
        }
    }

    #[test]
    fn test_is_image_file_case_insensitive() {
        let backend = crate::storage::local::LocalStorageBackend::new();

        let extensions = vec!["JPG", "JPEG", "PNG", "GIF", "Jpg", "pNg"];

        for ext in extensions {
            let item = StorageItem {
                id: format!("file.{ext}"),
                name: format!("file.{ext}"),
                size: 1000,
                is_directory: false,
                extension: Some(ext.to_string()),
            };
            assert!(
                backend.is_image_file(&item),
                "Extension {ext} should be recognized as image"
            );
        }
    }

    #[test]
    fn test_is_image_file_invalid_extensions() {
        let backend = crate::storage::local::LocalStorageBackend::new();

        let invalid_extensions = vec!["txt", "pdf", "doc", "mp4", "mp3", "exe"];

        for ext in invalid_extensions {
            let item = StorageItem {
                id: format!("file.{ext}"),
                name: format!("file.{ext}"),
                size: 1000,
                is_directory: false,
                extension: Some(ext.to_string()),
            };
            assert!(
                !backend.is_image_file(&item),
                "Extension {ext} should not be recognized as image"
            );
        }
    }

    #[test]
    fn test_is_image_file_no_extension() {
        let backend = crate::storage::local::LocalStorageBackend::new();

        let item = StorageItem {
            id: "file_without_extension".to_string(),
            name: "file_without_extension".to_string(),
            size: 1000,
            is_directory: false,
            extension: None,
        };

        assert!(!backend.is_image_file(&item));
    }

    #[test]
    fn test_is_image_file_directory() {
        let backend = crate::storage::local::LocalStorageBackend::new();

        let item = StorageItem {
            id: "directory.jpg".to_string(),
            name: "directory.jpg".to_string(),
            size: 0,
            is_directory: true,
            extension: Some("jpg".to_string()),
        };

        // Directories should not be considered image files even with image extensions
        assert!(!backend.is_image_file(&item));
    }

    #[test]
    fn test_storage_item_debug() {
        let item = StorageItem {
            id: "test.jpg".to_string(),
            name: "test.jpg".to_string(),
            size: 5000,
            is_directory: false,
            extension: Some("jpg".to_string()),
        };

        let debug_str = format!("{item:?}");
        assert!(debug_str.contains("test.jpg"));
        assert!(debug_str.contains("5000"));
    }

    #[test]
    fn test_storage_item_clone() {
        let item = StorageItem {
            id: "original.png".to_string(),
            name: "original.png".to_string(),
            size: 2048,
            is_directory: false,
            extension: Some("png".to_string()),
        };

        let cloned = item.clone();
        assert_eq!(item.id, cloned.id);
        assert_eq!(item.name, cloned.name);
        assert_eq!(item.size, cloned.size);
        assert_eq!(item.is_directory, cloned.is_directory);
        assert_eq!(item.extension, cloned.extension);
    }
}
