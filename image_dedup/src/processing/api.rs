// 高レベル公開API
// ProcessingPipelineを簡単に使用できるようにするための便利な関数

use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
};
use super::{
    ProcessingPipeline,
    ProcessingEngine,
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
    reporter: R,
    persistence: P,
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
    pipeline.execute(files, config, Arc::new(reporter), Arc::new(persistence)).await
}

/// ファイルリストを直接並列処理（ファイル発見済み）
pub async fn process_files_parallel<L, H, C, R, P>(
    files: Vec<String>,
    loader: L,
    hasher: H,
    config: &C,
    reporter: R,
    persistence: P,
) -> Result<ProcessingSummary>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    C: ProcessingConfig,
    R: ProgressReporter + 'static,
    P: HashPersistence + 'static,
{
    let pipeline = ProcessingPipeline::new(Arc::new(loader), Arc::new(hasher));
    pipeline.execute(files, config, Arc::new(reporter), Arc::new(persistence)).await
}

// ========================================
// 新しいDI対応API - ProcessingEngineベース
// ========================================

/// 設定済みProcessingEngineでディレクトリを処理（DI推奨）
/// 
/// 全ての依存関係が事前注入されたエンジンを使用する真のDI API
pub async fn process_directory_with_engine<L, H, S, C, R, P>(
    directory: &str,
    engine: &ProcessingEngine<L, H, S, C, R, P>,
) -> Result<ProcessingSummary>
where
    L: ImageLoaderBackend + Clone + 'static,
    H: PerceptualHashBackend + Clone + 'static,
    S: StorageBackend + 'static,
    C: ProcessingConfig,
    R: ProgressReporter + Clone + 'static,
    P: HashPersistence + Clone + 'static,
{
    engine.process_directory(directory).await
}

/// 設定済みProcessingEngineでファイルリストを処理（DI推奨）
/// 
/// ファイル発見を済ませた場合に使用する細かい制御用API
pub async fn process_files_with_engine<L, H, S, C, R, P>(
    files: Vec<String>,
    engine: &ProcessingEngine<L, H, S, C, R, P>,
) -> Result<ProcessingSummary>
where
    L: ImageLoaderBackend + Clone + 'static,
    H: PerceptualHashBackend + Clone + 'static,
    S: StorageBackend + 'static,
    C: ProcessingConfig,
    R: ProgressReporter + Clone + 'static,
    P: HashPersistence + Clone + 'static,
{
    engine.process_files(files).await
}

/// ProcessingEngine作成のヘルパー関数
/// 
/// デフォルト設定での簡単なエンジン作成
pub fn create_default_processing_engine<L, H, S>(
    loader: L,
    hasher: H,
    storage: S,
) -> ProcessingEngine<L, H, S, DefaultProcessingConfig, ConsoleProgressReporter, MemoryHashPersistence>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    ProcessingEngine::new(
        loader,
        hasher,
        storage,
        DefaultProcessingConfig::default(),
        ConsoleProgressReporter::new(),
        MemoryHashPersistence::new(),
    )
}

/// ProcessingEngine作成のヘルパー関数（静音版）
/// 
/// テストやバックグラウンド処理用の静音エンジン作成
pub fn create_quiet_processing_engine<L, H, S>(
    loader: L,
    hasher: H,
    storage: S,
) -> ProcessingEngine<L, H, S, DefaultProcessingConfig, super::NoOpProgressReporter, MemoryHashPersistence>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    ProcessingEngine::new(
        loader,
        hasher,
        storage,
        DefaultProcessingConfig::default(),
        super::NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    )
}

// ========================================
// レガシーAPI - 後方互換性のため保持
// ========================================

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
    use crate::processing::tests::create_test_png_file;
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
        let (_temp_dir, test_file) = create_test_png_file("test.png");
        let files = vec![test_file.to_str().unwrap().to_string()];
        
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        let config = DefaultProcessingConfig::default();
        let reporter = super::super::NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
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

    // ========================================
    // 新しいDI対応APIのテスト
    // ========================================

    #[tokio::test]
    async fn test_process_directory_with_engine() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        // テスト用画像作成
        let (_temp_file, test_file) = create_test_png_file("test.png");
        fs::copy(&test_file, temp_dir.path().join("test.png")).unwrap();

        let engine = create_quiet_processing_engine(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );

        let result = process_directory_with_engine(temp_path, &engine).await.unwrap();

        assert_eq!(result.total_files, 1);
        assert_eq!(result.processed_files, 1);
        assert_eq!(result.error_count, 0);

        // 結果がエンジンの永続化に保存されていることを確認
        assert_eq!(engine.persistence().stored_count(), 1);
    }

    #[tokio::test]
    async fn test_process_files_with_engine() {
        let (_temp_dir, test_file) = create_test_png_file("test.png");
        let files = vec![test_file.to_str().unwrap().to_string()];

        let engine = create_quiet_processing_engine(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );

        let result = process_files_with_engine(files, &engine).await.unwrap();

        assert_eq!(result.total_files, 1);
        assert_eq!(result.processed_files, 1);
        assert_eq!(result.error_count, 0);
        assert_eq!(engine.persistence().stored_count(), 1);
    }

    #[test]
    fn test_create_default_processing_engine() {
        let engine = create_default_processing_engine(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );

        assert_eq!(engine.config().max_concurrent_tasks(), num_cpus::get().max(1) * 2);
        assert!(engine.config().enable_progress_reporting());
    }

    #[test]
    fn test_create_quiet_processing_engine() {
        let engine = create_quiet_processing_engine(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );

        assert_eq!(engine.config().max_concurrent_tasks(), num_cpus::get().max(1) * 2);
        assert!(engine.config().enable_progress_reporting()); // 設定は有効だが、NoOpReporterが静音
    }
}