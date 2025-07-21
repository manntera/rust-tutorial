// Pipeline - Producer-Consumer パイプライン
// メインパイプライン機能とオーケストレーション

use super::super::super::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
};
use super::super::traits::{ProcessingConfig, ProgressReporter, HashPersistence};
use super::super::types::ProcessingSummary;
use super::{producer::spawn_producer, consumer::spawn_consumers};
use super::super::data_persistence::spawn_result_collector;
use anyhow::Result;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

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
        
        // 同期プリミティブ
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrent_tasks()));
        let processed_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let error_count = Arc::new(tokio::sync::RwLock::new(0usize));
        
        let total_files = files.len();
        reporter.report_started(total_files).await;
        
        // Producer起動
        let producer_handle = spawn_producer(files, work_tx);
        
        // Consumer Pool起動
        let consumer_handles = spawn_consumers(
            self.loader.clone(),
            self.hasher.clone(),
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
        
        // 完了報告
        let final_processed = *processed_count.read().await;
        let final_errors = *error_count.read().await;
        reporter.report_completed(final_processed, final_errors).await;
        
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
    use crate::perceptual_hash::dct_hash::DCTHasher;
    use crate::processing::{
        DefaultProcessingConfig, NoOpProgressReporter, MemoryHashPersistence
    };

    #[tokio::test]
    async fn test_processing_pipeline_creation() {
        let loader = Arc::new(StandardImageLoader::new());
        let hasher = Arc::new(DCTHasher::new(8));
        
        let _pipeline = ProcessingPipeline::new(loader, hasher);
        
        // 基本的な作成テスト
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_processing_pipeline_empty_files() {
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
        );
        
        let config = DefaultProcessingConfig::default();
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let result = pipeline.execute(
            vec![],
            &config,
            Arc::new(reporter),
            Arc::new(persistence),
        ).await.unwrap();
        
        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }
}