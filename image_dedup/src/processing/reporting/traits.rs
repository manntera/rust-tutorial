// 進捗報告のトレイト定義

/// 進捗報告を抽象化するトレイト
#[async_trait::async_trait]
pub trait ProgressReporter: Send + Sync {
    /// 処理開始時の報告
    async fn report_started(&self, total_files: usize);
    
    /// 進捗状況の報告
    async fn report_progress(&self, completed: usize, total: usize);
    
    /// エラー発生時の報告
    async fn report_error(&self, file_path: &str, error: &str);
    
    /// 処理完了時の報告
    async fn report_completed(&self, total_processed: usize, total_errors: usize);
}