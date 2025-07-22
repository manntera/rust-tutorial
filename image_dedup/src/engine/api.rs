// 高レベル公開API
// ProcessingPipelineを簡単に使用できるようにするための便利な関数

use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
    core::{ProcessingConfig, ProgressReporter, HashPersistence, ProcessingSummary, error::ProcessingResult},
    services::{DefaultProcessingConfig, ConsoleProgressReporter, MemoryHashPersistence, NoOpProgressReporter},
};
use super::ProcessingEngine;

// ========================================
// DI対応API - ProcessingEngineベース
// ========================================

/// 設定済みProcessingEngineでディレクトリを処理（DI推奨）
/// 
/// 全ての依存関係が事前注入されたエンジンを使用する真のDI API
pub async fn process_directory_with_engine<L, H, S, C, R, P>(
    directory: &str,
    engine: &ProcessingEngine<L, H, S, C, R, P>,
) -> ProcessingResult<ProcessingSummary>
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
) -> ProcessingResult<ProcessingSummary>
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
) -> ProcessingEngine<L, H, S, DefaultProcessingConfig, NoOpProgressReporter, MemoryHashPersistence>
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
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    )
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image_loader::standard::StandardImageLoader,
        perceptual_hash::dct_hash::DCTHasher,
        storage::local::LocalStorageBackend,
    };
    // Local test utility
    const MINIMAL_PNG_DATA: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn create_test_png_file(filename: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
        let png_file = temp_dir.path().join(filename);
        std::fs::write(&png_file, MINIMAL_PNG_DATA).expect("Failed to write PNG file");
        (temp_dir, png_file)
    }
    use tempfile::TempDir;
    use std::fs;


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