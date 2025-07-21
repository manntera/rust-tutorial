// 並列処理システムのトレイト定義
// 全ての抽象化インターフェースを定義

use async_trait::async_trait;
use anyhow::Result;
use super::types::{ProcessingMetadata, ProcessingSummary};

/// 並列処理の設定を抽象化するトレイト
pub trait ProcessingConfig: Send + Sync {
    /// 最大同時実行タスク数を取得
    fn max_concurrent_tasks(&self) -> usize;
    
    /// チャンネルバッファサイズを取得
    fn channel_buffer_size(&self) -> usize;
    
    /// バッチ処理のサイズを取得
    fn batch_size(&self) -> usize;
    
    /// 進捗報告を有効にするかどうか
    fn enable_progress_reporting(&self) -> bool;
}

/// 進捗報告の抽象化トレイト
#[async_trait]
pub trait ProgressReporter: Send + Sync {
    /// 処理開始時の報告
    async fn report_started(&self, total_files: usize);
    
    /// 進捗更新の報告
    async fn report_progress(&self, completed: usize, total: usize);
    
    /// エラー発生時の報告
    async fn report_error(&self, file_path: &str, error: &str);
    
    /// 処理完了時の報告
    async fn report_completed(&self, total_processed: usize, total_errors: usize);
}

/// 処理結果の永続化抽象化トレイト
#[async_trait]
pub trait HashPersistence: Send + Sync {
    /// 単一ハッシュの保存
    async fn store_hash(
        &self,
        file_path: &str,
        hash: &str,
        metadata: &ProcessingMetadata,
    ) -> Result<()>;
    
    /// バッチでのハッシュ保存
    async fn store_batch(
        &self,
        results: &[(String, String, ProcessingMetadata)],
    ) -> Result<()>;
    
    /// 永続化の完了処理
    async fn finalize(&self) -> Result<()>;
}

/// 並列処理オーケストレーターの抽象化トレイト
#[async_trait]
pub trait ParallelProcessor: Send + Sync {
    type Config: ProcessingConfig;
    type Reporter: ProgressReporter;
    type Persistence: HashPersistence;

    /// ディレクトリの並列処理実行
    async fn process_directory(
        &self,
        path: &str,
        config: &Self::Config,
        reporter: &Self::Reporter,
        persistence: &Self::Persistence,
    ) -> Result<ProcessingSummary>;
}