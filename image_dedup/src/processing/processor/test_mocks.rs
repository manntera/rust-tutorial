// テスト用の並列処理制御モック実装

use super::traits::ParallelProcessor;
use crate::processing::types::{ProcessingMetadata, ProcessingSummary};
use crate::processing::config::{ProcessingConfig, MockProcessingConfig};
use crate::processing::reporting::{ProgressReporter, MockProgressReporter};
use crate::processing::persistence::{HashPersistence, MockHashPersistence};

pub struct MockParallelProcessor;

#[async_trait::async_trait]
impl ParallelProcessor for MockParallelProcessor {
    type Config = MockProcessingConfig;
    type Reporter = MockProgressReporter;
    type Persistence = MockHashPersistence;

    async fn process_directory(
        &self,
        _path: &str,
        config: &Self::Config,
        reporter: &Self::Reporter,
        persistence: &Self::Persistence,
    ) -> anyhow::Result<ProcessingSummary> {
        // Mock implementation that uses all provided dependencies
        let total_files = 10;
        
        reporter.report_started(total_files).await;
        
        // Simulate processing with config parameters
        let _max_concurrent = config.max_concurrent_tasks();
        let _batch_size = config.batch_size();
        
        // Simulate progress reporting
        for i in 0..=total_files {
            reporter.report_progress(i, total_files).await;
            if i == 5 {
                reporter.report_error("error_file.jpg", "Mock error").await;
            }
        }
        
        // Store some mock results in batches
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        let batch_data = vec![
            ("file1.jpg".to_string(), "hash1".to_string(), metadata.clone()),
            ("file2.jpg".to_string(), "hash2".to_string(), metadata.clone()),
        ];
        
        persistence.store_batch(&batch_data).await?;
        persistence.finalize().await?;
        
        reporter.report_completed(9, 1).await;
        
        Ok(ProcessingSummary {
            total_files,
            processed_files: 9,
            error_count: 1,
            total_processing_time_ms: 1000,
            average_time_per_file_ms: 111.11,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::config::MockProcessingConfig;
    use crate::processing::reporting::MockProgressReporter;
    use crate::processing::persistence::MockHashPersistence;

    #[tokio::test]
    async fn test_parallel_processor_trait() {
        let processor = MockParallelProcessor;
        let config = MockProcessingConfig {
            max_concurrent: 4,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        };
        let reporter = MockProgressReporter::new();
        let persistence = MockHashPersistence::new();
        
        let result = processor.process_directory(
            "/test/path",
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        // Verify summary
        assert_eq!(result.total_files, 10);
        assert_eq!(result.processed_files, 9);
        assert_eq!(result.error_count, 1);
        assert_eq!(result.total_processing_time_ms, 1000);
        assert!((result.average_time_per_file_ms - 111.11).abs() < 0.01);
        
        // Verify all components were called
        assert!(*reporter.started_called.lock().unwrap());
        assert!(reporter.completed_called.lock().unwrap().is_some());
        assert_eq!(reporter.progress_calls.lock().unwrap().len(), 11); // 0..=10
        assert_eq!(reporter.error_calls.lock().unwrap().len(), 1);
        
        assert_eq!(persistence.stored_hashes.lock().unwrap().len(), 2);
        assert!(*persistence.finalize_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_parallel_processor_thread_safety() {
        let processor = MockParallelProcessor;
        let processor_ref: &dyn ParallelProcessor<
            Config = MockProcessingConfig,
            Reporter = MockProgressReporter,
            Persistence = MockHashPersistence,
        > = &processor;
        
        let config = MockProcessingConfig {
            max_concurrent: 2,
            buffer_size: 50,
            batch_size: 25,
            enable_progress: false,
        };
        let reporter = MockProgressReporter::new();
        let persistence = MockHashPersistence::new();
        
        let result = processor_ref.process_directory(
            "/test",
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(result.total_files, 10);
        assert_eq!(result.processed_files, 9);
    }
}