// Collector - 結果収集と永続化機能

use crate::core::types::ProcessingOutcome;
use crate::core::{HashPersistence, ProgressReporter};
use anyhow::Result;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::mpsc;

/// Collector: 結果収集と永続化
/// AtomicUsizeを使用して効率的なカウンターを実装
pub fn spawn_result_collector<R, P>(
    mut result_rx: mpsc::Receiver<ProcessingOutcome>,
    total_files: usize,
    processed_count: Arc<AtomicUsize>,
    error_count: Arc<AtomicUsize>,
    reporter: Arc<R>,
    persistence: Arc<P>,
    batch_size: usize,
) -> tokio::task::JoinHandle<Result<()>>
where
    R: ProgressReporter + 'static,
    P: HashPersistence + 'static,
{
    tokio::spawn(async move {
        let mut batch = Vec::with_capacity(batch_size);
        let mut completed = 0;
        let mut errors = 0;

        while let Some(result) = result_rx.recv().await {
            match result {
                ProcessingOutcome::Success {
                    file_path,
                    hash,
                    algorithm,
                    hash_bits,
                    metadata,
                } => {
                    batch.push((file_path, hash, algorithm, hash_bits, metadata));
                    completed += 1;

                    // バッチ永続化
                    if batch.len() >= batch_size {
                        persistence.store_batch(&batch).await?;
                        batch.clear();
                    }
                }
                ProcessingOutcome::Error { file_path, error } => {
                    reporter.report_error(&file_path, &error).await;
                    errors += 1;
                }
            }

            // 進捗報告
            reporter
                .report_progress(completed + errors, total_files)
                .await;
        }

        // 残りバッチの永続化
        if !batch.is_empty() {
            persistence.store_batch(&batch).await?;
        }

        // カウンタ更新 - AtomicUsizeで効率的な更新
        processed_count.store(completed, Ordering::Relaxed);
        error_count.store(errors, Ordering::Relaxed);

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ProcessingMetadata;
    use crate::services::monitoring::implementations::NoOpProgressReporter;
    use crate::services::persistence::implementations::MemoryHashPersistence;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_result_collector_processes_success_results() {
        let (result_tx, result_rx) = mpsc::channel::<ProcessingOutcome>(10);
        let processed_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();

        let collector_handle = spawn_result_collector(
            result_rx,
            3,
            processed_count.clone(),
            error_count.clone(),
            Arc::new(reporter),
            Arc::new(persistence.clone()),
            2, // バッチサイズ
        );

        // 成功結果を送信
        for i in 0..3 {
            let metadata = ProcessingMetadata {
                file_size: 1024,
                processing_time_ms: 100,
                image_dimensions: (512, 512),
                was_resized: false,
            };

            result_tx
                .send(ProcessingOutcome::Success {
                    file_path: format!("/test{i}.jpg").into(),
                    hash: format!("hash{i}"),
                    algorithm: "DCT".to_string(),
                    hash_bits: i as u64,
                    metadata,
                })
                .await
                .unwrap();
        }

        drop(result_tx); // チャンネル終了

        // コレクター完了確認
        collector_handle.await.unwrap().unwrap();

        // 結果確認
        assert_eq!(processed_count.load(Ordering::Relaxed), 3);
        assert_eq!(error_count.load(Ordering::Relaxed), 0);

        let stored_data = persistence.get_stored_data().unwrap();
        assert_eq!(stored_data.len(), 3);
        assert!(stored_data.contains_key("/test0.jpg"));
        assert!(stored_data.contains_key("/test1.jpg"));
        assert!(stored_data.contains_key("/test2.jpg"));
    }

    #[tokio::test]
    async fn test_result_collector_processes_mixed_results() {
        let (result_tx, result_rx) = mpsc::channel::<ProcessingOutcome>(10);
        let processed_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();

        let collector_handle = spawn_result_collector(
            result_rx,
            4,
            processed_count.clone(),
            error_count.clone(),
            Arc::new(reporter),
            Arc::new(persistence.clone()),
            10, // 大きなバッチサイズ
        );

        // 成功結果
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };

        result_tx
            .send(ProcessingOutcome::Success {
                file_path: "/success1.jpg".into(),
                hash: "hash1".to_string(),
                algorithm: "DCT".to_string(),
                hash_bits: 1u64,
                metadata: metadata.clone(),
            })
            .await
            .unwrap();

        result_tx
            .send(ProcessingOutcome::Success {
                file_path: "/success2.jpg".into(),
                hash: "hash2".to_string(),
                algorithm: "DCT".to_string(),
                hash_bits: 2u64,
                metadata,
            })
            .await
            .unwrap();

        // エラー結果
        result_tx
            .send(ProcessingOutcome::Error {
                file_path: "/error1.jpg".into(),
                error: "load failed".to_string(),
            })
            .await
            .unwrap();

        result_tx
            .send(ProcessingOutcome::Error {
                file_path: "/error2.jpg".into(),
                error: "invalid format".to_string(),
            })
            .await
            .unwrap();

        drop(result_tx);
        collector_handle.await.unwrap().unwrap();

        assert_eq!(processed_count.load(Ordering::Relaxed), 2);
        assert_eq!(error_count.load(Ordering::Relaxed), 2);

        let stored_data = persistence.get_stored_data().unwrap();
        assert_eq!(stored_data.len(), 2);
        assert!(stored_data.contains_key("/success1.jpg"));
        assert!(stored_data.contains_key("/success2.jpg"));
    }

    #[tokio::test]
    async fn test_result_collector_batching() {
        let (result_tx, result_rx) = mpsc::channel::<ProcessingOutcome>(10);
        let processed_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();

        let collector_handle = spawn_result_collector(
            result_rx,
            5,
            processed_count.clone(),
            error_count.clone(),
            Arc::new(reporter),
            Arc::new(persistence.clone()),
            2, // バッチサイズ2
        );

        // 5つの成功結果（2+2+1のバッチに分かれるはず）
        for i in 0..5 {
            let metadata = ProcessingMetadata {
                file_size: 1024,
                processing_time_ms: 100,
                image_dimensions: (512, 512),
                was_resized: false,
            };

            result_tx
                .send(ProcessingOutcome::Success {
                    file_path: format!("/test{i}.jpg").into(),
                    hash: format!("hash{i}"),
                    algorithm: "DCT".to_string(),
                    hash_bits: i as u64,
                    metadata,
                })
                .await
                .unwrap();
        }

        drop(result_tx);
        collector_handle.await.unwrap().unwrap();

        assert_eq!(processed_count.load(Ordering::Relaxed), 5);
        assert_eq!(error_count.load(Ordering::Relaxed), 0);

        let stored_data = persistence.get_stored_data().unwrap();
        assert_eq!(stored_data.len(), 5);
    }
}
