// 高レベル公開API
// ProcessingPipelineを簡単に使用できるようにするための便利な関数

use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
};
use super::{
    ProcessingPipeline,
    traits::{ProcessingConfig, ProgressReporter, HashPersistence},
    types::ProcessingSummary,
    DefaultProcessingConfig,
    ConsoleProgressReporter,
    MemoryHashPersistence,
};
use anyhow::Result;
use std::sync::Arc;

/// ディレクトリを並列処理する最も簡単な方法
/// デフォルト設定でコンソール進捗表示、結果はメモリに保存
pub async fn process_directory_parallel<L, H, S>(
    directory: &str,
    loader: L,
    hasher: H,
    storage: S,
) -> Result<ProcessingSummary>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    let files = discover_image_files(&storage, directory).await?;
    
    let pipeline = ProcessingPipeline::new(Arc::new(loader), Arc::new(hasher));
    let config = DefaultProcessingConfig::default();
    let reporter = Arc::new(ConsoleProgressReporter::new());
    let persistence = Arc::new(MemoryHashPersistence::new());

    pipeline.execute(files, &config, reporter, persistence).await
}

/// カスタム設定でディレクトリを並列処理
pub async fn process_directory_with_config<L, H, S, C, R, P>(
    directory: &str,
    loader: L,
    hasher: H,
    storage: S,
    config: &C,
    reporter: Arc<R>,
    persistence: Arc<P>,
) -> Result<ProcessingSummary>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
    C: ProcessingConfig,
    R: ProgressReporter + 'static,
    P: HashPersistence + 'static,
{
    let files = discover_image_files(&storage, directory).await?;
    
    let pipeline = ProcessingPipeline::new(Arc::new(loader), Arc::new(hasher));
    pipeline.execute(files, config, reporter, persistence).await
}

/// ファイルリストを直接並列処理（ファイル発見済み）
pub async fn process_files_parallel<L, H, C, R, P>(
    files: Vec<String>,
    loader: L,
    hasher: H,
    config: &C,
    reporter: Arc<R>,
    persistence: Arc<P>,
) -> Result<ProcessingSummary>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    C: ProcessingConfig,
    R: ProgressReporter + 'static,
    P: HashPersistence + 'static,
{
    let pipeline = ProcessingPipeline::new(Arc::new(loader), Arc::new(hasher));
    pipeline.execute(files, config, reporter, persistence).await
}

/// ディレクトリから画像ファイルを発見（内部ヘルパー）
async fn discover_image_files<S>(storage: &S, directory: &str) -> Result<Vec<String>>
where
    S: StorageBackend,
{
    let items = storage.list_items(directory).await?;
    
    let mut image_files = Vec::new();
    for item in items {
        if !item.is_directory && storage.is_image_file(&item) {
            image_files.push(item.id);
        }
    }
    
    image_files.sort(); // 一貫した順序で処理
    Ok(image_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image_loader::standard::StandardImageLoader,
        perceptual_hash::dct_hash::DCTHasher,
        storage::local::LocalStorageBackend,
    };
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_process_directory_parallel_empty() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        let storage = LocalStorageBackend::new();
        
        let result = process_directory_parallel(temp_path, loader, hasher, storage).await.unwrap();
        
        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }

    #[tokio::test]
    async fn test_discover_image_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        // テスト用ファイル作成
        fs::write(temp_dir.path().join("test1.jpg"), b"fake jpg content").unwrap();
        fs::write(temp_dir.path().join("test2.png"), b"fake png content").unwrap();
        fs::write(temp_dir.path().join("not_image.txt"), b"text content").unwrap();
        
        let storage = LocalStorageBackend::new();
        let files = discover_image_files(&storage, temp_path).await.unwrap();
        
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("test1.jpg")));
        assert!(files.iter().any(|f| f.ends_with("test2.png")));
        assert!(!files.iter().any(|f| f.ends_with("not_image.txt")));
    }
    
    #[tokio::test]
    async fn test_process_files_parallel() {
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
        
        let files = vec![test_file.to_str().unwrap().to_string()];
        
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        let config = DefaultProcessingConfig::default();
        let reporter = Arc::new(super::super::NoOpProgressReporter::new());
        let persistence = Arc::new(MemoryHashPersistence::new());
        
        let result = process_files_parallel(
            files,
            loader,
            hasher,
            &config,
            reporter,
            persistence.clone(),
        ).await.unwrap();
        
        assert_eq!(result.total_files, 1);
        assert_eq!(result.processed_files, 1);
        assert_eq!(result.error_count, 0);
        
        // 結果がメモリに保存されていることを確認
        assert_eq!(persistence.stored_count(), 1);
    }
}