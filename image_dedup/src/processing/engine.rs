// Phase 3: コア処理エンジン実装
// ParallelProcessingEngine - 依存性注入による並列処理エンジン

use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
};
use super::{ProcessingMetadata, ProcessingResult};
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use std::path::Path;

/// 依存性注入によるコア処理エンジン
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
    async fn discover_image_files(&self, path: &str) -> Result<Vec<String>> {
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

    /// 単一ファイルの処理
    async fn process_single_file(
        loader: &L,
        hasher: &H,
        file_path: &str,
        _worker_id: usize,
    ) -> ProcessingResult {
        let start_time = Instant::now();
        
        let result = async {
            // 画像読み込み
            let path = Path::new(file_path);
            let load_result = loader.load_from_path(path).await?;
            
            // ファイルサイズを取得
            let file_size = std::fs::metadata(file_path)?.len();
            
            // ハッシュ生成
            let hash_result = hasher.generate_hash(&load_result.image).await?;
            
            // メタデータ作成
            let metadata = ProcessingMetadata {
                file_size,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                image_dimensions: (load_result.image.width(), load_result.image.height()),
                was_resized: load_result.was_resized,
            };
            
            Result::<(String, ProcessingMetadata)>::Ok((hash_result.to_hex(), metadata))
        }.await;
        
        match result {
            Ok((hash, metadata)) => ProcessingResult::Success {
                file_path: file_path.to_string(),
                hash,
                metadata,
            },
            Err(error) => ProcessingResult::Error {
                file_path: file_path.to_string(),
                error: error.to_string(),
            },
        }
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
    async fn test_process_single_file_success() {
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
        
        let result = ParallelProcessingEngine::<StandardImageLoader, DCTHasher, LocalStorageBackend>::process_single_file(
            &loader,
            &hasher,
            test_file.to_str().unwrap(),
            0,
        ).await;
        
        match result {
            ProcessingResult::Success { file_path, hash, metadata } => {
                assert!(file_path.ends_with("test.png"));
                assert!(!hash.is_empty());
                assert_eq!(metadata.image_dimensions, (1, 1));
                // 処理時間は0以上であることを確認（高速処理で0になる場合もある）
                assert!(metadata.processing_time_ms >= 0);
            }
            ProcessingResult::Error { .. } => panic!("Expected success"),
        }
    }
    
    #[tokio::test]
    async fn test_process_single_file_error() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = temp_dir.path().join("invalid.jpg");
        fs::write(&invalid_file, b"not a valid image").unwrap();
        
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        
        let result = ParallelProcessingEngine::<StandardImageLoader, DCTHasher, LocalStorageBackend>::process_single_file(
            &loader,
            &hasher,
            invalid_file.to_str().unwrap(),
            0,
        ).await;
        
        match result {
            ProcessingResult::Success { .. } => panic!("Expected error"),
            ProcessingResult::Error { file_path, error } => {
                assert!(file_path.ends_with("invalid.jpg"));
                assert!(!error.is_empty());
            }
        }
    }
    
    #[tokio::test]
    async fn test_process_nonexistent_file() {
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        
        let result = ParallelProcessingEngine::<StandardImageLoader, DCTHasher, LocalStorageBackend>::process_single_file(
            &loader,
            &hasher,
            "/nonexistent/file.jpg",
            0,
        ).await;
        
        match result {
            ProcessingResult::Success { .. } => panic!("Expected error"),
            ProcessingResult::Error { file_path, error } => {
                assert_eq!(file_path, "/nonexistent/file.jpg");
                assert!(!error.is_empty());
            }
        }
    }
}