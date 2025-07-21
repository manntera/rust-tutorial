// ProcessingEngine - 完全依存性注入による並列処理エンジン
// 全ての依存関係がコンストラクタで注入される真のDIパターン実装

use super::super::super::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
};
use super::super::traits::{ProcessingConfig, ProgressReporter, HashPersistence};
use super::super::types::ProcessingSummary;
use super::super::error::{ProcessingError, ProcessingResult};
use super::pipeline::ProcessingPipeline;
use std::sync::Arc;

/// 完全依存性注入による並列処理エンジン
/// 
/// 全ての依存関係がコンストラクタで注入される真のDIパターンを実装。
/// テスタビリティと保守性を重視した設計。
/// 
/// エンジン自体をArc<ProcessingEngine>でラップして共有する設計。
/// 内部フィールドは直接所有でシンプルに保つ。
pub struct ProcessingEngine<L, H, S, C, R, P> {
    loader: L,
    hasher: H,
    storage: S,
    config: C,
    reporter: R,
    persistence: P,
}

impl<L, H, S, C, R, P> ProcessingEngine<L, H, S, C, R, P>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
    C: ProcessingConfig,
    R: ProgressReporter + 'static,
    P: HashPersistence + 'static,
{
    /// 新しい処理エンジンを作成
    /// 
    /// 全ての依存関係をコンストラクタで注入する（Constructor Injection）
    /// 直接所有権を受け取り、内部でArcは使用しない
    pub fn new(
        loader: L,
        hasher: H,
        storage: S,
        config: C,
        reporter: R,
        persistence: P,
    ) -> Self {
        Self {
            loader,
            hasher,
            storage,
            config,
            reporter,
            persistence,
        }
    }

    /// 指定されたディレクトリを並列処理
    /// 
    /// ファイル発見から処理完了まで全てを管理する高レベルAPI
    pub async fn process_directory(&self, directory: &str) -> ProcessingResult<ProcessingSummary>
    where
        L: Clone,
        H: Clone,
        R: Clone,
        P: Clone,
    {
        // ファイル発見
        let files = self.discover_image_files(directory).await?;
        
        // ファイルリスト処理
        self.process_files(files).await
    }

    /// 指定されたファイルリストを並列処理
    /// 
    /// より細かい制御が必要な場合のAPI
    pub async fn process_files(&self, files: Vec<String>) -> ProcessingResult<ProcessingSummary>
    where
        L: Clone,
        H: Clone,
        R: Clone,
        P: Clone,
    {
        // 内部的にProcessingPipelineを使用（実装詳細）
        // Pipelineが並列実行でArcを必要とするため、ここで参照からArc<T>を作成
        let pipeline = ProcessingPipeline::new(
            Arc::new(self.loader.clone()),
            Arc::new(self.hasher.clone()),
        );

        pipeline.execute(
            files,
            &self.config,
            Arc::new(self.reporter.clone()),
            Arc::new(self.persistence.clone()),
        ).await
        .map_err(|e| ProcessingError::parallel_execution(format!("パイプライン実行エラー: {e}")))
    }

    /// ディレクトリから画像ファイルを発見
    /// 
    /// ストレージバックエンドを使用してファイル発見処理を行う
    async fn discover_image_files(&self, directory: &str) -> ProcessingResult<Vec<String>> {
        // 設定検証
        if self.config.max_concurrent_tasks() == 0 {
            return Err(ProcessingError::configuration("並列タスク数は1以上である必要があります"));
        }
        
        if self.config.batch_size() == 0 {
            return Err(ProcessingError::configuration("バッチサイズは1以上である必要があります"));
        }
        
        let items = self.storage.list_items(directory).await
            .map_err(|e| ProcessingError::file_discovery(directory, e))?;
        
        let mut image_files = Vec::new();
        for item in items {
            if !item.is_directory && self.storage.is_image_file(&item) {
                image_files.push(item.id);
            }
        }
        
        image_files.sort(); // 一貫した順序で処理
        Ok(image_files)
    }

    /// 設定への参照を取得（読み取り専用アクセス）
    pub fn config(&self) -> &C {
        &self.config
    }

    /// レポーターへの参照を取得
    pub fn reporter(&self) -> &R {
        &self.reporter
    }

    /// 永続化への参照を取得
    pub fn persistence(&self) -> &P {
        &self.persistence
    }
}

