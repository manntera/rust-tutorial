// Consumer - 並列ワーカー機能

use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
};
use super::super::types::ProcessingResult;
use super::super::image_processing::process_single_file;
use tokio::sync::mpsc;
use std::sync::Arc;
use anyhow::Result;

/// 単一Consumerワーカー
pub fn spawn_single_consumer<L, H>(
    worker_id: usize,
    loader: Arc<L>,
    hasher: Arc<H>,
    work_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<String>>>,
    result_tx: mpsc::Sender<ProcessingResult>,
    semaphore: Arc<tokio::sync::Semaphore>,
) -> tokio::task::JoinHandle<Result<()>>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    tokio::spawn(async move {
        loop {
            // 次の作業を取得
            let file_path = {
                let mut rx = work_rx.lock().await;
                match rx.recv().await {
                    Some(path) => path,
                    None => break, // チャンネル終了
                }
            };
            
            // セマフォで同時実行数制御
            let _permit = semaphore.acquire().await
                .map_err(|e| anyhow::anyhow!("Semaphore error: {}", e))?;
            
            // 単一ファイル処理
            let result = process_single_file(
                loader.as_ref(),
                hasher.as_ref(),
                &file_path,
                worker_id,
            ).await;
            
            // 結果送信
            if (result_tx.send(result).await).is_err() {
                // 結果チャンネルが閉じられた場合は終了
                break;
            }
        }
        Ok(())
    })
}

