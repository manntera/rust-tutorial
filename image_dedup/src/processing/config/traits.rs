// 設定管理のトレイト定義

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