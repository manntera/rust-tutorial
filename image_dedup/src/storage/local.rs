use super::{StorageBackend, StorageItem};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;

/// ローカルファイルシステム用のストレージバックエンド
#[derive(Clone)]
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
        let path = Path::new(prefix);
        let mut items = Vec::new();

        // 非同期でディレクトリを読み込む
        let mut entries = tokio::fs::read_dir(path)
            .await
            .with_context(|| format!("Failed to read directory: {prefix}"))?;

        while let Some(entry) = entries.next_entry().await? {
            if let Ok(item) = Self::path_to_storage_item(&entry.path()) {
                items.push(item);
            }
        }

        Ok(items)
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
        assert!(
            items
                .iter()
                .any(|i| i.name == "root.jpg" && !i.is_directory)
        );
        assert!(items.iter().any(|i| i.name == "subdir" && i.is_directory));
        assert!(
            items
                .iter()
                .any(|i| i.name == "nested.png" && !i.is_directory)
        );
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
}