// ProcessingEngineは直接所有権ベースの単一コンストラクタのみサポート
// 共有が必要な場合はArc<ProcessingEngine>を使用する

impl<L, H, S, C, R, P> ProcessingEngine<L, H, S, C, R, P>
where
    L: ImageLoaderBackend + Clone + 'static,
    H: PerceptualHashBackend + Clone + 'static,
    S: StorageBackend + 'static,
    C: ProcessingConfig,
    R: ProgressReporter + Clone + 'static,
    P: HashPersistence + Clone + 'static,
{
    /// 指定されたディレクトリを並列処理する（設定等を使用）
    pub async fn process_directory_with_settings(&self) -> ProcessingResult<ProcessingSummary>
    where
        L: Clone,
        H: Clone,
        R: Clone,
        P: Clone,
    {
        self.process_directory_with_config(
            ".",  // デフォルトディレクトリ
            &self.config,
            &self.reporter,
            &self.persistence,
        ).await
    }
    
    /// 指定されたディレクトリを指定された設定で並列処理する
    pub async fn process_directory_with_config(
        &self,
        path: &str,
        config: &C,
        reporter: &R,
        persistence: &P,
    ) -> ProcessingResult<ProcessingSummary>
    where
        L: Clone,
        H: Clone,
        R: Clone,
        P: Clone,
    {
        let start_time = std::time::Instant::now();
        
        // 設定検証
        if config.max_concurrent_tasks() == 0 {
            return Err(ProcessingError::configuration("並列タスク数は1以上である必要があります"));
        }
        
        if config.batch_size() == 0 {
            return Err(ProcessingError::configuration("バッチサイズは1以上である必要があります"));
        }
        
        // ファイル発見
        let files = self.discover_image_files(path).await?;
        let total_files = files.len();
        
        if config.enable_progress_reporting() {
            reporter.report_started(total_files).await;
        }
        
        // パイプライン実行
        let pipeline = ProcessingPipeline::new(
            Arc::new(self.loader.clone()),
            Arc::new(self.hasher.clone()),
        );
        
        let mut summary = pipeline.execute(
            files, 
            config, 
            Arc::new(reporter.clone()),
            Arc::new(persistence.clone())
        ).await
        .map_err(|e| ProcessingError::parallel_execution(format!("パイプライン実行エラー: {e}")))?;
        
        // タイミング計測完了
        let total_time = start_time.elapsed().as_millis() as u64;
        summary.total_processing_time_ms = total_time;
        
        if summary.processed_files > 0 {
            summary.average_time_per_file_ms = total_time as f64 / summary.processed_files as f64;
        }
        
        if config.enable_progress_reporting() {
            reporter.report_completed(summary.processed_files, summary.error_count).await;
        }
        
        // 永続化完了処理
        persistence.finalize().await
            .map_err(ProcessingError::persistence)?;
        
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image_loader::standard::StandardImageLoader,
        perceptual_hash::dct_hash::DCTHasher,
        storage::local::LocalStorageBackend,
    };
    use super::super::super::{
        DefaultProcessingConfig,
        ConsoleProgressReporter,
        MemoryHashPersistence,
        ProcessingError,
    };
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_processing_engine_creation() {
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        let storage = LocalStorageBackend::new();
        let config = DefaultProcessingConfig::default();
        let reporter = ConsoleProgressReporter::quiet();
        let persistence = MemoryHashPersistence::new();

        let engine = ProcessingEngine::new(
            loader,
            hasher,
            storage,
            config,
            reporter,
            persistence,
        );

        // エンジン作成が成功すればOK
        assert_eq!(engine.config().max_concurrent_tasks(), num_cpus::get().max(1) * 2);
    }

    #[test]
    fn test_processing_engine_direct_ownership() {
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        let storage = LocalStorageBackend::new();
        let config = DefaultProcessingConfig::default();
        let reporter = ConsoleProgressReporter::quiet();
        let persistence = MemoryHashPersistence::new();

        let engine = ProcessingEngine::new(
            loader,
            hasher,
            storage,
            config,
            reporter,
            persistence,
        );

        // 直接所有権での作成が成功すればOK
        assert!(engine.config().enable_progress_reporting());
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

        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::default(),
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        let files = engine.discover_image_files(temp_path).await.unwrap();

        // 画像ファイルのみが発見されることを確認
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("test1.jpg")));
        assert!(files.iter().any(|f| f.ends_with("test2.png")));
        assert!(!files.iter().any(|f| f.ends_with("not_image.txt")));
    }

    #[tokio::test]
    async fn test_process_files_empty() {
        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::default(),
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        let result = engine.process_files(vec![]).await.unwrap();

        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }

    #[tokio::test]
    async fn test_process_directory_empty() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::default(),
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        let result = engine.process_directory(temp_path).await.unwrap();

        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }

    #[tokio::test]
    async fn test_process_directory_validation_errors() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        // 無効な並列数の設定
        let invalid_config = DefaultProcessingConfig::default().with_max_concurrent(0);
        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            invalid_config,
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        let result = engine.process_directory(temp_path).await;
        assert!(matches!(result, Err(ProcessingError::ConfigurationError { .. })));
        assert!(result.unwrap_err().to_string().contains("並列タスク数は1以上である必要があります"));

        // 無効なバッチサイズの設定
        let invalid_config = DefaultProcessingConfig::default().with_batch_size(0);
        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            invalid_config,
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        let result = engine.process_directory(temp_path).await;
        assert!(matches!(result, Err(ProcessingError::ConfigurationError { .. })));
        assert!(result.unwrap_err().to_string().contains("バッチサイズは1以上である必要があります"));
    }

    #[tokio::test]
    async fn test_process_nonexistent_directory() {
        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::default(),
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        let result = engine.process_directory("/nonexistent/directory").await;
        assert!(matches!(result, Err(ProcessingError::FileDiscoveryError { .. })));
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("ファイル発見エラー"));
        assert!(error.to_string().contains("/nonexistent/directory"));
    }
    
    #[tokio::test]
    async fn test_process_directory_with_config() {
        use crate::processing::tests::test_data::MINIMAL_PNG_DATA;
        
        // テスト用ディレクトリと画像作成
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        // テスト用ファイル作成（有効な画像データを使用）
        fs::write(temp_dir.path().join("test1.png"), MINIMAL_PNG_DATA).unwrap();
        fs::write(temp_dir.path().join("test2.png"), MINIMAL_PNG_DATA).unwrap();
        fs::write(temp_dir.path().join("not_image.txt"), b"text content").unwrap();

        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::default().with_max_concurrent(2),
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );

        // 新しいAPIを使用して処理実行
        let summary = engine.process_directory_with_config(
            temp_path,
            engine.config(),
            engine.reporter(),
            engine.persistence(),
        ).await.unwrap();

        // 結果確認
        assert_eq!(summary.total_files, 2); // 画像ファイルのみ
        assert_eq!(summary.processed_files, 2);
        assert_eq!(summary.error_count, 0);
        assert!(summary.total_processing_time_ms > 0);
        assert!(summary.average_time_per_file_ms > 0.0);

        // 永続化確認
        let stored_data = engine.persistence().get_stored_data();
        assert_eq!(stored_data.len(), 2);
        assert!(engine.persistence().is_finalized());
    }
    
    #[tokio::test]
    async fn test_process_directory_with_errors() {
        use crate::processing::tests::test_data::SMALL_PNG;
        
        let temp_dir = TempDir::new().unwrap();
        
        // 有効な画像
        fs::write(temp_dir.path().join("valid.png"), SMALL_PNG).unwrap();
        // 無効なファイル
        fs::write(temp_dir.path().join("invalid.jpg"), b"not a valid image").unwrap();
        
        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::default(),
            ConsoleProgressReporter::quiet(),
            MemoryHashPersistence::new(),
        );
        
        let summary = engine.process_directory_with_config(
            temp_dir.path().to_str().unwrap(),
            engine.config(),
            engine.reporter(),
            engine.persistence(),
        ).await.unwrap();
        
        assert_eq!(summary.total_files, 2);
        assert_eq!(summary.processed_files, 1); // 有効な画像のみ
        assert_eq!(summary.error_count, 1); // 無効な画像
        
        let stored_data = engine.persistence().get_stored_data();
        assert_eq!(stored_data.len(), 1);
    }
}