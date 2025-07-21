// Phase 3: コア処理エンジン実装
// ParallelProcessingEngine - 依存性注入による並列処理エンジン

use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
};
use anyhow::Result;
use std::sync::Arc;

/// 依存性注入によるコア処理エンジン
#[allow(dead_code)]  // Phase 5で process_directory 実装時に使用予定
pub struct ParallelProcessingEngine<L, H, S> {
    loader: Arc<L>,
    hasher: Arc<H>,
    storage: Arc<S>,
}

impl<L, H, S> ParallelProcessingEngine<L, H, S>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    /// コンストラクタインジェクション
    pub fn new(loader: L, hasher: H, storage: S) -> Self {
        Self {
            loader: Arc::new(loader),
            hasher: Arc::new(hasher),
            storage: Arc::new(storage),
        }
    }

    /// ファクトリーメソッド（既存のAppから構築）
    pub fn from_app(app: crate::App<L, H, S>) -> Self {
        Self::new(app.loader, app.hasher, app.storage)
    }
    

    /// ディレクトリから画像ファイルを発見
    pub async fn discover_image_files(&self, path: &str) -> Result<Vec<String>> {
        let items = self.storage.list_items(path).await?;
        
        let mut image_files = Vec::new();
        for item in items {
            if !item.is_directory && self.storage.is_image_file(&item) {
                image_files.push(item.id);
            }
        }
        
        image_files.sort(); // 一貫した順序で処理
        Ok(image_files)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::standard::StandardImageLoader;
    use crate::perceptual_hash::dct_hash::DCTHasher;
    use crate::storage::local::LocalStorageBackend;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_parallel_processing_engine_new() {
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        let storage = LocalStorageBackend::new();
        
        let _engine = ParallelProcessingEngine::new(loader, hasher, storage);
        
        // コンパイルが通れば成功
        assert!(true);
    }
    
    #[test]
    fn test_parallel_processing_engine_from_app() {
        let app = crate::App::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let _engine = ParallelProcessingEngine::from_app(app);
        
        // コンパイルが通れば成功
        assert!(true);
    }

    #[tokio::test]
    async fn test_discover_image_files() {
        // テスト用ディレクトリ作成
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        // テスト用ファイル作成
        fs::write(temp_dir.path().join("test1.jpg"), b"fake jpg content").unwrap();
        fs::write(temp_dir.path().join("test2.png"), b"fake png content").unwrap();
        fs::write(temp_dir.path().join("not_image.txt"), b"text content").unwrap();
        
        // エンジン作成
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        // ファイル発見実行
        let files = engine.discover_image_files(temp_path).await.unwrap();
        
        // 画像ファイルのみが発見されることを確認
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("test1.jpg")));
        assert!(files.iter().any(|f| f.ends_with("test2.png")));
        assert!(!files.iter().any(|f| f.ends_with("not_image.txt")));
    }
    
    #[tokio::test]
    async fn test_discover_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let files = engine.discover_image_files(temp_path).await.unwrap();
        assert_eq!(files.len(), 0);
    }

    #[tokio::test]
    async fn test_process_single_file_integration() {
        use crate::processing::image_processing::process_single_file;
        
        // 1x1の最小PNGファイル（有効な画像データ）
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.png");
        fs::write(&test_file, &png_data).unwrap();
        
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        
        let result = process_single_file(
            &loader,
            &hasher,
            test_file.to_str().unwrap(),
            0,
        ).await;
        
        match result {
            crate::processing::types::ProcessingResult::Success { file_path, hash, metadata } => {
                assert!(file_path.ends_with("test.png"));
                assert!(!hash.is_empty());
                assert_eq!(metadata.image_dimensions, (1, 1));
                assert!(metadata.processing_time_ms < 10000);
            }
            crate::processing::types::ProcessingResult::Error { .. } => panic!("Expected success"),
        }
    }
}