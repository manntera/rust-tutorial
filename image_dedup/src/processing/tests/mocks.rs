// テスト用のモック実装
// 各トレイトのテスト用モック実装を提供

use super::super::traits::{ProcessingConfig, ProgressReporter, ParallelProcessor};
use super::super::types::ProcessingSummary;
use async_trait::async_trait;
use anyhow::Result;
use std::sync::{Arc, Mutex};

// ProcessingConfig のモック実装
pub struct MockProcessingConfig {
    pub max_concurrent: usize,
    pub buffer_size: usize,
    pub batch_size: usize,
    pub enable_progress: bool,
}

impl ProcessingConfig for MockProcessingConfig {
    fn max_concurrent_tasks(&self) -> usize {
        self.max_concurrent
    }
    
    fn channel_buffer_size(&self) -> usize {
        self.buffer_size
    }
    
    fn batch_size(&self) -> usize {
        self.batch_size
    }
    
    fn enable_progress_reporting(&self) -> bool {
        self.enable_progress
    }
}

// ProgressReporter のモック実装
#[derive(Default)]
pub struct MockProgressReporter {
    pub calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ProgressReporter for MockProgressReporter {
    async fn report_started(&self, total_files: usize) {
        self.calls.lock().unwrap().push(format!("started:{total_files}"));
    }
    
    async fn report_progress(&self, completed: usize, total: usize) {
        self.calls.lock().unwrap().push(format!("progress:{completed}:{total}"));
    }
    
    async fn report_error(&self, file_path: &str, error: &str) {
        self.calls.lock().unwrap().push(format!("error:{file_path}:{error}"));
    }
    
    async fn report_completed(&self, total_processed: usize, total_errors: usize) {
        self.calls.lock().unwrap().push(format!("completed:{total_processed}:{total_errors}"));
    }
}

// HashPersistence のモック実装 (統合された MemoryHashPersistence を使用)
pub use super::super::data_persistence::implementations::MemoryHashPersistence as MockHashPersistence;

// ParallelProcessor のモック実装
pub struct MockParallelProcessor;

#[async_trait]
impl ParallelProcessor for MockParallelProcessor {
    type Config = MockProcessingConfig;
    type Reporter = MockProgressReporter;
    type Persistence = MockHashPersistence;
    
    async fn process_directory(
        &self,
        _path: &str,
        _config: &Self::Config,
        _reporter: &Self::Reporter,
        _persistence: &Self::Persistence,
    ) -> Result<ProcessingSummary> {
        Ok(ProcessingSummary {
            total_files: 0,
            processed_files: 0,
            error_count: 0,
            total_processing_time_ms: 0,
            average_time_per_file_ms: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_config_trait() {
        let config = MockProcessingConfig {
            max_concurrent: 8,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        };
        
        assert_eq!(config.max_concurrent_tasks(), 8);
        assert_eq!(config.channel_buffer_size(), 100);
        assert_eq!(config.batch_size(), 50);
        assert!(config.enable_progress_reporting());
    }

    #[test]
    fn test_processing_config_thread_safety() {
        let config = MockProcessingConfig {
            max_concurrent: 4,
            buffer_size: 50,
            batch_size: 25,
            enable_progress: false,
        };
        
        // Test that config can be shared across threads
        let config_ref: &dyn ProcessingConfig = &config;
        assert_eq!(config_ref.max_concurrent_tasks(), 4);
        assert_eq!(config_ref.channel_buffer_size(), 50);
        assert_eq!(config_ref.batch_size(), 25);
        assert!(!config_ref.enable_progress_reporting());
    }

    #[tokio::test]
    async fn test_progress_reporter_trait() {
        let reporter = MockProgressReporter::default();
        
        reporter.report_started(100).await;
        reporter.report_progress(50, 100).await;
        reporter.report_error("/path/test.jpg", "load failed").await;
        reporter.report_completed(99, 1).await;
        
        let calls = reporter.calls.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0], "started:100");
        assert_eq!(calls[1], "progress:50:100");
        assert_eq!(calls[2], "error:/path/test.jpg:load failed");
        assert_eq!(calls[3], "completed:99:1");
    }

    #[test]
    fn test_progress_reporter_thread_safety() {
        let reporter = MockProgressReporter::default();
        let reporter_ref: &dyn ProgressReporter = &reporter;
        
        // Test that reporter can be used as trait object
        assert!(std::ptr::eq(
            reporter_ref as *const dyn ProgressReporter,
            &reporter as &dyn ProgressReporter as *const dyn ProgressReporter
        ));
    }

    #[tokio::test]
    async fn test_hash_persistence_trait() {
        use crate::processing::types::ProcessingMetadata;
        use crate::processing::traits::HashPersistence;
        
        let persistence = MockHashPersistence::default();
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // 単一保存テスト
        persistence.store_hash("/test1.jpg", "hash1", &metadata).await.unwrap();
        
        // バッチ保存テスト
        let batch = vec![
            ("/test2.jpg".to_string(), "hash2".to_string(), metadata.clone()),
            ("/test3.jpg".to_string(), "hash3".to_string(), metadata.clone()),
        ];
        persistence.store_batch(&batch).await.unwrap();
        
        // 完了処理テスト
        persistence.finalize().await.unwrap();
        
        // 統合された MemoryHashPersistence のメソッドを使用
        assert_eq!(persistence.stored_count(), 3);
        assert!(persistence.contains_file("/test1.jpg"));
        assert!(persistence.contains_file("/test2.jpg"));
        assert!(persistence.contains_file("/test3.jpg"));
        
        assert!(persistence.is_finalized());
    }

    #[test]
    fn test_hash_persistence_thread_safety() {
        use crate::processing::traits::HashPersistence;
        
        let persistence = MockHashPersistence::default();
        let persistence_ref: &dyn HashPersistence = &persistence;
        
        // Test that persistence can be used as trait object
        assert!(std::ptr::eq(
            persistence_ref as *const dyn HashPersistence,
            &persistence as &dyn HashPersistence as *const dyn HashPersistence
        ));
    }

    #[tokio::test]
    async fn test_parallel_processor_trait() {
        let processor = MockParallelProcessor;
        let config = MockProcessingConfig {
            max_concurrent: 4,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        };
        let reporter = MockProgressReporter::default();
        let persistence = MockHashPersistence::default();
        
        let result = processor
            .process_directory("/test", &config, &reporter, &persistence)
            .await
            .unwrap();
            
        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
    }

    #[test]
    fn test_parallel_processor_thread_safety() {
        let processor = MockParallelProcessor;
        let processor_ref: &dyn ParallelProcessor<
            Config = MockProcessingConfig, 
            Reporter = MockProgressReporter, 
            Persistence = MockHashPersistence
        > = &processor;
        
        // Test that processor can be used as trait object
        assert!(std::ptr::eq(
            processor_ref as *const dyn ParallelProcessor<
                Config = MockProcessingConfig, 
                Reporter = MockProgressReporter, 
                Persistence = MockHashPersistence
            >,
            &processor as &dyn ParallelProcessor<
                Config = MockProcessingConfig, 
                Reporter = MockProgressReporter, 
                Persistence = MockHashPersistence
            > as *const dyn ParallelProcessor<
                Config = MockProcessingConfig, 
                Reporter = MockProgressReporter, 
                Persistence = MockHashPersistence
            >
        ));
    }
}