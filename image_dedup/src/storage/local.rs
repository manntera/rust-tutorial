use super::{StorageBackend, StorageItem};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use walkdir::WalkDir;

/// ローカルファイルシステム用のストレージバックエンド
pub struct LocalStorageBackend;

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
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn list_items(&self, prefix: &str) -> Result<Vec<StorageItem>> {
        let path = Path::new(prefix);
        let mut items = Vec::new();

        for entry in WalkDir::new(path) {
            let entry = entry?;

            if let Ok(item) = Self::path_to_storage_item(entry.path()) {
                if !item.is_directory && self.is_image_file(&item) {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    async fn read_item(&self, id: &str) -> Result<Vec<u8>> {
        let path = Path::new(id);
        let data = tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read file: {}", id))?;
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
                .with_context(|| format!("Failed to delete file: {}", id))?;
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

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.name == "image1.jpg"));
        assert!(items.iter().any(|i| i.name == "image2.png"));
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
