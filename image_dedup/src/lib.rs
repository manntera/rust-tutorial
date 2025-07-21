pub mod image_loader;
pub mod perceptual_hash;
pub mod storage;

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
