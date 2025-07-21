// Phase 2: 基本具象実装
// 並列処理システムの基本実装群

use super::{ProcessingConfig, ProgressReporter, HashPersistence, ProcessingMetadata};
use async_trait::async_trait;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// デフォルト設定実装
#[derive(Debug, Clone)]
pub struct DefaultProcessingConfig {
    max_concurrent: usize,
    buffer_size: usize,
    batch_size: usize,
    enable_progress: bool,
}

impl DefaultProcessingConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent = max_concurrent;
        self
    }
    
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
    
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
    
    pub fn with_progress_reporting(mut self, enable: bool) -> Self {
        self.enable_progress = enable;
        self
    }
}

impl Default for DefaultProcessingConfig {
    fn default() -> Self {
        Self {
            max_concurrent: num_cpus::get().max(1) * 2,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        }
    }
}

impl ProcessingConfig for DefaultProcessingConfig {
    fn max_concurrent_tasks(&self) -> usize {
        self.max_concurrent
    }
    
    fn channel_buffer_size(&self) -> usize {
        self.buffer_size
    }
    
    fn batch_size(&self) -> usize {
        self.batch_size
    }
    
    fn enable_progress_reporting(&self) -> bool {
        self.enable_progress
    }
}

/// コンソール出力による進捗報告実装
#[derive(Debug, Default)]
pub struct ConsoleProgressReporter {
    quiet: bool,
}

impl ConsoleProgressReporter {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn quiet() -> Self {
        Self { quiet: true }
    }
}

#[async_trait]
impl ProgressReporter for ConsoleProgressReporter {
    async fn report_started(&self, total_files: usize) {
        if !self.quiet {
            println!("🚀 Starting processing {total_files} files...");
        }
    }
    
    async fn report_progress(&self, completed: usize, total: usize) {
        if !self.quiet && (completed % 100 == 0 || completed == total) {
            let percentage = (completed as f64 / total as f64) * 100.0;
            println!("📊 Progress: {completed}/{total} ({percentage:.1}%)");
        }
    }
    
    async fn report_error(&self, file_path: &str, error: &str) {
        if !self.quiet {
            eprintln!("❌ Error processing {file_path}: {error}");
        }
    }
    
    async fn report_completed(&self, total_processed: usize, total_errors: usize) {
        if !self.quiet {
            println!("✅ Completed! Processed: {total_processed}, Errors: {total_errors}");
        }
    }
}

/// 何もしない進捗報告実装（テスト・ベンチマーク用）
#[derive(Debug, Default)]
pub struct NoOpProgressReporter;

impl NoOpProgressReporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProgressReporter for NoOpProgressReporter {
    async fn report_started(&self, _total_files: usize) {
        // 何もしない
    }
    
    async fn report_progress(&self, _completed: usize, _total: usize) {
        // 何もしない
    }
    
    async fn report_error(&self, _file_path: &str, _error: &str) {
        // 何もしない
    }
    
    async fn report_completed(&self, _total_processed: usize, _total_errors: usize) {
        // 何もしない
    }
}

/// メモリ内保存の永続化実装（テスト用）
#[derive(Debug, Clone)]
pub struct MemoryHashPersistence {
    storage: Arc<Mutex<HashMap<String, (String, ProcessingMetadata)>>>,
    finalized: Arc<Mutex<bool>>,
}

impl Default for MemoryHashPersistence {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryHashPersistence {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            finalized: Arc::new(Mutex::new(false)),
        }
    }
    
    /// テスト用：保存されたデータを取得
    pub fn get_stored_data(&self) -> HashMap<String, (String, ProcessingMetadata)> {
        self.storage.lock().unwrap().clone()
    }
    
    /// テスト用：完了状態を確認
    pub fn is_finalized(&self) -> bool {
        *self.finalized.lock().unwrap()
    }
    
    /// テスト用：データクリア
    pub fn clear(&self) {
        self.storage.lock().unwrap().clear();
        *self.finalized.lock().unwrap() = false;
    }
}

