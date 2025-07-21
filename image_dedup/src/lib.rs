pub mod image_loader;
pub mod perceptual_hash;
pub mod storage;
pub mod processing;

use processing::{
    ProcessingEngine,
    traits::{ProcessingConfig, ProgressReporter, HashPersistence},
    DefaultProcessingConfig,
    ConsoleProgressReporter,
    NoOpProgressReporter,
    MemoryHashPersistence,
};

// DIコンテナの役割を果たすジェネリックなApp構造体
// 依存関係を直接所有し、必要に応じてArc<App>で共有する設計
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

    /// アプリケーションの主要なロジック（シンプルな逐次処理）
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

    // ========================================
    // 並列処理エンジン作成メソッド
    // ========================================

    /// デフォルト設定の並列処理エンジンを作成
    /// 
    /// 依存関係をクローンして新しいエンジンを作成
    /// Cloneが重い場合は、Arc<App>を使用して共有することを推奨
    pub fn create_processing_engine(
        &self,
    ) -> ProcessingEngine<L, H, S, DefaultProcessingConfig, ConsoleProgressReporter, MemoryHashPersistence>
    where
        L: Clone + 'static,
        H: Clone + 'static,
        S: Clone + 'static,
    {
        ProcessingEngine::new(
            self.loader.clone(),
            self.hasher.clone(),
            self.storage.clone(),
            DefaultProcessingConfig::default(),
            ConsoleProgressReporter::new(),
            MemoryHashPersistence::new(),
        )
    }


    /// 静音版の並列処理エンジンを作成（バックグラウンド処理用）
    pub fn create_quiet_processing_engine(
        &self,
    ) -> ProcessingEngine<L, H, S, DefaultProcessingConfig, NoOpProgressReporter, MemoryHashPersistence>
    where
        L: Clone + 'static,
        H: Clone + 'static,
        S: Clone + 'static,
    {
        ProcessingEngine::new(
            self.loader.clone(),
            self.hasher.clone(),
            self.storage.clone(),
            DefaultProcessingConfig::default(),
            NoOpProgressReporter::new(),
            MemoryHashPersistence::new(),
        )
    }

    /// カスタム設定で並列処理エンジンを作成
    pub fn create_custom_processing_engine<C, R, P>(
        &self,
        config: C,
        reporter: R,
        persistence: P,
    ) -> ProcessingEngine<L, H, S, C, R, P>
    where
        L: Clone + 'static,
        H: Clone + 'static,
        S: Clone + 'static,
        C: ProcessingConfig,
        R: ProgressReporter + 'static,
        P: HashPersistence + 'static,
    {
        ProcessingEngine::new(
            self.loader.clone(),
            self.hasher.clone(),
            self.storage.clone(),
            config,
            reporter,
            persistence,
        )
    }

    /// 並列処理でディレクトリを処理（高レベル便利メソッド）
    pub async fn run_parallel(&self, path: &str) -> anyhow::Result<processing::ProcessingSummary>
    where
        L: Clone + 'static,
        H: Clone + 'static,
        S: Clone + 'static,
    {
        let engine = self.create_processing_engine();
        engine.process_directory(path).await
            .map_err(|e| anyhow::anyhow!("並列処理エラー: {e}"))
    }

    /// 静音並列処理でディレクトリを処理（バックグラウンド用）
    pub async fn run_parallel_quiet(&self, path: &str) -> anyhow::Result<processing::ProcessingSummary>
    where
        L: Clone + 'static,
        H: Clone + 'static,
        S: Clone + 'static,
    {
        let engine = self.create_quiet_processing_engine();
        engine.process_directory(path).await
            .map_err(|e| anyhow::anyhow!("静音並列処理エラー: {e}"))
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

    #[test]
    fn test_create_processing_engine() {
        let app = App::new(
            StandardImageLoader::new(),
            AverageHasher::new(8),
            crate::storage::local::LocalStorageBackend::new(),
        );

        let engine = app.create_processing_engine();
        
        // エンジンが正常に作成されることを確認
        assert_eq!(engine.config().max_concurrent_tasks(), num_cpus::get().max(1) * 2);
        assert!(engine.config().enable_progress_reporting());
    }

    #[test]
    fn test_create_quiet_processing_engine() {
        let app = App::new(
            StandardImageLoader::new(),
            AverageHasher::new(8),
            crate::storage::local::LocalStorageBackend::new(),
        );

        let engine = app.create_quiet_processing_engine();
        
        // 静音エンジンが正常に作成されることを確認
        assert_eq!(engine.config().max_concurrent_tasks(), num_cpus::get().max(1) * 2);
        assert!(engine.config().enable_progress_reporting()); // 設定は有効だがNoOpReporterが静音
    }

    #[test]
    fn test_create_custom_processing_engine() {
        let app = App::new(
            StandardImageLoader::new(),
            AverageHasher::new(8),
            crate::storage::local::LocalStorageBackend::new(),
        );

        let custom_config = DefaultProcessingConfig::default()
            .with_max_concurrent(4)
            .with_batch_size(10);
        
        let engine = app.create_custom_processing_engine(
            custom_config,
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        // カスタム設定が反映されることを確認
        assert_eq!(engine.config().max_concurrent_tasks(), 4);
        assert_eq!(engine.config().batch_size(), 10);
    }

    #[tokio::test]
    async fn test_run_parallel_quiet_empty_directory() {
        let app = App::new(
            StandardImageLoader::new(),
            AverageHasher::new(8),
            crate::storage::local::LocalStorageBackend::new(),
        );

        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        let result = app.run_parallel_quiet(temp_path).await.unwrap();

        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }
}
