// Pipeline - Producer-Consumer パイプライン
// メインパイプライン機能とオーケストレーション

use super::{consumer::spawn_consumers, producer::spawn_producer};
use crate::{
    core::{HashPersistence, ProcessingConfig, ProcessingSummary, ProgressReporter},
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    services::persistence::spawn_result_collector,
};
use anyhow::Result;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::mpsc;

/// 責任が明確に分離されたパイプライン
pub struct ProcessingPipeline<L, H> {
    loader: Arc<L>,
    hasher: Arc<H>,
}

impl<L, H> ProcessingPipeline<L, H>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    /// 新しいパイプラインを作成
    pub fn new(loader: Arc<L>, hasher: Arc<H>) -> Self {
        Self { loader, hasher }
    }

    /// ファイルリストを処理
    pub async fn execute<C, R, P>(
        &self,
        files: Vec<String>,
        config: &C,
        reporter: Arc<R>,
        persistence: Arc<P>,
    ) -> Result<ProcessingSummary>
    where
        C: ProcessingConfig,
        R: ProgressReporter + 'static,
        P: HashPersistence + 'static,
    {
        let start_time = Instant::now();

        // Producer-Consumerチャンネル構築
        let (work_tx, work_rx) = mpsc::channel::<String>(config.channel_buffer_size());
        let (result_tx, result_rx) = mpsc::channel(config.channel_buffer_size());

        // 同期プリミティブ - AtomicUsizeで効率的なカウンター
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrent_tasks()));
        let processed_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));

        let total_files = files.len();
        reporter.report_started(total_files).await;

        // Producer起動
        let producer_handle = spawn_producer(files, work_tx);

        // Consumer Pool起動
        let consumer_handles = spawn_consumers(
            Arc::clone(&self.loader),
            Arc::clone(&self.hasher),
            work_rx,
            result_tx.clone(),
            semaphore,
            config.max_concurrent_tasks(),
        );

        // Result Collector起動
        let collector_handle = spawn_result_collector(
            result_rx,
            total_files,
            processed_count.clone(),
            error_count.clone(),
            reporter.clone(),
            persistence.clone(),
            config.batch_size(),
        );

        // Producer完了を待機
        producer_handle.await??;

        // Consumer完了を待機
        for handle in consumer_handles {
            handle.await??;
        }

        // result_txを閉じてCollectorに完了を通知
        drop(result_tx);

        // Collector完了を待機
        collector_handle.await??;

        // 完了報告 - AtomicUsizeからのload
        let final_processed = processed_count.load(Ordering::Relaxed);
        let final_errors = error_count.load(Ordering::Relaxed);
        reporter
            .report_completed(final_processed, final_errors)
            .await;

        // 永続化完了
        persistence.finalize().await?;

        let total_time_ms = start_time.elapsed().as_millis() as u64;
        let average_time_per_file_ms = if total_files > 0 {
            total_time_ms as f64 / total_files as f64
        } else {
            0.0
        };

        Ok(ProcessingSummary {
            total_files,
            processed_files: final_processed,
            error_count: final_errors,
            total_processing_time_ms: total_time_ms,
            average_time_per_file_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::standard::StandardImageLoader;
    use crate::perceptual_hash::dct_hash::DctHasher;
    use crate::services::{DefaultProcessingConfig, MemoryHashPersistence, NoOpProgressReporter};

    #[tokio::test]
    async fn test_processing_pipeline_creation() {
        let loader = Arc::new(StandardImageLoader::new());
        let hasher = Arc::new(DctHasher::new(8));

        let _pipeline = ProcessingPipeline::new(loader, hasher);

        // パイプラインが正常に作成されることを確認
    }

    #[tokio::test]
    async fn test_processing_pipeline_empty_files() {
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DctHasher::new(8)),
        );

        let config = DefaultProcessingConfig::default();
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();

        let result = pipeline
            .execute(vec![], &config, Arc::new(reporter), Arc::new(persistence))
            .await
            .unwrap();

        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }

    #[tokio::test]
    async fn test_pipeline_end_to_end() {
        use std::fs;
        use tempfile::TempDir;
        // Local test utility
        const SMALL_PNG: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        // 複数のテスト用画像作成
        let temp_dir = TempDir::new().unwrap();
        let mut test_files = Vec::new();

        // 有効な画像ファイル作成
        for i in 0..3 {
            let test_file = temp_dir.path().join(format!("valid{i}.png"));
            fs::write(&test_file, SMALL_PNG).unwrap();
            test_files.push(test_file.to_str().unwrap().to_string());
        }

        // 無効なファイル作成
        let invalid_file = temp_dir.path().join("invalid.jpg");
        fs::write(&invalid_file, b"not a valid image").unwrap();
        test_files.push(invalid_file.to_str().unwrap().to_string());

        // パイプライン実行
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DctHasher::new(8)),
        );

        let config = DefaultProcessingConfig::default()
            .with_max_concurrent(2)
            .with_batch_size(2);
        let reporter = Arc::new(NoOpProgressReporter::new());
        let persistence = Arc::new(MemoryHashPersistence::new());

        let summary = pipeline
            .execute(test_files, &config, reporter, persistence.clone())
            .await
            .unwrap();

        // 結果確認
        assert_eq!(summary.total_files, 4);
        assert_eq!(summary.processed_files, 3); // 有効なファイル3つ
        assert_eq!(summary.error_count, 1); // 無効なファイル1つ
        assert!(summary.total_processing_time_ms > 0);
        assert!(summary.average_time_per_file_ms > 0.0);

        // 永続化確認
        let stored_data = persistence.get_stored_data().unwrap();
        assert_eq!(stored_data.len(), 3);

        for i in 0..3 {
            assert!(stored_data
                .iter()
                .any(|(path, _)| path.contains(&format!("valid{i}.png"))));
        }
    }

    #[tokio::test]
    async fn test_pipeline_with_high_concurrency() {
        use std::fs;
        use tempfile::TempDir;
        // Local test utility
        const SMALL_PNG: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        let temp_dir = TempDir::new().unwrap();
        let mut test_files = Vec::new();

        // 10個のファイル作成
        for i in 0..10 {
            let test_file = temp_dir.path().join(format!("test{i}.png"));
            fs::write(&test_file, SMALL_PNG).unwrap();
            test_files.push(test_file.to_str().unwrap().to_string());
        }

        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DctHasher::new(8)),
        );

        let config = DefaultProcessingConfig::default()
            .with_max_concurrent(8) // 高い並列度
            .with_batch_size(3);
        let reporter = Arc::new(NoOpProgressReporter::new());
        let persistence = Arc::new(MemoryHashPersistence::new());

        let summary = pipeline
            .execute(test_files, &config, reporter, persistence.clone())
            .await
            .unwrap();

        assert_eq!(summary.total_files, 10);
        assert_eq!(summary.processed_files, 10);
        assert_eq!(summary.error_count, 0);

        let stored_data = persistence.get_stored_data().unwrap();
        assert_eq!(stored_data.len(), 10);
    }
}