/// Consumers: 並列ワーカープール
pub fn spawn_consumers<L, H>(
    loader: Arc<L>,
    hasher: Arc<H>,
    work_rx: mpsc::Receiver<String>,
    result_tx: mpsc::Sender<ProcessingResult>,
    semaphore: Arc<tokio::sync::Semaphore>,
    worker_count: usize,
) -> Vec<tokio::task::JoinHandle<Result<()>>>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    let work_rx = Arc::new(tokio::sync::Mutex::new(work_rx));
    let mut handles = Vec::new();
    
    for worker_id in 0..worker_count {
        let handle = spawn_single_consumer(
            worker_id,
            loader.clone(),
            hasher.clone(),
            work_rx.clone(),
            result_tx.clone(),
            semaphore.clone(),
        );
        handles.push(handle);
    }
    
    handles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::standard::StandardImageLoader;
    use crate::perceptual_hash::dct_hash::DCTHasher;
    use crate::processing::ProcessingResult;
    use crate::processing::tests::{MINIMAL_PNG_DATA, create_test_png_file, create_multiple_test_png_files};
    use tempfile::TempDir;
    use std::fs;
    use std::collections::HashSet;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_single_consumer_processes_files() {
        // テスト用画像作成
        let (_temp_dir, test_file) = create_test_png_file("test.png");
        
        // チャンネル作成
        let (work_tx, work_rx) = mpsc::channel::<String>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<ProcessingResult>(10);
        let work_rx = Arc::new(tokio::sync::Mutex::new(work_rx));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        
        // ワーカー起動
        let worker_handle = spawn_single_consumer(
            0,
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
            work_rx,
            result_tx,
            semaphore,
        );
        
        // ファイルパス送信
        work_tx.send(test_file.to_str().unwrap().to_string()).await.unwrap();
        drop(work_tx); // チャンネル終了
        
        // 結果受信
        let result = result_rx.recv().await.unwrap();
        
        // ワーカー完了確認
        worker_handle.await.unwrap().unwrap();
        
        // 結果確認
        match result {
            ProcessingResult::Success { file_path, hash, metadata } => {
                assert!(file_path.ends_with("test.png"));
                assert!(!hash.is_empty());
                assert_eq!(metadata.image_dimensions, (1, 1));
            }
            ProcessingResult::Error { .. } => panic!("Expected success"),
        }
    }
    
    #[tokio::test]
    async fn test_single_consumer_handles_errors() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = temp_dir.path().join("invalid.jpg");
        fs::write(&invalid_file, b"not a valid image").unwrap();
        
        let (work_tx, work_rx) = mpsc::channel::<String>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<ProcessingResult>(10);
        let work_rx = Arc::new(tokio::sync::Mutex::new(work_rx));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        
        let worker_handle = spawn_single_consumer(
            0,
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
            work_rx,
            result_tx,
            semaphore,
        );
        
        work_tx.send(invalid_file.to_str().unwrap().to_string()).await.unwrap();
        drop(work_tx);
        
        let result = result_rx.recv().await.unwrap();
        worker_handle.await.unwrap().unwrap();
        
        match result {
            ProcessingResult::Success { .. } => panic!("Expected error"),
            ProcessingResult::Error { file_path, error } => {
                assert!(file_path.ends_with("invalid.jpg"));
                assert!(!error.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_consumer_pool_processes_multiple_files() {
        // 複数のテスト用画像作成
        let (_temp_dir, test_file_paths) = create_multiple_test_png_files(5);
        let test_files: Vec<String> = test_file_paths.iter()
            .map(|p| p.to_str().unwrap().to_string())
            .collect();
        
        // チャンネル作成
        let (work_tx, work_rx) = mpsc::channel::<String>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<ProcessingResult>(10);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(3));
        
        // Consumer pool起動
        let worker_handles = spawn_consumers(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
            work_rx,
            result_tx,
            semaphore,
            3, // 3つのワーカー
        );
        
        // ファイルパス送信
        for file_path in &test_files {
            work_tx.send(file_path.clone()).await.unwrap();
        }
        drop(work_tx); // チャンネル終了
        
        // 結果収集
        let mut results = Vec::new();
        while results.len() < test_files.len() {
            if let Ok(Some(result)) = timeout(Duration::from_secs(5), result_rx.recv()).await {
                results.push(result);
            } else {
                break;
            }
        }
        
        // ワーカー完了確認
        for handle in worker_handles {
            handle.await.unwrap().unwrap();
        }
        
        // 結果確認
        assert_eq!(results.len(), 5);
        let processed_files: HashSet<String> = results.iter().map(|r| match r {
            ProcessingResult::Success { file_path, .. } => file_path.clone(),
            ProcessingResult::Error { file_path, .. } => file_path.clone(),
        }).collect();
        
        for file_path in &test_files {
            assert!(processed_files.iter().any(|p| p.contains(&format!("test{}.png", 
                file_path.split("test").nth(1).unwrap().split('.').next().unwrap()))));
        }
    }
    
    #[tokio::test]
    async fn test_consumer_pool_with_mixed_results() {
        let temp_dir = TempDir::new().unwrap();
        
        // 有効な画像
        let valid_file = temp_dir.path().join("valid.png");
        fs::write(&valid_file, MINIMAL_PNG_DATA).unwrap();
        
        // 無効な画像
        let invalid_file = temp_dir.path().join("invalid.jpg");
        fs::write(&invalid_file, b"not a valid image").unwrap();
        
        let (work_tx, work_rx) = mpsc::channel::<String>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<ProcessingResult>(10);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(2));
        
        let worker_handles = spawn_consumers(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
            work_rx,
            result_tx,
            semaphore,
            2,
        );
        
        work_tx.send(valid_file.to_str().unwrap().to_string()).await.unwrap();
        work_tx.send(invalid_file.to_str().unwrap().to_string()).await.unwrap();
        drop(work_tx);
        
        let mut success_count = 0;
        let mut error_count = 0;
        
        for _ in 0..2 {
            if let Ok(Some(result)) = timeout(Duration::from_secs(5), result_rx.recv()).await {
                match result {
                    ProcessingResult::Success { .. } => success_count += 1,
                    ProcessingResult::Error { .. } => error_count += 1,
                }
            }
        }
        
        for handle in worker_handles {
            handle.await.unwrap().unwrap();
        }
        
        assert_eq!(success_count, 1);
        assert_eq!(error_count, 1);
    }
}