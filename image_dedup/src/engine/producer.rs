// Producer - ファイル配信機能

use anyhow::Result;
use tokio::sync::mpsc;

/// Producer: ファイルパスを配信
pub fn spawn_producer(
    files: Vec<String>,
    work_tx: mpsc::Sender<String>,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        for file_path in files {
            if (work_tx.send(file_path).await).is_err() {
                // チャンネルが閉じられた場合は正常終了
                break;
            }
        }
        // work_txをドロップしてチャンネル終了シグナル
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_producer_sends_all_files() {
        let files = vec![
            "/test1.jpg".to_string(),
            "/test2.png".to_string(),
            "/test3.gif".to_string(),
        ];

        let (work_tx, mut work_rx) = mpsc::channel::<String>(10);

        // Producer起動
        let producer_handle = spawn_producer(files.clone(), work_tx);

        // 全ファイルを受信
        let mut received = Vec::new();
        while let Ok(Some(file_path)) = timeout(Duration::from_millis(100), work_rx.recv()).await {
            received.push(file_path);
        }

        // Producer完了確認
        producer_handle.await.unwrap().unwrap();

        // 送信内容確認
        assert_eq!(received.len(), 3);
        assert_eq!(received, files);
    }

    #[tokio::test]
    async fn test_producer_empty_files() {
        let files: Vec<String> = vec![];
        let (work_tx, mut work_rx) = mpsc::channel::<String>(10);

        let producer_handle = spawn_producer(files, work_tx);

        // チャンネルが即座に閉じることを確認
        let received = timeout(Duration::from_millis(100), work_rx.recv()).await;
        assert!(received.is_err() || received.unwrap().is_none());

        producer_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_producer_channel_closed_early() {
        let files = vec!["/test1.jpg".to_string(), "/test2.jpg".to_string()];
        let (work_tx, work_rx) = mpsc::channel::<String>(1);

        // 受信側を即座に閉じる
        drop(work_rx);

        let producer_handle = spawn_producer(files, work_tx);

        // Producerはエラーなく終了すべき
        producer_handle.await.unwrap().unwrap();
    }
}
