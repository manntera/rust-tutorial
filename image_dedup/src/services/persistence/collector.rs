// Collector - 結果収集と永続化機能

use crate::core::types::ProcessingResult;
use crate::core::{HashPersistence, ProgressReporter};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Collector: 結果収集と永続化
pub fn spawn_result_collector<R, P>(
    mut result_rx: mpsc::Receiver<ProcessingResult>,
    total_files: usize,
    processed_count: Arc<tokio::sync::RwLock<usize>>,
    error_count: Arc<tokio::sync::RwLock<usize>>,
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
                ProcessingResult::Success {
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
                ProcessingResult::Error { file_path, error } => {
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

        // カウンタ更新
        *processed_count.write().await = completed;
        *error_count.write().await = errors;

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
        let (result_tx, result_rx) = mpsc::channel::<ProcessingResult>(10);
        let processed_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let error_count = Arc::new(tokio::sync::RwLock::new(0usize));
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
                .send(ProcessingResult::Success {
                    file_path: format!("/test{i}.jpg"),
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
        assert_eq!(*processed_count.read().await, 3);
        assert_eq!(*error_count.read().await, 0);

        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 3);
        assert!(stored_data.contains_key("/test0.jpg"));
        assert!(stored_data.contains_key("/test1.jpg"));
        assert!(stored_data.contains_key("/test2.jpg"));
    }

    #[tokio::test]
    async fn test_result_collector_processes_mixed_results() {
        let (result_tx, result_rx) = mpsc::channel::<ProcessingResult>(10);
        let processed_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let error_count = Arc::new(tokio::sync::RwLock::new(0usize));
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
            .send(ProcessingResult::Success {
                file_path: "/success1.jpg".to_string(),
                hash: "hash1".to_string(),
                algorithm: "DCT".to_string(),
                hash_bits: 1u64,
                metadata: metadata.clone(),
            })
            .await
            .unwrap();

        result_tx
            .send(ProcessingResult::Success {
                file_path: "/success2.jpg".to_string(),
                hash: "hash2".to_string(),
                algorithm: "DCT".to_string(),
                hash_bits: 2u64,
                metadata,
            })
            .await
            .unwrap();

        // エラー結果
        result_tx
            .send(ProcessingResult::Error {
                file_path: "/error1.jpg".to_string(),
                error: "load failed".to_string(),
            })
            .await
            .unwrap();

        result_tx
            .send(ProcessingResult::Error {
                file_path: "/error2.jpg".to_string(),
                error: "invalid format".to_string(),
            })
            .await
            .unwrap();

        drop(result_tx);
        collector_handle.await.unwrap().unwrap();

        assert_eq!(*processed_count.read().await, 2);
        assert_eq!(*error_count.read().await, 2);

        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 2);
        assert!(stored_data.contains_key("/success1.jpg"));
        assert!(stored_data.contains_key("/success2.jpg"));
    }

    #[tokio::test]
    async fn test_result_collector_batching() {
        let (result_tx, result_rx) = mpsc::channel::<ProcessingResult>(10);
        let processed_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let error_count = Arc::new(tokio::sync::RwLock::new(0usize));
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
                .send(ProcessingResult::Success {
                    file_path: format!("/test{i}.jpg"),
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

        assert_eq!(*processed_count.read().await, 5);
        assert_eq!(*error_count.read().await, 0);

        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 5);
    }
}
