pub mod image_loader;
pub mod perceptual_hash;
pub mod storage;
pub mod processing;

// DIコンテナの役割を果たすジェネリックなApp構造体
pub struct App<L, H, S>
where
    L: image_loader::ImageLoaderBackend,
    H: perceptual_hash::PerceptualHashBackend,
    S: storage::StorageBackend,
{
    pub loader: L,
    pub hasher: H,
    pub storage: S,
}

impl<L, H, S> App<L, H, S>
where
    L: image_loader::ImageLoaderBackend,
    H: perceptual_hash::PerceptualHashBackend,
    S: storage::StorageBackend,
{
    /// 新しいAppインスタンスを作成（コンストラクタインジェクション）
    pub fn new(loader: L, hasher: H, storage: S) -> Self {
        Self {
            loader,
            hasher,
            storage,
        }
    }

    /// アプリケーションの主要なロジックを実行
    pub async fn run(&self, path: &str) -> anyhow::Result<()> {
        println!("Starting image deduplication process in: {path}");

        let items = self.storage.list_items(path).await?;
        let image_files = items.iter().filter(|item| self.storage.is_image_file(item));

        for item in image_files {
            println!("Processing: {}", item.name);
            // ここで画像の読み込み、ハッシュ化、比較などの処理を実装
            // let image_data = self.storage.read_item(&item.id).await?;
            // let loaded_image = self.loader.load_from_bytes(&image_data).await?;
            // let hash = self.hasher.generate_hash(&loaded_image.image).await?;
            // println!("  - Hash: {}", hash.to_hex());
        }

        println!("Process finished.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::standard::StandardImageLoader;
    use crate::perceptual_hash::average_hash::AverageHasher;
    use crate::storage::{MockStorageBackend, StorageItem};
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_run_with_mock_storage() {
        let mut mock_storage = MockStorageBackend::new();

        // `list_items`が呼ばれたときの振る舞いを定義
        mock_storage
            .expect_list_items()
            .with(eq("test_path"))
            .times(1)
            .returning(|_| {
                Ok(vec![
                    StorageItem {
                        id: "image1.jpg".to_string(),
                        name: "image1.jpg".to_string(),
                        size: 1024,
                        is_directory: false,
                        extension: Some("jpg".to_string()),
                    },
                    StorageItem {
                        id: "not_an_image.txt".to_string(),
                        name: "not_an_image.txt".to_string(),
                        size: 100,
                        is_directory: false,
                        extension: Some("txt".to_string()),
                    },
                ])
            });

        // `is_image_file`が呼ばれたときの振る舞いを定義
        mock_storage
            .expect_is_image_file()
            .returning(|item| matches!(item.extension.as_deref(), Some("jpg")));

        let app = App::new(
            StandardImageLoader::new(),
            AverageHasher::new(8),
            mock_storage,
        );

        let result = app.run("test_path").await;
        assert!(result.is_ok());
    }
}
