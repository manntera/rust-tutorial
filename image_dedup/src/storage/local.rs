use super::{StorageBackend, StorageItem};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;

/// ローカルファイルシステム用のストレージバックエンド
#[derive(Clone, Debug)]
pub struct LocalStorageBackend;

impl Default for LocalStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalStorageBackend {
    pub fn new() -> Self {
        Self
    }

    fn path_to_storage_item(path: &Path) -> Result<StorageItem> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let extension = if metadata.is_file() {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_string())
        } else {
            None
        };

        Ok(StorageItem {
            id: path.to_string_lossy().to_string(),
            name,
            size: metadata.len(),
            is_directory: metadata.is_dir(),
            extension,
        })
    }

    /// 指定されたディレクトリ以下の全てのアイテムを再帰的に取得
    pub async fn list_items_recursive(&self, prefix: &str) -> Result<Vec<StorageItem>> {
        let path = Path::new(prefix);
        let mut all_items = Vec::new();

        self.list_items_recursive_internal(path, &mut all_items)
            .await?;

        Ok(all_items)
    }

    async fn list_items_recursive_internal(
        &self,
        path: &Path,
        items: &mut Vec<StorageItem>,
    ) -> Result<()> {
        let mut entries = tokio::fs::read_dir(path)
            .await
            .with_context(|| format!("Failed to read directory: {}", path.display()))?;

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            if let Ok(item) = Self::path_to_storage_item(&entry_path) {
                items.push(item.clone());

                // ディレクトリの場合は再帰的に処理
                if item.is_directory {
                    Box::pin(self.list_items_recursive_internal(&entry_path, items)).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn list_items(&self, prefix: &str) -> Result<Vec<StorageItem>> {
        self.list_items_recursive(prefix).await
    }

    async fn read_item(&self, id: &str) -> Result<Vec<u8>> {
        let path = Path::new(id);
        let data = tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read file: {id}"))?;
        Ok(data)
    }

    async fn exists(&self, id: &str) -> Result<bool> {
        let path = Path::new(id);
        Ok(path.exists())
    }

    async fn delete_item(&self, id: &str) -> Result<()> {
        let path = Path::new(id);
        if path.is_file() {
            tokio::fs::remove_file(path)
                .await
                .with_context(|| format!("Failed to delete file: {id}"))?;
        } else {
            anyhow::bail!("Cannot delete directory using delete_item");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_items() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // テスト用のファイルを作成
        std::fs::write(temp_path.join("image1.jpg"), b"dummy").unwrap();
        std::fs::write(temp_path.join("image2.png"), b"dummy").unwrap();
        std::fs::write(temp_path.join("document.txt"), b"dummy").unwrap();

        let backend = LocalStorageBackend::new();
        let items = backend
            .list_items(temp_path.to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(items.len(), 3); // 全てのファイルがリストされる
        assert!(items.iter().any(|i| i.name == "image1.jpg"));
        assert!(items.iter().any(|i| i.name == "image2.png"));
        assert!(items.iter().any(|i| i.name == "document.txt"));

        // 画像ファイルのみフィルタリング
        let image_files: Vec<_> = items
            .iter()
            .filter(|item| backend.is_image_file(item))
            .collect();
        assert_eq!(image_files.len(), 2);
    }

    #[tokio::test]
    async fn test_list_items_recursive() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // サブディレクトリを作成
        let sub_dir = temp_path.join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();

        // ファイルを作成
        std::fs::write(temp_path.join("root.jpg"), b"dummy").unwrap();
        std::fs::write(sub_dir.join("nested.png"), b"dummy").unwrap();

        let backend = LocalStorageBackend::new();
        let items = backend
            .list_items_recursive(temp_path.to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(items.len(), 3); // root.jpg + subdir + nested.png
        assert!(items
            .iter()
            .any(|i| i.name == "root.jpg" && !i.is_directory));
        assert!(items.iter().any(|i| i.name == "subdir" && i.is_directory));
        assert!(items
            .iter()
            .any(|i| i.name == "nested.png" && !i.is_directory));
    }

    #[tokio::test]
    async fn test_read_item() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, World!";
        std::fs::write(&file_path, content).unwrap();

        let backend = LocalStorageBackend::new();
        let data = backend
            .read_item(file_path.to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(data, content);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let backend = LocalStorageBackend::new();
        let result = backend.read_item("/nonexistent/file.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_items_nonexistent_directory() {
        let backend = LocalStorageBackend::new();
        let result = backend.list_items("/nonexistent/directory").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_items_recursive_nonexistent_directory() {
        let backend = LocalStorageBackend::new();
        let result = backend.list_items_recursive("/nonexistent/directory").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_exists() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("exists.txt");
        std::fs::write(&file_path, b"content").unwrap();

        let backend = LocalStorageBackend::new();

        // 存在するファイルのテスト
        let exists = backend.exists(file_path.to_str().unwrap()).await.unwrap();
        assert!(exists);

        // 存在しないファイルのテスト
        let not_exists = backend.exists("/nonexistent/file.txt").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_delete_item_success() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("delete_me.txt");
        std::fs::write(&file_path, b"content").unwrap();

        let backend = LocalStorageBackend::new();

        // ファイルが存在することを確認
        assert!(file_path.exists());

        // ファイルを削除
        backend
            .delete_item(file_path.to_str().unwrap())
            .await
            .unwrap();

        // ファイルが削除されたことを確認
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_file() {
        let backend = LocalStorageBackend::new();
        let result = backend.delete_item("/nonexistent/file.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_directory_fails() {
        let temp_dir = tempdir().unwrap();
        let backend = LocalStorageBackend::new();

        let result = backend.delete_item(temp_dir.path().to_str().unwrap()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot delete directory"));
    }

    #[tokio::test]
    async fn test_path_to_storage_item_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.jpg");
        std::fs::write(&file_path, b"image data").unwrap();

        let item = LocalStorageBackend::path_to_storage_item(&file_path).unwrap();
        assert_eq!(item.name, "test.jpg");
        assert_eq!(item.extension, Some("jpg".to_string()));
        assert!(!item.is_directory);
        assert_eq!(item.size, 10); // "image data" は 10 bytes
    }

    #[tokio::test]
    async fn test_path_to_storage_item_directory() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        std::fs::create_dir(&dir_path).unwrap();

        let item = LocalStorageBackend::path_to_storage_item(&dir_path).unwrap();
        assert_eq!(item.name, "test_dir");
        assert_eq!(item.extension, None);
        assert!(item.is_directory);
    }

    #[tokio::test]
    async fn test_path_to_storage_item_nonexistent() {
        let nonexistent_path = std::path::Path::new("/nonexistent/path");
        let result = LocalStorageBackend::path_to_storage_item(nonexistent_path);
        assert!(result.is_err());
    }
}