#[async_trait]
impl HashPersistence for MemoryHashPersistence {
    async fn store_hash(
        &self,
        file_path: &str,
        hash: &str,
        metadata: &ProcessingMetadata,
    ) -> Result<()> {
        self.storage
            .lock()
            .unwrap()
            .insert(file_path.to_string(), (hash.to_string(), metadata.clone()));
        Ok(())
    }
    
    async fn store_batch(
        &self,
        results: &[(String, String, ProcessingMetadata)],
    ) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();
        for (path, hash, metadata) in results {
            storage.insert(path.clone(), (hash.clone(), metadata.clone()));
        }
        Ok(())
    }
    
    async fn finalize(&self) -> Result<()> {
        *self.finalized.lock().unwrap() = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_processing_config() {
        let config = DefaultProcessingConfig::default();
        
        assert!(config.max_concurrent_tasks() > 0);
        assert_eq!(config.channel_buffer_size(), 100);
        assert_eq!(config.batch_size(), 50);
        assert!(config.enable_progress_reporting());
    }
    
    #[test]
    fn test_processing_config_builder() {
        let config = DefaultProcessingConfig::new()
            .with_max_concurrent(8)
            .with_buffer_size(200)
            .with_batch_size(100)
            .with_progress_reporting(false);
            
        assert_eq!(config.max_concurrent_tasks(), 8);
        assert_eq!(config.channel_buffer_size(), 200);
        assert_eq!(config.batch_size(), 100);
        assert!(!config.enable_progress_reporting());
    }

    #[tokio::test]
    async fn test_console_progress_reporter() {
        // 出力キャプチャは複雑なため、基本的な呼び出しテストのみ
        let reporter = ConsoleProgressReporter::quiet(); // quiet modeでテスト
        
        reporter.report_started(100).await;
        reporter.report_progress(50, 100).await;
        reporter.report_error("/test.jpg", "test error").await;
        reporter.report_completed(99, 1).await;
        
        // パニックしなければOK
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_console_progress_reporter_creation() {
        let reporter1 = ConsoleProgressReporter::new();
        let reporter2 = ConsoleProgressReporter::quiet();
        
        assert!(!reporter1.quiet);
        assert!(reporter2.quiet);
    }

    #[tokio::test]
    async fn test_noop_progress_reporter() {
        let reporter = NoOpProgressReporter::new();
        
        // 全てのメソッドを呼び出してもパニックしない
        reporter.report_started(100).await;
        reporter.report_progress(50, 100).await;
        reporter.report_error("/test.jpg", "test error").await;
        reporter.report_completed(99, 1).await;
        
        assert!(true);
    }

    #[tokio::test]
    async fn test_memory_hash_persistence() {
        let persistence = MemoryHashPersistence::new();
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // 単一保存テスト
        persistence.store_hash("/test1.jpg", "hash1", &metadata).await.unwrap();
        
        let stored = persistence.get_stored_data();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored["/test1.jpg"].0, "hash1");
        assert_eq!(stored["/test1.jpg"].1, metadata);
        
        // バッチ保存テスト
        let batch = vec![
            ("/test2.jpg".to_string(), "hash2".to_string(), metadata.clone()),
            ("/test3.jpg".to_string(), "hash3".to_string(), metadata.clone()),
        ];
        persistence.store_batch(&batch).await.unwrap();
        
        let stored = persistence.get_stored_data();
        assert_eq!(stored.len(), 3);
        
        // 完了処理テスト
        assert!(!persistence.is_finalized());
        persistence.finalize().await.unwrap();
        assert!(persistence.is_finalized());
    }
    
    #[tokio::test]
    async fn test_memory_persistence_clear() {
        let persistence = MemoryHashPersistence::new();
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        persistence.store_hash("/test.jpg", "hash", &metadata).await.unwrap();
        persistence.finalize().await.unwrap();
        
        assert_eq!(persistence.get_stored_data().len(), 1);
        assert!(persistence.is_finalized());
        
        persistence.clear();
        
        assert_eq!(persistence.get_stored_data().len(), 0);
        assert!(!persistence.is_finalized());
    }
}