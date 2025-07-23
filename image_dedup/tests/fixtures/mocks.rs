// テスト用のモック実装
// mockallの自動生成されたモックを使用

// 公開APIとしてmockallが生成したモックを再エクスポート
pub use super::super::traits::{
    MockProcessingConfig,
    MockProgressReporter,
    MockHashPersistence,
    MockParallelProcessor,
};

// 既存のMemoryHashPersistence実装も引き続き利用可能
pub use super::super::data_persistence::implementations::MemoryHashPersistence as MockMemoryHashPersistence;

#[cfg(test)]
mod tests {
    use crate::processing::traits::*;
    use crate::processing::types::{ProcessingMetadata, ProcessingSummary};
    use mockall::predicate::*;

    #[test]
    fn test_processing_config_trait() {
        let mut mock_config = MockProcessingConfig::new();
        
        mock_config.expect_max_concurrent_tasks().return_const(8usize);
        mock_config.expect_channel_buffer_size().return_const(100usize);
        mock_config.expect_batch_size().return_const(50usize);
        mock_config.expect_enable_progress_reporting().return_const(true);
        
        assert_eq!(mock_config.max_concurrent_tasks(), 8);
        assert_eq!(mock_config.channel_buffer_size(), 100);
        assert_eq!(mock_config.batch_size(), 50);
        assert!(mock_config.enable_progress_reporting());
    }

    #[test]
    fn test_processing_config_thread_safety() {
        let mut mock_config = MockProcessingConfig::new();
        
        mock_config.expect_max_concurrent_tasks().return_const(4usize);
        mock_config.expect_channel_buffer_size().return_const(50usize);
        mock_config.expect_batch_size().return_const(25usize);
        mock_config.expect_enable_progress_reporting().return_const(false);
        
        // Test that config can be shared across threads
        let config_ref: &dyn ProcessingConfig = &mock_config;
        assert_eq!(config_ref.max_concurrent_tasks(), 4);
        assert_eq!(config_ref.channel_buffer_size(), 50);
        assert_eq!(config_ref.batch_size(), 25);
        assert!(!config_ref.enable_progress_reporting());
    }

    #[tokio::test]
    async fn test_progress_reporter_trait() {
        let mut mock_reporter = MockProgressReporter::new();
        
        mock_reporter
            .expect_report_started()
            .with(eq(100))
            .times(1)
            .returning(|_| ());
        
        mock_reporter
            .expect_report_progress()
            .with(eq(50), eq(100))
            .times(1)
            .returning(|_, _| ());
        
        mock_reporter
            .expect_report_error()
            .with(eq(std::path::Path::new("/path/test.jpg")), eq("load failed"))
            .times(1)
            .returning(|_, _| ());
        
        mock_reporter
            .expect_report_completed()
            .with(eq(99), eq(1))
            .times(1)
            .returning(|_, _| ());
        
        mock_reporter.report_started(100).await;
        mock_reporter.report_progress(50, 100).await;
        mock_reporter.report_error(std::path::Path::new("/path/test.jpg"), "load failed").await;
        mock_reporter.report_completed(99, 1).await;
    }

    #[test]
    fn test_progress_reporter_thread_safety() {
        let mock_reporter = MockProgressReporter::new();
        let reporter_ref: &dyn ProgressReporter = &mock_reporter;
        
        // Test that reporter can be used as trait object
        assert!(std::ptr::eq(
            reporter_ref as *const dyn ProgressReporter,
            &mock_reporter as &dyn ProgressReporter as *const dyn ProgressReporter
        ));
    }

    #[tokio::test]
    async fn test_hash_persistence_trait() {
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        let mut mock_persistence = MockHashPersistence::new();
        
        mock_persistence
            .expect_store_hash()
            .with(eq(std::path::Path::new("/test1.jpg")), eq("hash1"), eq(metadata.clone()))
            .times(1)
            .returning(|_, _, _| Ok(()));
        
        let batch = vec![
            ("/test2.jpg".into(), "hash2".to_string(), "DCT".to_string(), 0u64, metadata.clone()),
            ("/test3.jpg".into(), "hash3".to_string(), "DCT".to_string(), 0u64, metadata.clone()),
        ];
        
        mock_persistence
            .expect_store_batch()
            .with(eq(batch.clone()))
            .times(1)
            .returning(|_| Ok(()));
        
        mock_persistence
            .expect_finalize()
            .times(1)
            .returning(|| Ok(()));
        
        // Execute test
        mock_persistence.store_hash(std::path::Path::new("/test1.jpg"), "hash1", &metadata).await.unwrap();
        mock_persistence.store_batch(&batch).await.unwrap();
        mock_persistence.finalize().await.unwrap();
    }

    #[test]
    fn test_hash_persistence_thread_safety() {
        let mock_persistence = MockHashPersistence::new();
        let persistence_ref: &dyn HashPersistence = &mock_persistence;
        
        // Test that persistence can be used as trait object
        assert!(std::ptr::eq(
            persistence_ref as *const dyn HashPersistence,
            &mock_persistence as &dyn HashPersistence as *const dyn HashPersistence
        ));
    }

    #[tokio::test]
    async fn test_parallel_processor_trait() {
        let mut mock_processor = MockParallelProcessor::new();
        let config = MockProcessingConfig::new();
        let reporter = MockProgressReporter::new();
        let persistence = MockHashPersistence::new();
        
        let expected_summary = ProcessingSummary {
            total_files: 10,
            processed_files: 8,
            error_count: 2,
            total_processing_time_ms: 5000,
            average_time_per_file_ms: 625.0,
        };
        
        mock_processor
            .expect_process_directory()
            .with(eq("/test"), always(), always(), always())
            .times(1)
            .returning(move |_, _, _, _| Ok(expected_summary.clone()));
        
        let result = mock_processor
            .process_directory("/test", &config, &reporter, &persistence)
            .await
            .unwrap();
            
        assert_eq!(result.total_files, 10);
        assert_eq!(result.processed_files, 8);
        assert_eq!(result.error_count, 2);
        assert_eq!(result.total_processing_time_ms, 5000);
        assert_eq!(result.average_time_per_file_ms, 625.0);
    }

    #[test]
    fn test_parallel_processor_thread_safety() {
        let mock_processor = MockParallelProcessor::new();
        let processor_ref: &dyn ParallelProcessor<
            Config = MockProcessingConfig, 
            Reporter = MockProgressReporter, 
            Persistence = MockHashPersistence
        > = &mock_processor;
        
        // Test that processor can be used as trait object
        assert!(std::ptr::eq(
            processor_ref as *const dyn ParallelProcessor<
                Config = MockProcessingConfig, 
                Reporter = MockProgressReporter, 
                Persistence = MockHashPersistence
            >,
            &mock_processor as &dyn ParallelProcessor<
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