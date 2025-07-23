// 並列処理システムのトレイト定義
// 全ての抽象化インターフェースを定義

use super::types::{ProcessingMetadata, ProcessingSummary};
use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use std::path::{Path, PathBuf};

/// 並列処理の設定を抽象化するトレイト
#[automock]
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

// ProcessingConfig for Box<dyn ProcessingConfig>
impl ProcessingConfig for Box<dyn ProcessingConfig> {
    fn max_concurrent_tasks(&self) -> usize {
        self.as_ref().max_concurrent_tasks()
    }

    fn channel_buffer_size(&self) -> usize {
        self.as_ref().channel_buffer_size()
    }

    fn batch_size(&self) -> usize {
        self.as_ref().batch_size()
    }

    fn enable_progress_reporting(&self) -> bool {
        self.as_ref().enable_progress_reporting()
    }
}

/// 進捗報告の抽象化トレイト
#[automock]
#[async_trait]
pub trait ProgressReporter: Send + Sync {
    /// 処理開始時の報告
    async fn report_started(&self, total_files: usize);

    /// 進捗更新の報告
    async fn report_progress(&self, completed: usize, total: usize);

    /// エラー発生時の報告
    async fn report_error(&self, file_path: &Path, error: &str);

    /// 処理完了時の報告
    async fn report_completed(&self, total_processed: usize, total_errors: usize);
}

// ProgressReporter for Box<dyn ProgressReporter>
#[async_trait]
impl ProgressReporter for Box<dyn ProgressReporter> {
    async fn report_started(&self, total_files: usize) {
        self.as_ref().report_started(total_files).await
    }

    async fn report_progress(&self, completed: usize, total: usize) {
        self.as_ref().report_progress(completed, total).await
    }

    async fn report_error(&self, file_path: &Path, error: &str) {
        self.as_ref().report_error(file_path, error).await
    }

    async fn report_completed(&self, total_processed: usize, total_errors: usize) {
        self.as_ref().report_completed(total_processed, total_errors).await
    }
}

/// 処理結果の永続化抽象化トレイト
#[automock]
#[async_trait]
pub trait HashPersistence: Send + Sync {
    /// 単一ハッシュの保存
    async fn store_hash(
        &self,
        file_path: &Path,
        hash: &str,
        metadata: &ProcessingMetadata,
    ) -> Result<()>;

    /// バッチでのハッシュ保存
    async fn store_batch(
        &self,
        results: &[(PathBuf, String, String, u64, ProcessingMetadata)],
    ) -> Result<()>;

    /// スキャン情報の設定
    async fn set_scan_info(&self, operation: String, info: serde_json::Value) -> Result<()>;

    /// 永続化の完了処理
    async fn finalize(&self) -> Result<()>;
}

// HashPersistence for Box<dyn HashPersistence>
#[async_trait]
impl HashPersistence for Box<dyn HashPersistence> {
    async fn store_hash(
        &self,
        file_path: &Path,
        hash: &str,
        metadata: &ProcessingMetadata,
    ) -> Result<()> {
        self.as_ref().store_hash(file_path, hash, metadata).await
    }

    async fn store_batch(
        &self,
        results: &[(PathBuf, String, String, u64, ProcessingMetadata)],
    ) -> Result<()> {
        self.as_ref().store_batch(results).await
    }

    async fn set_scan_info(&self, operation: String, info: serde_json::Value) -> Result<()> {
        self.as_ref().set_scan_info(operation, info).await
    }

    async fn finalize(&self) -> Result<()> {
        self.as_ref().finalize().await
    }
}

/// 並列処理オーケストレーターの抽象化トレイト
#[automock(type Config = MockProcessingConfig; type Reporter = MockProgressReporter; type Persistence = MockHashPersistence;)]
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
