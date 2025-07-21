# 画像重複検出ツール - 並列処理実装タスク計画書

## 概要

本書は、画像重複検出ツールの並列処理機能を実装するための詳細なタスク計画書です。各タスクは `cargo test` と `cargo clippy` による検証が可能な粒度に設計されており、段階的かつ安全な開発を実現します。

## 開発方針

### 品質保証
- **各タスク完了後**: 必ず `cargo test && cargo clippy` を実行
- **エラーゼロ方針**: 警告・エラーが発生した場合は修正してから次タスクに進む
- **段階的実装**: 小さな単位で機能を実装し、都度動作確認

### 並列開発対応
- **タスク独立性**: 可能な限り各タスクを独立して実装可能に設計
- **マージ競合回避**: ファイル・モジュール単位でタスクを分割
- **テスト駆動**: テストを先に実装して、仕様を明確化

## 実装タスク詳細

### **Phase 1: 基盤トレイト実装**
*推定作業時間: 1日*

#### **Task 1.1: 基本データ構造定義**
**ファイル**: `src/processing/mod.rs`

**実装内容**:
```rust
// 処理時のメタデータ
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessingMetadata {
    pub file_size: u64,
    pub processing_time_ms: u64,
    pub image_dimensions: (u32, u32),
    pub was_resized: bool,
}

// 処理全体のサマリー
#[derive(Debug, PartialEq)]
pub struct ProcessingSummary {
    pub total_files: usize,
    pub processed_files: usize,
    pub error_count: usize,
    pub total_processing_time_ms: u64,
    pub average_time_per_file_ms: f64,
}

// 個別処理の結果
#[derive(Debug)]
pub enum ProcessingResult {
    Success {
        file_path: String,
        hash: String,
        metadata: ProcessingMetadata,
    },
    Error {
        file_path: String,
        error: String,
    },
}
```

**テスト内容**:
- 各構造体の作成とフィールドアクセス
- `ProcessingResult` の各バリアント作成
- `Debug` トレイトの動作確認

**成功基準**: 
- `cargo test` で新規テスト全てパス
- `cargo clippy` で警告なし

---

#### **Task 1.2: ProcessingConfig トレイト定義**
**ファイル**: `src/processing/mod.rs`

**実装内容**:
```rust
/// 並列処理の設定を抽象化
pub trait ProcessingConfig: Send + Sync {
    /// 同時実行タスクの最大数
    fn max_concurrent_tasks(&self) -> usize;
    
    /// チャンネルバッファサイズ
    fn channel_buffer_size(&self) -> usize;
    
    /// バッチ処理のサイズ
    fn batch_size(&self) -> usize;
    
    /// 進捗報告の有効/無効
    fn enable_progress_reporting(&self) -> bool;
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockConfig {
        max_concurrent: usize,
        buffer_size: usize,
        batch_size: usize,
        enable_progress: bool,
    }
    
    impl ProcessingConfig for MockConfig {
        fn max_concurrent_tasks(&self) -> usize { self.max_concurrent }
        fn channel_buffer_size(&self) -> usize { self.buffer_size }
        fn batch_size(&self) -> usize { self.batch_size }
        fn enable_progress_reporting(&self) -> bool { self.enable_progress }
    }
    
    #[test]
    fn test_processing_config_implementation() {
        let config = MockConfig {
            max_concurrent: 4,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        };
        
        assert_eq!(config.max_concurrent_tasks(), 4);
        assert_eq!(config.channel_buffer_size(), 100);
        assert_eq!(config.batch_size(), 50);
        assert!(config.enable_progress_reporting());
    }
}
```

**成功基準**:
- トレイトコンパイル成功
- Mockテスト実装とパス

---

#### **Task 1.3: ProgressReporter トレイト定義**
**ファイル**: `src/processing/mod.rs`

**実装内容**:
```rust
use async_trait::async_trait;

/// 進捗報告の抽象化
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
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockReporter {
        calls: Arc<Mutex<Vec<String>>>,
    }
    
    #[async_trait]
    impl ProgressReporter for MockReporter {
        async fn report_started(&self, total_files: usize) {
            self.calls.lock().unwrap().push(format!("started:{}", total_files));
        }
        
        async fn report_progress(&self, completed: usize, total: usize) {
            self.calls.lock().unwrap().push(format!("progress:{}:{}", completed, total));
        }
        
        async fn report_error(&self, file_path: &str, error: &str) {
            self.calls.lock().unwrap().push(format!("error:{}:{}", file_path, error));
        }
        
        async fn report_completed(&self, total_processed: usize, total_errors: usize) {
            self.calls.lock().unwrap().push(format!("completed:{}:{}", total_processed, total_errors));
        }
    }
    
    #[tokio::test]
    async fn test_progress_reporter() {
        let reporter = MockReporter::default();
        
        reporter.report_started(100).await;
        reporter.report_progress(50, 100).await;
        reporter.report_error("/path/test.jpg", "load failed").await;
        reporter.report_completed(99, 1).await;
        
        let calls = reporter.calls.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0], "started:100");
        assert_eq!(calls[1], "progress:50:100");
        assert_eq!(calls[2], "error:/path/test.jpg:load failed");
        assert_eq!(calls[3], "completed:99:1");
    }
}
```

**成功基準**:
- 非同期トレイトのコンパイル成功
- Mock実装でのテストパス

---

#### **Task 1.4: HashPersistence トレイト定義**
**ファイル**: `src/processing/mod.rs`

**実装内容**:
```rust
use anyhow::Result;

/// 処理結果の永続化抽象化
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
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockPersistence {
        storage: Arc<Mutex<HashMap<String, (String, ProcessingMetadata)>>>,
        finalized: Arc<Mutex<bool>>,
    }
    
    #[async_trait]
    impl HashPersistence for MockPersistence {
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
    
    #[tokio::test]
    async fn test_hash_persistence() {
        let persistence = MockPersistence::default();
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // 単一保存テスト
        persistence.store_hash("/test1.jpg", "hash1", &metadata).await.unwrap();
        
        // バッチ保存テスト
        let batch = vec![
            ("/test2.jpg".to_string(), "hash2".to_string(), metadata.clone()),
            ("/test3.jpg".to_string(), "hash3".to_string(), metadata.clone()),
        ];
        persistence.store_batch(&batch).await.unwrap();
        
        // 完了処理テスト
        persistence.finalize().await.unwrap();
        
        let storage = persistence.storage.lock().unwrap();
        assert_eq!(storage.len(), 3);
        assert!(storage.contains_key("/test1.jpg"));
        assert!(storage.contains_key("/test2.jpg"));
        assert!(storage.contains_key("/test3.jpg"));
        
        assert!(*persistence.finalized.lock().unwrap());
    }
}
```

**成功基準**:
- トレイトコンパイル成功
- Mock実装でのCRUDテストパス

---

#### **Task 1.5: ParallelProcessor トレイト定義**
**ファイル**: `src/processing/mod.rs`

**実装内容**:
```rust
/// 並列処理オーケストレーターの抽象化
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
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // コンパイル確認用のダミー実装
    struct DummyProcessor;
    
    #[async_trait]
    impl ParallelProcessor for DummyProcessor {
        type Config = MockConfig;
        type Reporter = MockReporter;
        type Persistence = MockPersistence;
        
        async fn process_directory(
            &self,
            _path: &str,
            _config: &Self::Config,
            _reporter: &Self::Reporter,
            _persistence: &Self::Persistence,
        ) -> Result<ProcessingSummary> {
            Ok(ProcessingSummary {
                total_files: 0,
                processed_files: 0,
                error_count: 0,
                total_processing_time_ms: 0,
                average_time_per_file_ms: 0.0,
            })
        }
    }
    
    #[tokio::test]
    async fn test_parallel_processor_trait() {
        let processor = DummyProcessor;
        let config = MockConfig {
            max_concurrent: 4,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        };
        let reporter = MockReporter::default();
        let persistence = MockPersistence::default();
        
        let result = processor
            .process_directory("/test", &config, &reporter, &persistence)
            .await
            .unwrap();
            
        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
    }
}
```

**成功基準**:
- 関連型を使ったトレイトのコンパイル成功
- ダミー実装での基本テストパス

---

### **Phase 2: 基本具象実装**
*推定作業時間: 1日*

#### **Task 2.1: DefaultProcessingConfig実装**
**ファイル**: `src/processing/implementations.rs`

**実装内容**:
```rust
use super::ProcessingConfig;

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
```

**テスト内容**:
```rust
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
}
```

**成功基準**:
- `num_cpus` クレートの依存関係追加
- 設定の取得・変更テストパス

---

#### **Task 2.2: ConsoleProgressReporter実装**
**ファイル**: `src/processing/implementations.rs`

**実装内容**:
```rust
use super::ProgressReporter;
use async_trait::async_trait;

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
            println!("🚀 Starting processing {} files...", total_files);
        }
    }
    
    async fn report_progress(&self, completed: usize, total: usize) {
        if !self.quiet && (completed % 100 == 0 || completed == total) {
            let percentage = (completed as f64 / total as f64) * 100.0;
            println!("📊 Progress: {}/{} ({:.1}%)", completed, total, percentage);
        }
    }
    
    async fn report_error(&self, file_path: &str, error: &str) {
        if !self.quiet {
            eprintln!("❌ Error processing {}: {}", file_path, error);
        }
    }
    
    async fn report_completed(&self, total_processed: usize, total_errors: usize) {
        if !self.quiet {
            println!("✅ Completed! Processed: {}, Errors: {}", total_processed, total_errors);
        }
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

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
}
```

**成功基準**:
- コンソール出力実装の完了
- 基本的な呼び出しテストパス

---

#### **Task 2.3: NoOpProgressReporter実装（テスト用）**
**ファイル**: `src/processing/implementations.rs`

**実装内容**:
```rust
/// 何もしない進捗報告実装（テスト・ベンチマーク用）
#[derive(Debug, Default)]
pub struct NoOpProgressReporter;

impl NoOpProgressReporter {
    pub fn new() -> Self {
        Self::default()
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
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

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
}
```

**成功基準**:
- 空実装の完了
- 基本呼び出しテストパス

---

#### **Task 2.4: MemoryHashPersistence実装（テスト用）**
**ファイル**: `src/processing/implementations.rs`

**実装内容**:
```rust
use super::{HashPersistence, ProcessingMetadata};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

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
```

**成功基準**:
- メモリ内データ保存の実装完了
- CRUD操作のテストパス

---

### **Phase 3: コア処理エンジン実装**
*推定作業時間: 0.5日*

#### **Task 3.1: ParallelProcessingEngine構造体実装**
**ファイル**: `src/processing/engine.rs`

**実装内容**:
```rust
use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
};
use super::{ParallelProcessor, ProcessingSummary};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 依存性注入によるコア処理エンジン
pub struct ParallelProcessingEngine<L, H, S> {
    loader: Arc<L>,
    hasher: Arc<H>,
    storage: Arc<S>,
}

impl<L, H, S> ParallelProcessingEngine<L, H, S>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    /// コンストラクタインジェクション
    pub fn new(loader: L, hasher: H, storage: S) -> Self {
        Self {
            loader: Arc::new(loader),
            hasher: Arc::new(hasher),
            storage: Arc::new(storage),
        }
    }

    /// ファクトリーメソッド（既存のAppから構築）
    pub fn from_app(app: crate::App<L, H, S>) -> Self {
        Self::new(app.loader, app.hasher, app.storage)
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::standard::StandardImageLoader;
    use crate::perceptual_hash::dct_hash::DCTHasher;
    use crate::storage::local::LocalStorageBackend;

    #[test]
    fn test_parallel_processing_engine_new() {
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        let storage = LocalStorageBackend::new();
        
        let engine = ParallelProcessingEngine::new(loader, hasher, storage);
        
        // コンパイルが通れば成功
        assert!(true);
    }
    
    #[test]
    fn test_parallel_processing_engine_from_app() {
        let app = crate::App::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let engine = ParallelProcessingEngine::from_app(app);
        
        // コンパイルが通れば成功
        assert!(true);
    }
}
```

**成功基準**:
- 構造体定義とコンストラクタの実装完了
- 基本的なインスタンス作成テストパス

---

#### **Task 3.2: ファイル発見機能実装**
**ファイル**: `src/processing/engine.rs`

**実装内容**:
```rust
impl<L, H, S> ParallelProcessingEngine<L, H, S>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    /// ディレクトリから画像ファイルを発見
    async fn discover_image_files(&self, path: &str) -> Result<Vec<String>> {
        let items = self.storage.list_items(path).await?;
        
        let mut image_files = Vec::new();
        for item in items {
            if !item.is_directory && self.storage.is_image_file(&item) {
                image_files.push(item.id);
            }
        }
        
        image_files.sort(); // 一貫した順序で処理
        Ok(image_files)
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_discover_image_files() {
        // テスト用ディレクトリ作成
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        // テスト用ファイル作成
        fs::write(temp_dir.path().join("test1.jpg"), b"fake jpg content").unwrap();
        fs::write(temp_dir.path().join("test2.png"), b"fake png content").unwrap();
        fs::write(temp_dir.path().join("not_image.txt"), b"text content").unwrap();
        
        // エンジン作成
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        // ファイル発見実行
        let files = engine.discover_image_files(temp_path).await.unwrap();
        
        // 画像ファイルのみが発見されることを確認
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("test1.jpg")));
        assert!(files.iter().any(|f| f.ends_with("test2.png")));
        assert!(!files.iter().any(|f| f.ends_with("not_image.txt")));
    }
    
    #[tokio::test]
    async fn test_discover_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let files = engine.discover_image_files(temp_path).await.unwrap();
        assert_eq!(files.len(), 0);
    }
}
```

**依存関係追加**: `tempfile = "3.8"` を `[dev-dependencies]` に追加

**成功基準**:
- ファイル発見ロジックの実装完了
- テスト用ディレクトリでの発見テストパス

---

#### **Task 3.3: 単一ファイル処理機能実装**
**ファイル**: `src/processing/engine.rs`

**実装内容**:
```rust
use super::{ProcessingMetadata, ProcessingResult};
use std::time::Instant;

impl<L, H, S> ParallelProcessingEngine<L, H, S>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    /// 単一ファイルの処理
    async fn process_single_file(
        loader: &L,
        hasher: &H,
        file_path: &str,
        _worker_id: usize,
    ) -> ProcessingResult {
        let start_time = Instant::now();
        
        let result = async {
            // 画像読み込み
            let load_result = loader.load_from_path(file_path).await?;
            
            // ハッシュ生成
            let hash_result = hasher.generate_hash(&load_result.image).await?;
            
            // メタデータ作成
            let metadata = ProcessingMetadata {
                file_size: load_result.file_size,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                image_dimensions: (load_result.image.width(), load_result.image.height()),
                was_resized: load_result.was_resized,
            };
            
            Result::<(String, ProcessingMetadata)>::Ok((hash_result.to_hex(), metadata))
        }.await;
        
        match result {
            Ok((hash, metadata)) => ProcessingResult::Success {
                file_path: file_path.to_string(),
                hash,
                metadata,
            },
            Err(error) => ProcessingResult::Error {
                file_path: file_path.to_string(),
                error: error.to_string(),
            },
        }
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_process_single_file_success() {
        // 1x1の最小PNGファイル（有効な画像データ）
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.png");
        fs::write(&test_file, &png_data).unwrap();
        
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        
        let result = ParallelProcessingEngine::process_single_file(
            &loader,
            &hasher,
            test_file.to_str().unwrap(),
            0,
        ).await;
        
        match result {
            ProcessingResult::Success { file_path, hash, metadata } => {
                assert!(file_path.ends_with("test.png"));
                assert!(!hash.is_empty());
                assert_eq!(metadata.image_dimensions, (1, 1));
                assert!(metadata.processing_time_ms > 0);
            }
            ProcessingResult::Error { .. } => panic!("Expected success"),
        }
    }
    
    #[tokio::test]
    async fn test_process_single_file_error() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = temp_dir.path().join("invalid.jpg");
        fs::write(&invalid_file, b"not a valid image").unwrap();
        
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        
        let result = ParallelProcessingEngine::process_single_file(
            &loader,
            &hasher,
            invalid_file.to_str().unwrap(),
            0,
        ).await;
        
        match result {
            ProcessingResult::Success { .. } => panic!("Expected error"),
            ProcessingResult::Error { file_path, error } => {
                assert!(file_path.ends_with("invalid.jpg"));
                assert!(!error.is_empty());
            }
        }
    }
    
    #[tokio::test]
    async fn test_process_nonexistent_file() {
        let loader = StandardImageLoader::new();
        let hasher = DCTHasher::new(8);
        
        let result = ParallelProcessingEngine::process_single_file(
            &loader,
            &hasher,
            "/nonexistent/file.jpg",
            0,
        ).await;
        
        match result {
            ProcessingResult::Success { .. } => panic!("Expected error"),
            ProcessingResult::Error { file_path, error } => {
                assert_eq!(file_path, "/nonexistent/file.jpg");
                assert!(!error.is_empty());
            }
        }
    }
}
```

**成功基準**:
- 単一ファイル処理ロジックの実装完了
- 成功・エラーケースのテストパス
- メタデータ収集の確認

---

---

### **Phase 4: Producer-Consumer パイプライン実装**
*推定作業時間: 1.5日*

#### **Task 4.1: ProcessingPipeline構造体実装**
**ファイル**: `src/processing/pipeline.rs`

**実装内容**:
```rust
use crate::{
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
};
use super::{ProcessingConfig, ProgressReporter, HashPersistence, ProcessingResult, ProcessingSummary};
use anyhow::Result;
use tokio::sync::mpsc;
use std::sync::Arc;

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
        reporter: &R,
        persistence: &P,
    ) -> Result<ProcessingSummary>
    where
        C: ProcessingConfig,
        R: ProgressReporter,
        P: HashPersistence,
    {
        // Producer-Consumerチャンネル構築
        let (work_tx, work_rx) = mpsc::channel::<String>(config.channel_buffer_size());
        let (result_tx, result_rx) = mpsc::channel::<ProcessingResult>(config.channel_buffer_size());
        
        // 同期プリミティブ
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrent_tasks()));
        let processed_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let error_count = Arc::new(tokio::sync::RwLock::new(0usize));
        
        // プレースホルダー実装（後続タスクで実装）
        Ok(ProcessingSummary {
            total_files: files.len(),
            processed_files: 0,
            error_count: 0,
            total_processing_time_ms: 0,
            average_time_per_file_ms: 0.0,
        })
    }
}
```

**テスト内容**:
```rust
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
        
        let pipeline = ProcessingPipeline::new(loader, hasher);
        
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
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }
}
```

**成功基準**:
- パイプライン基本構造の実装完了
- 空ファイルリストでの実行テストパス

---

#### **Task 4.2: Producer実装**
**ファイル**: `src/processing/pipeline.rs`

**実装内容**:
```rust
impl<L, H> ProcessingPipeline<L, H>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    /// Producer: ファイルパスを配信
    fn spawn_producer(
        files: Vec<String>,
        work_tx: mpsc::Sender<String>,
    ) -> tokio::task::JoinHandle<Result<()>> {
        tokio::spawn(async move {
            for file_path in files {
                if let Err(_) = work_tx.send(file_path).await {
                    // チャンネルが閉じられた場合は正常終了
                    break;
                }
            }
            // work_txをドロップしてチャンネル終了シグナル
            Ok(())
        })
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
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
        let producer_handle = ProcessingPipeline::<StandardImageLoader, DCTHasher>::spawn_producer(
            files.clone(),
            work_tx,
        );
        
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
        
        let producer_handle = ProcessingPipeline::<StandardImageLoader, DCTHasher>::spawn_producer(
            files,
            work_tx,
        );
        
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
        
        let producer_handle = ProcessingPipeline::<StandardImageLoader, DCTHasher>::spawn_producer(
            files,
            work_tx,
        );
        
        // Producerはエラーなく終了すべき
        producer_handle.await.unwrap().unwrap();
    }
}
```

**成功基準**:
- ファイル配信ロジックの実装完了
- 正常ケース・異常ケースのテストパス

---

#### **Task 4.3: 単一Consumer実装**
**ファイル**: `src/processing/pipeline.rs`

**実装内容**:
```rust
impl<L, H> ProcessingPipeline<L, H>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    /// 単一Consumerワーカー
    fn spawn_single_consumer(
        worker_id: usize,
        loader: Arc<L>,
        hasher: Arc<H>,
        work_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<String>>>,
        result_tx: mpsc::Sender<ProcessingResult>,
        semaphore: Arc<tokio::sync::Semaphore>,
    ) -> tokio::task::JoinHandle<Result<()>> {
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
                let result = Self::process_single_file(
                    &loader,
                    &hasher,
                    &file_path,
                    worker_id,
                ).await;
                
                // 結果送信
                if let Err(_) = result_tx.send(result).await {
                    // 結果チャンネルが閉じられた場合は終了
                    break;
                }
            }
            Ok(())
        })
    }

    /// 単一ファイルの処理（engine.rsから移動）
    async fn process_single_file(
        loader: &L,
        hasher: &H,
        file_path: &str,
        _worker_id: usize,
    ) -> ProcessingResult {
        use std::time::Instant;
        use super::ProcessingMetadata;
        
        let start_time = Instant::now();
        
        let result = async {
            let load_result = loader.load_from_path(file_path).await?;
            let hash_result = hasher.generate_hash(&load_result.image).await?;
            
            let metadata = ProcessingMetadata {
                file_size: load_result.file_size,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                image_dimensions: (load_result.image.width(), load_result.image.height()),
                was_resized: load_result.was_resized,
            };
            
            Result::<(String, ProcessingMetadata)>::Ok((hash_result.to_hex(), metadata))
        }.await;
        
        match result {
            Ok((hash, metadata)) => ProcessingResult::Success {
                file_path: file_path.to_string(),
                hash,
                metadata,
            },
            Err(error) => ProcessingResult::Error {
                file_path: file_path.to_string(),
                error: error.to_string(),
            },
        }
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_single_consumer_processes_files() {
        // テスト用画像作成
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.png");
        fs::write(&test_file, &png_data).unwrap();
        
        // チャンネル作成
        let (work_tx, work_rx) = mpsc::channel::<String>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<ProcessingResult>(10);
        let work_rx = Arc::new(tokio::sync::Mutex::new(work_rx));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        
        // ワーカー起動
        let worker_handle = ProcessingPipeline::spawn_single_consumer(
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
        
        let worker_handle = ProcessingPipeline::spawn_single_consumer(
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
}
```

**成功基準**:
- 単一ワーカーロジックの実装完了
- 成功・エラー処理のテストパス
- セマフォによる制御の確認

---

#### **Task 4.4: Consumer Pool実装**
**ファイル**: `src/processing/pipeline.rs`

**実装内容**:
```rust
impl<L, H> ProcessingPipeline<L, H>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    /// Consumers: 並列ワーカープール
    fn spawn_consumers(
        &self,
        work_rx: mpsc::Receiver<String>,
        result_tx: mpsc::Sender<ProcessingResult>,
        semaphore: Arc<tokio::sync::Semaphore>,
        worker_count: usize,
    ) -> Vec<tokio::task::JoinHandle<Result<()>>> {
        let work_rx = Arc::new(tokio::sync::Mutex::new(work_rx));
        let mut handles = Vec::new();
        
        for worker_id in 0..worker_count {
            let handle = Self::spawn_single_consumer(
                worker_id,
                self.loader.clone(),
                self.hasher.clone(),
                work_rx.clone(),
                result_tx.clone(),
                semaphore.clone(),
            );
            handles.push(handle);
        }
        
        handles
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_consumer_pool_processes_multiple_files() {
        // 複数のテスト用画像作成
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        let mut test_files = Vec::new();
        
        for i in 0..5 {
            let test_file = temp_dir.path().join(format!("test{}.png", i));
            fs::write(&test_file, &png_data).unwrap();
            test_files.push(test_file.to_str().unwrap().to_string());
        }
        
        // パイプライン作成
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
        );
        
        // チャンネル作成
        let (work_tx, work_rx) = mpsc::channel::<String>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<ProcessingResult>(10);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(3));
        
        // Consumer pool起動
        let worker_handles = pipeline.spawn_consumers(
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
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let valid_file = temp_dir.path().join("valid.png");
        fs::write(&valid_file, &png_data).unwrap();
        
        let invalid_file = temp_dir.path().join("invalid.jpg");
        fs::write(&invalid_file, b"not a valid image").unwrap();
        
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
        );
        
        let (work_tx, work_rx) = mpsc::channel::<String>(10);
        let (result_tx, mut result_rx) = mpsc::channel::<ProcessingResult>(10);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(2));
        
        let worker_handles = pipeline.spawn_consumers(work_rx, result_tx, semaphore, 2);
        
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
```

**成功基準**:
- 複数ワーカーによる並列処理の実装完了
- 並列処理でのファイル処理テストパス
- 成功・失敗混在ケースのテストパス

---

#### **Task 4.5: Result Collector実装**
**ファイル**: `src/processing/pipeline.rs`

**実装内容**:
```rust
impl<L, H> ProcessingPipeline<L, H>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    /// Collector: 結果収集と永続化
    fn spawn_result_collector<R, P>(
        mut result_rx: mpsc::Receiver<ProcessingResult>,
        total_files: usize,
        processed_count: Arc<tokio::sync::RwLock<usize>>,
        error_count: Arc<tokio::sync::RwLock<usize>>,
        reporter: &R,
        persistence: &P,
        batch_size: usize,
    ) -> tokio::task::JoinHandle<Result<()>>
    where
        R: ProgressReporter + 'static,
        P: HashPersistence + 'static,
    {
        let reporter = Arc::new(reporter);
        let persistence = Arc::new(persistence);
        
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            let mut completed = 0;
            let mut errors = 0;
            
            while let Some(result) = result_rx.recv().await {
                match result {
                    ProcessingResult::Success { file_path, hash, metadata } => {
                        batch.push((file_path, hash, metadata));
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
                reporter.report_progress(completed + errors, total_files).await;
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
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{ProcessingMetadata, MemoryHashPersistence, NoOpProgressReporter};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_result_collector_processes_success_results() {
        let (result_tx, result_rx) = mpsc::channel::<ProcessingResult>(10);
        let processed_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let error_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let collector_handle = ProcessingPipeline::<StandardImageLoader, DCTHasher>::spawn_result_collector(
            result_rx,
            3,
            processed_count.clone(),
            error_count.clone(),
            &reporter,
            &persistence,
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
            
            result_tx.send(ProcessingResult::Success {
                file_path: format!("/test{}.jpg", i),
                hash: format!("hash{}", i),
                metadata,
            }).await.unwrap();
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
        
        let collector_handle = ProcessingPipeline::<StandardImageLoader, DCTHasher>::spawn_result_collector(
            result_rx,
            4,
            processed_count.clone(),
            error_count.clone(),
            &reporter,
            &persistence,
            10, // 大きなバッチサイズ
        );
        
        // 成功結果
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        result_tx.send(ProcessingResult::Success {
            file_path: "/success1.jpg".to_string(),
            hash: "hash1".to_string(),
            metadata: metadata.clone(),
        }).await.unwrap();
        
        result_tx.send(ProcessingResult::Success {
            file_path: "/success2.jpg".to_string(),
            hash: "hash2".to_string(),
            metadata,
        }).await.unwrap();
        
        // エラー結果
        result_tx.send(ProcessingResult::Error {
            file_path: "/error1.jpg".to_string(),
            error: "load failed".to_string(),
        }).await.unwrap();
        
        result_tx.send(ProcessingResult::Error {
            file_path: "/error2.jpg".to_string(),
            error: "invalid format".to_string(),
        }).await.unwrap();
        
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
        
        let collector_handle = ProcessingPipeline::<StandardImageLoader, DCTHasher>::spawn_result_collector(
            result_rx,
            5,
            processed_count.clone(),
            error_count.clone(),
            &reporter,
            &persistence,
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
            
            result_tx.send(ProcessingResult::Success {
                file_path: format!("/test{}.jpg", i),
                hash: format!("hash{}", i),
                metadata,
            }).await.unwrap();
        }
        
        drop(result_tx);
        collector_handle.await.unwrap().unwrap();
        
        assert_eq!(*processed_count.read().await, 5);
        assert_eq!(*error_count.read().await, 0);
        
        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 5);
    }
}
```

**成功基準**:
- 結果収集と永続化ロジックの実装完了
- バッチ処理のテストパス
- 成功・失敗混在処理のテストパス

---

#### **Task 4.6: パイプライン統合実装**
**ファイル**: `src/processing/pipeline.rs`

**実装内容**:
```rust
impl<L, H> ProcessingPipeline<L, H>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
{
    /// 完全なパイプライン実行
    pub async fn execute<C, R, P>(
        &self,
        files: Vec<String>,
        config: &C,
        reporter: &R,
        persistence: &P,
    ) -> Result<ProcessingSummary>
    where
        C: ProcessingConfig,
        R: ProgressReporter,
        P: HashPersistence,
    {
        let total_files = files.len();
        
        if total_files == 0 {
            return Ok(ProcessingSummary {
                total_files: 0,
                processed_files: 0,
                error_count: 0,
                total_processing_time_ms: 0,
                average_time_per_file_ms: 0.0,
            });
        }
        
        // Producer-Consumerチャンネル構築
        let (work_tx, work_rx) = mpsc::channel::<String>(config.channel_buffer_size());
        let (result_tx, result_rx) = mpsc::channel::<ProcessingResult>(config.channel_buffer_size());
        
        // 同期プリミティブ
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrent_tasks()));
        let processed_count = Arc::new(tokio::sync::RwLock::new(0usize));
        let error_count = Arc::new(tokio::sync::RwLock::new(0usize));
        
        // 3つの独立したコンポーネント起動
        let producer_handle = Self::spawn_producer(files, work_tx);
        
        let consumer_handles = self.spawn_consumers(
            work_rx,
            result_tx,
            semaphore,
            config.max_concurrent_tasks(),
        );
        
        let collector_handle = Self::spawn_result_collector(
            result_rx,
            total_files,
            processed_count.clone(),
            error_count.clone(),
            reporter,
            persistence,
            config.batch_size(),
        );
        
        // 全コンポーネントの完了待機
        producer_handle.await??;
        
        // 全Consumerの完了待機
        for handle in consumer_handles {
            handle.await??;
        }
        
        // Result Collectorの完了待機
        collector_handle.await??;
        
        // 結果サマリー構築
        let processed = *processed_count.read().await;
        let errors = *error_count.read().await;
        
        Ok(ProcessingSummary {
            total_files,
            processed_files: processed,
            error_count: errors,
            total_processing_time_ms: 0, // 呼び出し元で計算
            average_time_per_file_ms: 0.0,
        })
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_pipeline_end_to_end() {
        // 複数のテスト用画像作成
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        let mut test_files = Vec::new();
        
        // 有効な画像ファイル作成
        for i in 0..3 {
            let test_file = temp_dir.path().join(format!("valid{}.png", i));
            fs::write(&test_file, &png_data).unwrap();
            test_files.push(test_file.to_str().unwrap().to_string());
        }
        
        // 無効なファイル作成
        let invalid_file = temp_dir.path().join("invalid.jpg");
        fs::write(&invalid_file, b"not a valid image").unwrap();
        test_files.push(invalid_file.to_str().unwrap().to_string());
        
        // パイプライン実行
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
        );
        
        let config = DefaultProcessingConfig::default()
            .with_max_concurrent(2)
            .with_batch_size(2);
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let summary = pipeline.execute(
            test_files,
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        // 結果確認
        assert_eq!(summary.total_files, 4);
        assert_eq!(summary.processed_files, 3); // 有効なファイル3つ
        assert_eq!(summary.error_count, 1); // 無効なファイル1つ
        
        // 永続化確認
        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 3);
        
        for i in 0..3 {
            assert!(stored_data.iter().any(|(path, _)| path.contains(&format!("valid{}.png", i))));
        }
    }
    
    #[tokio::test]
    async fn test_pipeline_empty_files() {
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
        );
        
        let config = DefaultProcessingConfig::default();
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let summary = pipeline.execute(
            vec![],
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(summary.total_files, 0);
        assert_eq!(summary.processed_files, 0);
        assert_eq!(summary.error_count, 0);
    }
    
    #[tokio::test]
    async fn test_pipeline_with_high_concurrency() {
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        let mut test_files = Vec::new();
        
        // 10個のファイル作成
        for i in 0..10 {
            let test_file = temp_dir.path().join(format!("test{}.png", i));
            fs::write(&test_file, &png_data).unwrap();
            test_files.push(test_file.to_str().unwrap().to_string());
        }
        
        let pipeline = ProcessingPipeline::new(
            Arc::new(StandardImageLoader::new()),
            Arc::new(DCTHasher::new(8)),
        );
        
        let config = DefaultProcessingConfig::default()
            .with_max_concurrent(8) // 高い並列度
            .with_batch_size(3);
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let summary = pipeline.execute(
            test_files,
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(summary.total_files, 10);
        assert_eq!(summary.processed_files, 10);
        assert_eq!(summary.error_count, 0);
        
        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 10);
    }
}
```

**成功基準**:
- 完全なパイプライン統合の実装完了
- エンドツーエンドテストパス
- 高並列度でのテストパス

---

### **Phase 5: エンジン統合とParallelProcessor実装**
*推定作業時間: 1日*

#### **Task 5.1: ParallelProcessor trait実装**
**ファイル**: `src/processing/engine.rs`

**実装内容**:
```rust
use super::{ParallelProcessor, DefaultProcessingConfig, NoOpProgressReporter, MemoryHashPersistence};
use super::pipeline::ProcessingPipeline;
use std::time::Instant;

#[async_trait]
impl<L, H, S> ParallelProcessor for ParallelProcessingEngine<L, H, S>
where
    L: ImageLoaderBackend + 'static,
    H: PerceptualHashBackend + 'static,
    S: StorageBackend + 'static,
{
    type Config = dyn ProcessingConfig;
    type Reporter = dyn ProgressReporter;
    type Persistence = dyn HashPersistence;

    async fn process_directory(
        &self,
        path: &str,
        config: &Self::Config,
        reporter: &Self::Reporter,
        persistence: &Self::Persistence,
    ) -> Result<ProcessingSummary> {
        let start_time = Instant::now();
        
        // 1. ファイル発見フェーズ
        let files = self.discover_image_files(path).await?;
        let total_files = files.len();
        
        if config.enable_progress_reporting() {
            reporter.report_started(total_files).await;
        }
        
        // 2. Producer-Consumerパイプライン構築
        let pipeline = ProcessingPipeline::new(
            self.loader.clone(),
            self.hasher.clone(),
        );
        
        // 3. 並列処理実行
        let mut summary = pipeline.execute(files, config, reporter, persistence).await?;
        
        // 4. タイミング計測完了
        let total_time = start_time.elapsed().as_millis() as u64;
        summary.total_processing_time_ms = total_time;
        
        if summary.processed_files > 0 {
            summary.average_time_per_file_ms = total_time as f64 / summary.processed_files as f64;
        }
        
        if config.enable_progress_reporting() {
            reporter.report_completed(summary.processed_files, summary.error_count).await;
        }
        
        // 5. 永続化完了処理
        persistence.finalize().await?;
        
        Ok(summary)
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_parallel_processor_trait_implementation() {
        // テスト用ディレクトリと画像作成
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        
        // 画像ファイル作成
        for i in 0..3 {
            let test_file = temp_dir.path().join(format!("test{}.png", i));
            fs::write(&test_file, &png_data).unwrap();
        }
        
        // 非画像ファイル作成（無視されるはず）
        fs::write(temp_dir.path().join("readme.txt"), b"text content").unwrap();
        
        // エンジン作成
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        // 設定とレポーター作成
        let config = DefaultProcessingConfig::default().with_max_concurrent(2);
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        // 処理実行
        let summary = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        // 結果確認
        assert_eq!(summary.total_files, 3); // 画像ファイルのみ
        assert_eq!(summary.processed_files, 3);
        assert_eq!(summary.error_count, 0);
        assert!(summary.total_processing_time_ms > 0);
        assert!(summary.average_time_per_file_ms > 0.0);
        
        // 永続化確認
        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 3);
        assert!(persistence.is_finalized());
        
        for i in 0..3 {
            assert!(stored_data.iter().any(|(path, _)| path.contains(&format!("test{}.png", i))));
        }
    }
    
    #[tokio::test]
    async fn test_process_directory_empty() {
        let temp_dir = TempDir::new().unwrap();
        
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let config = DefaultProcessingConfig::default();
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let summary = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(summary.total_files, 0);
        assert_eq!(summary.processed_files, 0);
        assert_eq!(summary.error_count, 0);
        assert!(persistence.is_finalized());
    }
    
    #[tokio::test]
    async fn test_process_directory_with_errors() {
        let temp_dir = TempDir::new().unwrap();
        
        // 有効な画像
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        fs::write(temp_dir.path().join("valid.png"), &png_data).unwrap();
        fs::write(temp_dir.path().join("invalid.jpg"), b"not a valid image").unwrap();
        
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let config = DefaultProcessingConfig::default();
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let summary = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(summary.total_files, 2);
        assert_eq!(summary.processed_files, 1); // 有効な画像のみ
        assert_eq!(summary.error_count, 1); // 無効な画像
        
        let stored_data = persistence.get_stored_data();
        assert_eq!(stored_data.len(), 1);
        assert!(stored_data.contains_key(&format!("{}/valid.png", temp_dir.path().to_str().unwrap())));
    }
    
    #[tokio::test]
    async fn test_process_directory_performance_metrics() {
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        
        let temp_dir = TempDir::new().unwrap();
        
        // 5つの画像ファイル作成
        for i in 0..5 {
            let test_file = temp_dir.path().join(format!("test{}.png", i));
            fs::write(&test_file, &png_data).unwrap();
        }
        
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let config = DefaultProcessingConfig::default().with_max_concurrent(4);
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let start_time = std::time::Instant::now();
        
        let summary = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        let actual_elapsed = start_time.elapsed().as_millis() as u64;
        
        // パフォーマンスメトリクス確認
        assert_eq!(summary.processed_files, 5);
        assert!(summary.total_processing_time_ms > 0);
        assert!(summary.average_time_per_file_ms > 0.0);
        
        // 実際の処理時間との差が大きくないことを確認（誤差範囲）
        let time_diff = if actual_elapsed > summary.total_processing_time_ms {
            actual_elapsed - summary.total_processing_time_ms
        } else {
            summary.total_processing_time_ms - actual_elapsed
        };
        assert!(time_diff < 1000); // 1秒以内の誤差
        
        // 平均時間の整合性確認
        let expected_avg = summary.total_processing_time_ms as f64 / 5.0;
        assert!((summary.average_time_per_file_ms - expected_avg).abs() < 1.0);
    }
}
```

**成功基準**:
- `ParallelProcessor` トレイト実装の完了
- 完全な処理フローテストパス
- パフォーマンスメトリクス計算の確認

---

#### **Task 5.2: エラーハンドリング強化**
**ファイル**: `src/processing/error.rs`

**実装内容**:
```rust
use thiserror::Error;

/// 並列処理固有のエラー型
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("ファイル発見エラー: {path} - {source}")]
    FileDiscoveryError {
        path: String,
        source: anyhow::Error,
    },
    
    #[error("並列処理エラー: {message}")]
    ParallelExecutionError {
        message: String,
    },
    
    #[error("永続化エラー: {source}")]
    PersistenceError {
        source: anyhow::Error,
    },
    
    #[error("設定エラー: {message}")]
    ConfigurationError {
        message: String,
    },
    
    #[error("チャンネルエラー: {message}")]
    ChannelError {
        message: String,
    },
    
    #[error("タスクエラー: {source}")]
    TaskError {
        source: tokio::task::JoinError,
    },
}

impl ProcessingError {
    pub fn file_discovery(path: impl Into<String>, source: anyhow::Error) -> Self {
        Self::FileDiscoveryError {
            path: path.into(),
            source,
        }
    }
    
    pub fn parallel_execution(message: impl Into<String>) -> Self {
        Self::ParallelExecutionError {
            message: message.into(),
        }
    }
    
    pub fn persistence(source: anyhow::Error) -> Self {
        Self::PersistenceError { source }
    }
    
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }
    
    pub fn channel(message: impl Into<String>) -> Self {
        Self::ChannelError {
            message: message.into(),
        }
    }
    
    pub fn task(source: tokio::task::JoinError) -> Self {
        Self::TaskError { source }
    }
}

/// 並列処理の結果型
pub type ProcessingResult<T> = std::result::Result<T, ProcessingError>;
```

**engine.rs の更新**:
```rust
use super::error::{ProcessingError, ProcessingResult};

impl<L, H, S> ParallelProcessingEngine<L, H, S> {
    async fn discover_image_files(&self, path: &str) -> ProcessingResult<Vec<String>> {
        self.storage.list_items(path).await
            .map_err(|e| ProcessingError::file_discovery(path, e))
            .map(|items| {
                let mut image_files = Vec::new();
                for item in items {
                    if !item.is_directory && self.storage.is_image_file(&item) {
                        image_files.push(item.id);
                    }
                }
                image_files.sort();
                image_files
            })
    }
}

#[async_trait]
impl<L, H, S> ParallelProcessor for ParallelProcessingEngine<L, H, S> {
    async fn process_directory(
        &self,
        path: &str,
        config: &Self::Config,
        reporter: &Self::Reporter,
        persistence: &Self::Persistence,
    ) -> ProcessingResult<ProcessingSummary> {
        // バリデーション
        if config.max_concurrent_tasks() == 0 {
            return Err(ProcessingError::configuration("並列タスク数は1以上である必要があります"));
        }
        
        if config.batch_size() == 0 {
            return Err(ProcessingError::configuration("バッチサイズは1以上である必要があります"));
        }
        
        let start_time = Instant::now();
        
        // ファイル発見
        let files = self.discover_image_files(path).await?;
        let total_files = files.len();
        
        if config.enable_progress_reporting() {
            reporter.report_started(total_files).await;
        }
        
        // パイプライン実行
        let pipeline = ProcessingPipeline::new(
            self.loader.clone(),
            self.hasher.clone(),
        );
        
        let mut summary = pipeline.execute(files, config, reporter, persistence).await
            .map_err(|e| ProcessingError::parallel_execution(format!("パイプライン実行エラー: {}", e)))?;
        
        // タイミング計測
        let total_time = start_time.elapsed().as_millis() as u64;
        summary.total_processing_time_ms = total_time;
        
        if summary.processed_files > 0 {
            summary.average_time_per_file_ms = total_time as f64 / summary.processed_files as f64;
        }
        
        if config.enable_progress_reporting() {
            reporter.report_completed(summary.processed_files, summary.error_count).await;
        }
        
        // 永続化完了
        persistence.finalize().await
            .map_err(ProcessingError::persistence)?;
        
        Ok(summary)
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_error_creation() {
        let file_error = ProcessingError::file_discovery(
            "/test/path",
            anyhow::anyhow!("ファイルが見つかりません"),
        );
        assert!(file_error.to_string().contains("/test/path"));
        assert!(file_error.to_string().contains("ファイル発見エラー"));
        
        let parallel_error = ProcessingError::parallel_execution("並列処理が失敗しました");
        assert!(parallel_error.to_string().contains("並列処理エラー"));
        
        let config_error = ProcessingError::configuration("無効な設定です");
        assert!(config_error.to_string().contains("設定エラー"));
    }
    
    #[tokio::test]
    async fn test_process_directory_validation_errors() {
        let temp_dir = TempDir::new().unwrap();
        
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        // 無効な並列数
        let invalid_config = DefaultProcessingConfig::default().with_max_concurrent(0);
        let result = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &invalid_config,
            &reporter,
            &persistence,
        ).await;
        
        assert!(matches!(result, Err(ProcessingError::ConfigurationError { .. })));
        
        // 無効なバッチサイズ
        let invalid_config = DefaultProcessingConfig::default().with_batch_size(0);
        let result = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &invalid_config,
            &reporter,
            &persistence,
        ).await;
        
        assert!(matches!(result, Err(ProcessingError::ConfigurationError { .. })));
    }
    
    #[tokio::test]
    async fn test_process_nonexistent_directory() {
        let engine = ParallelProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
        );
        
        let config = DefaultProcessingConfig::default();
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let result = engine.process_directory(
            "/nonexistent/directory",
            &config,
            &reporter,
            &persistence,
        ).await;
        
        assert!(matches!(result, Err(ProcessingError::FileDiscoveryError { .. })));
    }
}
```

**依存関係追加**: `thiserror = "1.0"` を `[dependencies]` に追加

**成功基準**:
- カスタムエラー型の実装完了
- エラーハンドリング強化の確認
- バリデーションエラーのテストパス

---

### **Phase 6: JSON永続化実装**
*推定作業時間: 1日*

#### **Task 6.1: JsonHashPersistence実装**
**ファイル**: `src/processing/implementations.rs`

**実装内容**:
```rust
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};

/// JSON形式で保存するハッシュデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    pub file_path: String,
    pub hash: String,
    pub metadata: ProcessingMetadata,
}

/// JSON形式での永続化実装
#[derive(Debug)]
pub struct JsonHashPersistence {
    file_path: String,
    writer: Option<BufWriter<File>>,
    entries_written: usize,
}

impl JsonHashPersistence {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            writer: None,
            entries_written: 0,
        }
    }
    
    /// ファイルを初期化（JSON配列開始）
    async fn initialize_file(&mut self) -> Result<()> {
        if self.writer.is_some() {
            return Ok(());
        }
        
        // 親ディレクトリが存在しない場合は作成
        if let Some(parent) = Path::new(&self.file_path).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| anyhow::anyhow!("ディレクトリ作成エラー: {}", e))?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)
            .await
            .map_err(|e| anyhow::anyhow!("ファイル作成エラー: {}", e))?;
            
        let mut writer = BufWriter::new(file);
        
        // JSON配列開始
        writer.write_all(b"[\n").await
            .map_err(|e| anyhow::anyhow!("書き込みエラー: {}", e))?;
            
        self.writer = Some(writer);
        Ok(())
    }
}

#[async_trait]
impl HashPersistence for JsonHashPersistence {
    async fn store_hash(
        &self,
        file_path: &str,
        hash: &str,
        metadata: &ProcessingMetadata,
    ) -> Result<()> {
        let entry = HashEntry {
            file_path: file_path.to_string(),
            hash: hash.to_string(),
            metadata: metadata.clone(),
        };
        
        self.store_batch(&[(entry.file_path, entry.hash, entry.metadata)]).await
    }
    
    async fn store_batch(
        &mut self,
        results: &[(String, String, ProcessingMetadata)],
    ) -> Result<()> {
        if results.is_empty() {
            return Ok(());
        }
        
        self.initialize_file().await?;
        
        let writer = self.writer.as_mut()
            .ok_or_else(|| anyhow::anyhow!("ファイルが初期化されていません"))?;
        
        for (file_path, hash, metadata) in results {
            let entry = HashEntry {
                file_path: file_path.clone(),
                hash: hash.clone(),
                metadata: metadata.clone(),
            };
            
            // カンマ追加（最初のエントリ以外）
            if self.entries_written > 0 {
                writer.write_all(b",\n").await
                    .map_err(|e| anyhow::anyhow!("書き込みエラー: {}", e))?;
            }
            
            // JSON エントリを書き込み
            let json_str = serde_json::to_string_pretty(&entry)
                .map_err(|e| anyhow::anyhow!("JSON変換エラー: {}", e))?;
                
            // インデント追加
            let indented = json_str.lines()
                .map(|line| format!("  {}", line))
                .collect::<Vec<_>>()
                .join("\n");
                
            writer.write_all(indented.as_bytes()).await
                .map_err(|e| anyhow::anyhow!("書き込みエラー: {}", e))?;
                
            self.entries_written += 1;
        }
        
        writer.flush().await
            .map_err(|e| anyhow::anyhow!("フラッシュエラー: {}", e))?;
            
        Ok(())
    }
    
    async fn finalize(&mut self) -> Result<()> {
        if let Some(mut writer) = self.writer.take() {
            // JSON配列終了
            writer.write_all(b"\n]").await
                .map_err(|e| anyhow::anyhow!("書き込みエラー: {}", e))?;
                
            writer.flush().await
                .map_err(|e| anyhow::anyhow!("フラッシュエラー: {}", e))?;
        }
        
        Ok(())
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    use serde_json::Value;

    #[tokio::test]
    async fn test_json_hash_persistence_single_entry() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("test_hashes.json");
        
        let mut persistence = JsonHashPersistence::new(&json_file);
        
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 150,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // 単一エントリ保存
        persistence.store_hash("/test.jpg", "abcd1234", &metadata).await.unwrap();
        
        // 完了処理
        persistence.finalize().await.unwrap();
        
        // ファイル内容確認
        let content = fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        
        assert!(json_value.is_array());
        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), 1);
        
        let entry = &array[0];
        assert_eq!(entry["file_path"], "/test.jpg");
        assert_eq!(entry["hash"], "abcd1234");
        assert_eq!(entry["metadata"]["file_size"], 1024);
        assert_eq!(entry["metadata"]["processing_time_ms"], 150);
    }
    
    #[tokio::test]
    async fn test_json_hash_persistence_batch() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("batch_hashes.json");
        
        let mut persistence = JsonHashPersistence::new(&json_file);
        
        let metadata = ProcessingMetadata {
            file_size: 2048,
            processing_time_ms: 200,
            image_dimensions: (1024, 1024),
            was_resized: true,
        };
        
        // バッチ保存
        let batch = vec![
            ("/test1.jpg".to_string(), "hash1".to_string(), metadata.clone()),
            ("/test2.png".to_string(), "hash2".to_string(), metadata.clone()),
            ("/test3.gif".to_string(), "hash3".to_string(), metadata.clone()),
        ];
        
        persistence.store_batch(&batch).await.unwrap();
        persistence.finalize().await.unwrap();
        
        // ファイル内容確認
        let content = fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        
        assert!(json_value.is_array());
        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), 3);
        
        for i in 0..3 {
            let entry = &array[i];
            assert_eq!(entry["file_path"], format!("/test{}.{}", i + 1, 
                match i { 0 => "jpg", 1 => "png", 2 => "gif", _ => unreachable!() }));
            assert_eq!(entry["hash"], format!("hash{}", i + 1));
            assert_eq!(entry["metadata"]["was_resized"], true);
        }
    }
    
    #[tokio::test]
    async fn test_json_hash_persistence_multiple_batches() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("multi_batch.json");
        
        let mut persistence = JsonHashPersistence::new(&json_file);
        
        let metadata = ProcessingMetadata {
            file_size: 512,
            processing_time_ms: 100,
            image_dimensions: (256, 256),
            was_resized: false,
        };
        
        // 複数バッチ保存
        let batch1 = vec![
            ("/batch1_1.jpg".to_string(), "hash1_1".to_string(), metadata.clone()),
            ("/batch1_2.jpg".to_string(), "hash1_2".to_string(), metadata.clone()),
        ];
        
        let batch2 = vec![
            ("/batch2_1.jpg".to_string(), "hash2_1".to_string(), metadata.clone()),
            ("/batch2_2.jpg".to_string(), "hash2_2".to_string(), metadata.clone()),
            ("/batch2_3.jpg".to_string(), "hash2_3".to_string(), metadata.clone()),
        ];
        
        persistence.store_batch(&batch1).await.unwrap();
        persistence.store_batch(&batch2).await.unwrap();
        persistence.finalize().await.unwrap();
        
        // ファイル内容確認
        let content = fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        
        assert!(json_value.is_array());
        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), 5);
        
        // 順序確認
        assert_eq!(array[0]["file_path"], "/batch1_1.jpg");
        assert_eq!(array[1]["file_path"], "/batch1_2.jpg");
        assert_eq!(array[2]["file_path"], "/batch2_1.jpg");
        assert_eq!(array[3]["file_path"], "/batch2_2.jpg");
        assert_eq!(array[4]["file_path"], "/batch2_3.jpg");
    }
    
    #[tokio::test]
    async fn test_json_hash_persistence_empty() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("empty.json");
        
        let mut persistence = JsonHashPersistence::new(&json_file);
        
        // 何も保存せずに完了
        persistence.finalize().await.unwrap();
        
        // ファイル確認
        let content = fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        
        assert!(json_value.is_array());
        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), 0);
    }
    
    #[tokio::test]
    async fn test_json_hash_persistence_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("directory").join("hashes.json");
        
        let mut persistence = JsonHashPersistence::new(&nested_path);
        
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        persistence.store_hash("/test.jpg", "hash", &metadata).await.unwrap();
        persistence.finalize().await.unwrap();
        
        // ファイルが作成されていることを確認
        assert!(nested_path.exists());
        
        let content = fs::read_to_string(&nested_path).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json_value.as_array().unwrap().len(), 1);
    }
}
```

**依存関係追加**: `serde = { version = "1.0", features = ["derive"] }`, `serde_json = "1.0"` を `[dependencies]` に追加

**成功基準**:
- JSON形式での永続化実装完了
- 単一・バッチ・複数バッチ保存のテストパス
- ディレクトリ自動作成のテストパス

---

#### **Task 6.2: 大量データ対応**
**ファイル**: `src/processing/implementations.rs`

**実装内容**:
```rust
/// 大量データ対応の改良版JSON永続化
#[derive(Debug)]
pub struct StreamingJsonHashPersistence {
    file_path: String,
    writer: Option<BufWriter<File>>,
    entries_written: usize,
    buffer_size: usize,
    write_buffer: Vec<u8>,
}

impl StreamingJsonHashPersistence {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self::with_buffer_size(file_path, 8192) // デフォルト8KB
    }
    
    pub fn with_buffer_size<P: AsRef<Path>>(file_path: P, buffer_size: usize) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            writer: None,
            entries_written: 0,
            buffer_size,
            write_buffer: Vec::with_capacity(buffer_size),
        }
    }
    
    /// バッファをフラッシュ
    async fn flush_buffer(&mut self) -> Result<()> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }
        
        if let Some(writer) = &mut self.writer {
            writer.write_all(&self.write_buffer).await
                .map_err(|e| anyhow::anyhow!("バッファ書き込みエラー: {}", e))?;
            writer.flush().await
                .map_err(|e| anyhow::anyhow!("フラッシュエラー: {}", e))?;
        }
        
        self.write_buffer.clear();
        Ok(())
    }
    
    /// バッファに書き込み（必要に応じてフラッシュ）
    async fn write_to_buffer(&mut self, data: &[u8]) -> Result<()> {
        self.write_buffer.extend_from_slice(data);
        
        if self.write_buffer.len() >= self.buffer_size {
            self.flush_buffer().await?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl HashPersistence for StreamingJsonHashPersistence {
    async fn store_hash(
        &mut self,
        file_path: &str,
        hash: &str,
        metadata: &ProcessingMetadata,
    ) -> Result<()> {
        self.store_batch(&[(file_path.to_string(), hash.to_string(), metadata.clone())]).await
    }
    
    async fn store_batch(
        &mut self,
        results: &[(String, String, ProcessingMetadata)],
    ) -> Result<()> {
        if results.is_empty() {
            return Ok(());
        }
        
        // 必要に応じてファイル初期化
        if self.writer.is_none() {
            if let Some(parent) = Path::new(&self.file_path).parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| anyhow::anyhow!("ディレクトリ作成エラー: {}", e))?;
            }
            
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.file_path)
                .await
                .map_err(|e| anyhow::anyhow!("ファイル作成エラー: {}", e))?;
                
            self.writer = Some(BufWriter::with_capacity(self.buffer_size * 2, file));
            self.write_to_buffer(b"[\n").await?;
        }
        
        for (file_path, hash, metadata) in results {
            let entry = HashEntry {
                file_path: file_path.clone(),
                hash: hash.clone(),
                metadata: metadata.clone(),
            };
            
            // カンマ追加（最初のエントリ以外）
            if self.entries_written > 0 {
                self.write_to_buffer(b",\n").await?;
            }
            
            // コンパクトなJSON生成（pretty printなし）
            let json_str = serde_json::to_string(&entry)
                .map_err(|e| anyhow::anyhow!("JSON変換エラー: {}", e))?;
            
            // インデント追加（最小限）
            let indented = format!("  {}", json_str);
            self.write_to_buffer(indented.as_bytes()).await?;
            
            self.entries_written += 1;
        }
        
        Ok(())
    }
    
    async fn finalize(&mut self) -> Result<()> {
        // 残りのバッファをフラッシュ
        self.flush_buffer().await?;
        
        if let Some(mut writer) = self.writer.take() {
            writer.write_all(b"\n]").await
                .map_err(|e| anyhow::anyhow!("書き込みエラー: {}", e))?;
            writer.flush().await
                .map_err(|e| anyhow::anyhow!("最終フラッシュエラー: {}", e))?;
        }
        
        Ok(())
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_json_persistence_large_dataset() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("large_dataset.json");
        
        let mut persistence = StreamingJsonHashPersistence::with_buffer_size(&json_file, 1024); // 小さなバッファ
        
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // 大量データをバッチで保存
        let batch_size = 100;
        let total_entries = 1000;
        
        for batch_idx in 0..(total_entries / batch_size) {
            let mut batch = Vec::new();
            for i in 0..batch_size {
                let entry_idx = batch_idx * batch_size + i;
                batch.push((
                    format!("/test{}.jpg", entry_idx),
                    format!("hash{}", entry_idx),
                    metadata.clone(),
                ));
            }
            
            persistence.store_batch(&batch).await.unwrap();
        }
        
        persistence.finalize().await.unwrap();
        
        // ファイル内容確認
        let content = fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        
        assert!(json_value.is_array());
        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), total_entries);
        
        // いくつかのエントリを確認
        assert_eq!(array[0]["file_path"], "/test0.jpg");
        assert_eq!(array[0]["hash"], "hash0");
        assert_eq!(array[999]["file_path"], "/test999.jpg");
        assert_eq!(array[999]["hash"], "hash999");
    }
    
    #[tokio::test]
    async fn test_streaming_vs_regular_performance() {
        let temp_dir = TempDir::new().unwrap();
        
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        let test_data: Vec<_> = (0..500).map(|i| (
            format!("/test{}.jpg", i),
            format!("hash{}", i),
            metadata.clone(),
        )).collect();
        
        // 通常版のテスト
        let regular_file = temp_dir.path().join("regular.json");
        let start_regular = std::time::Instant::now();
        {
            let mut regular = JsonHashPersistence::new(&regular_file);
            regular.store_batch(&test_data).await.unwrap();
            regular.finalize().await.unwrap();
        }
        let regular_time = start_regular.elapsed();
        
        // ストリーミング版のテスト
        let streaming_file = temp_dir.path().join("streaming.json");
        let start_streaming = std::time::Instant::now();
        {
            let mut streaming = StreamingJsonHashPersistence::new(&streaming_file);
            streaming.store_batch(&test_data).await.unwrap();
            streaming.finalize().await.unwrap();
        }
        let streaming_time = start_streaming.elapsed();
        
        // 結果の内容が同じであることを確認
        let regular_content = fs::read_to_string(&regular_file).await.unwrap();
        let streaming_content = fs::read_to_string(&streaming_file).await.unwrap();
        
        let regular_json: Value = serde_json::from_str(&regular_content).unwrap();
        let streaming_json: Value = serde_json::from_str(&streaming_content).unwrap();
        
        assert_eq!(regular_json.as_array().unwrap().len(), 500);
        assert_eq!(streaming_json.as_array().unwrap().len(), 500);
        
        // パフォーマンス情報を出力（テストログ用）
        println!("Regular: {:?}, Streaming: {:?}", regular_time, streaming_time);
        
        // どちらも合理的な時間で完了することを確認
        assert!(regular_time.as_millis() < 5000); // 5秒以内
        assert!(streaming_time.as_millis() < 5000); // 5秒以内
    }
    
    #[tokio::test]
    async fn test_streaming_memory_efficiency() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("memory_test.json");
        
        // 小さなバッファサイズで大量データを処理
        let mut persistence = StreamingJsonHashPersistence::with_buffer_size(&json_file, 256);
        
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // 小さなバッチを大量に送信してバッファフラッシュをテスト
        for i in 0..100 {
            let batch = vec![(
                format!("/test{}.jpg", i),
                format!("very_long_hash_string_to_test_buffer_efficiency_{}", i),
                metadata.clone(),
            )];
            
            persistence.store_batch(&batch).await.unwrap();
        }
        
        persistence.finalize().await.unwrap();
        
        let content = fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        
        assert_eq!(json_value.as_array().unwrap().len(), 100);
        
        // ファイルサイズが合理的であることを確認
        let file_metadata = fs::metadata(&json_file).await.unwrap();
        assert!(file_metadata.len() > 1000); // 最小サイズ確認
        assert!(file_metadata.len() < 1024 * 1024); // 1MB以下であることを確認
    }
}
```

**成功基準**:
- ストリーミング永続化の実装完了
- 大量データでのパフォーマンステストパス
- メモリ効率性のテストパス

### **Phase 7: 統合テストとパフォーマンス最適化**
*推定作業時間: 0.5日*

#### **Task 7.1: 統合テスト実装**
**ファイル**: `tests/integration_tests.rs`

**実装内容**:
```rust
use image_dedup::{
    App,
    image_loader::standard::StandardImageLoader,
    perceptual_hash::dct_hash::DCTHasher,
    storage::local::LocalStorageBackend,
    processing::{
        ParallelProcessingEngine,
        DefaultProcessingConfig,
        ConsoleProgressReporter,
        StreamingJsonHashPersistence,
        ParallelProcessor,
    },
};
use tempfile::TempDir;
use std::fs;
use serde_json::Value;

/// テスト用の小さなPNG画像データ
const SMALL_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// テスト用ディレクトリ構造を作成
fn create_test_directory_structure(temp_dir: &TempDir, file_count: usize) {
    let base_path = temp_dir.path();
    
    // 画像ファイル作成
    for i in 0..file_count {
        let file_path = base_path.join(format!("image_{:03}.png", i));
        fs::write(&file_path, SMALL_PNG).unwrap();
    }
    
    // サブディレクトリと画像作成
    let sub_dir = base_path.join("subdir");
    fs::create_dir(&sub_dir).unwrap();
    
    for i in 0..(file_count / 2) {
        let file_path = sub_dir.join(format!("sub_image_{:03}.png", i));
        fs::write(&file_path, SMALL_PNG).unwrap();
    }
    
    // 非画像ファイル作成（無視されるべき）
    fs::write(base_path.join("readme.txt"), b"This is not an image").unwrap();
    fs::write(base_path.join("data.json"), br#"{"key": "value"}"#).unwrap();
    
    // 無効な画像ファイル作成
    fs::write(base_path.join("invalid.jpg"), b"This is not a valid image").unwrap();
}

#[tokio::test]
async fn test_end_to_end_processing_small_dataset() {
    let temp_dir = TempDir::new().unwrap();
    create_test_directory_structure(&temp_dir, 10);
    
    // 処理エンジン作成
    let engine = ParallelProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
    );
    
    // 設定
    let config = DefaultProcessingConfig::default()
        .with_max_concurrent(4)
        .with_batch_size(3)
        .with_progress_reporting(false); // テスト用に無効化
    
    let reporter = ConsoleProgressReporter::quiet();
    
    // JSON出力ファイル
    let output_file = temp_dir.path().join("hashes_output.json");
    let persistence = StreamingJsonHashPersistence::new(&output_file);
    
    // 処理実行
    let summary = engine.process_directory(
        temp_dir.path().to_str().unwrap(),
        &config,
        &reporter,
        &persistence,
    ).await.unwrap();
    
    // 結果検証
    assert_eq!(summary.total_files, 16); // 10 + 5(sub) + 1(invalid) = 16
    assert_eq!(summary.processed_files, 15); // 有効な画像のみ
    assert_eq!(summary.error_count, 1); // 無効な画像1つ
    assert!(summary.total_processing_time_ms > 0);
    assert!(summary.average_time_per_file_ms > 0.0);
    
    // JSON出力ファイル確認
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    
    assert!(json.is_array());
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 15);
    
    // 各エントリの構造確認
    for entry in entries {
        assert!(entry["file_path"].is_string());
        assert!(entry["hash"].is_string());
        assert!(entry["metadata"]["file_size"].is_number());
        assert!(entry["metadata"]["processing_time_ms"].is_number());
        assert!(entry["metadata"]["image_dimensions"].is_array());
        assert!(entry["metadata"]["was_resized"].is_boolean());
    }
}

#[tokio::test]
async fn test_end_to_end_processing_medium_dataset() {
    let temp_dir = TempDir::new().unwrap();
    create_test_directory_structure(&temp_dir, 100);
    
    let engine = ParallelProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
    );
    
    let config = DefaultProcessingConfig::default()
        .with_max_concurrent(8)
        .with_batch_size(20)
        .with_progress_reporting(false);
    
    let reporter = ConsoleProgressReporter::quiet();
    let output_file = temp_dir.path().join("medium_dataset.json");
    let persistence = StreamingJsonHashPersistence::new(&output_file);
    
    let start_time = std::time::Instant::now();
    
    let summary = engine.process_directory(
        temp_dir.path().to_str().unwrap(),
        &config,
        &reporter,
        &persistence,
    ).await.unwrap();
    
    let elapsed = start_time.elapsed();
    
    // 結果検証
    assert_eq!(summary.total_files, 151); // 100 + 50(sub) + 1(invalid) = 151
    assert_eq!(summary.processed_files, 150);
    assert_eq!(summary.error_count, 1);
    
    // パフォーマンス確認（中程度のデータセット）
    assert!(elapsed.as_secs() < 30); // 30秒以内で完了
    assert!(summary.average_time_per_file_ms < 200.0); // 1ファイル200ms以下
    
    // JSON確認
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 150);
}

#[tokio::test] 
async fn test_concurrent_processing_stress() {
    let temp_dir = TempDir::new().unwrap();
    create_test_directory_structure(&temp_dir, 50);
    
    // 同じディレクトリに対して複数の処理エンジンを同時実行
    // （実際の使用例では非推奨だが、並行処理の堅牢性をテスト）
    
    let mut handles = Vec::new();
    
    for i in 0..3 {
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let output_file = temp_dir.path().join(format!("concurrent_{}.json", i));
        
        let handle = tokio::spawn(async move {
            let engine = ParallelProcessingEngine::new(
                StandardImageLoader::new(),
                DCTHasher::new(8),
                LocalStorageBackend::new(),
            );
            
            let config = DefaultProcessingConfig::default()
                .with_max_concurrent(4)
                .with_batch_size(5)
                .with_progress_reporting(false);
            
            let reporter = ConsoleProgressReporter::quiet();
            let persistence = StreamingJsonHashPersistence::new(&output_file);
            
            engine.process_directory(&temp_path, &config, &reporter, &persistence).await
        });
        
        handles.push(handle);
    }
    
    // 全タスク完了待機
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        results.push(result);
    }
    
    // 全ての結果が一致することを確認
    for summary in &results {
        assert_eq!(summary.total_files, 76); // 50 + 25(sub) + 1(invalid)
        assert_eq!(summary.processed_files, 75);
        assert_eq!(summary.error_count, 1);
    }
    
    // 全ての出力ファイルが作成されていることを確認
    for i in 0..3 {
        let output_file = temp_dir.path().join(format!("concurrent_{}.json", i));
        assert!(output_file.exists());
        
        let content = fs::read_to_string(&output_file).unwrap();
        let json: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 75);
    }
}

#[tokio::test]
async fn test_error_recovery_and_partial_processing() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // 有効な画像ファイル作成
    for i in 0..5 {
        let file_path = base_path.join(format!("valid_{}.png", i));
        fs::write(&file_path, SMALL_PNG).unwrap();
    }
    
    // 複数の無効な画像ファイル作成
    for i in 0..5 {
        let file_path = base_path.join(format!("invalid_{}.jpg", i));
        fs::write(&file_path, format!("Invalid image data {}", i).as_bytes()).unwrap();
    }
    
    // 権限エラーを引き起こすファイル（Unixシステムのみ）
    #[cfg(unix)]
    {
        let restricted_file = base_path.join("restricted.png");
        fs::write(&restricted_file, SMALL_PNG).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&restricted_file).unwrap().permissions();
        perms.set_mode(0o000); // 読み取り権限なし
        fs::set_permissions(&restricted_file, perms).unwrap();
    }
    
    let engine = ParallelProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
    );
    
    let config = DefaultProcessingConfig::default()
        .with_max_concurrent(3)
        .with_batch_size(2)
        .with_progress_reporting(false);
    
    let reporter = ConsoleProgressReporter::quiet();
    let output_file = temp_dir.path().join("error_recovery.json");
    let persistence = StreamingJsonHashPersistence::new(&output_file);
    
    let summary = engine.process_directory(
        temp_dir.path().to_str().unwrap(),
        &config,
        &reporter,
        &persistence,
    ).await.unwrap();
    
    // エラー回復確認
    assert_eq!(summary.processed_files, 5); // 有効な画像のみ
    
    #[cfg(unix)]
    assert!(summary.error_count >= 6); // 5つの無効 + 1つの権限エラー
    
    #[cfg(not(unix))]
    assert_eq!(summary.error_count, 5); // 5つの無効な画像
    
    // 有効な結果は正常に保存されていることを確認
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn test_empty_directory_handling() {
    let temp_dir = TempDir::new().unwrap();
    
    // 空のサブディレクトリのみ作成
    fs::create_dir(temp_dir.path().join("empty_sub")).unwrap();
    
    let engine = ParallelProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
    );
    
    let config = DefaultProcessingConfig::default();
    let reporter = ConsoleProgressReporter::quiet();
    let output_file = temp_dir.path().join("empty_result.json");
    let persistence = StreamingJsonHashPersistence::new(&output_file);
    
    let summary = engine.process_directory(
        temp_dir.path().to_str().unwrap(),
        &config,
        &reporter,
        &persistence,
    ).await.unwrap();
    
    // 空ディレクトリの処理確認
    assert_eq!(summary.total_files, 0);
    assert_eq!(summary.processed_files, 0);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.total_processing_time_ms, 0);
    
    // 空のJSONファイルが作成されることを確認
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 0);
}
```

**テスト内容**:
- エンドツーエンド処理テスト（小・中規模データセット）
- 並行処理ストレステスト
- エラー回復と部分処理テスト
- 空ディレクトリハンドリングテスト

**成功基準**:
- 全統合テストのパス
- 実際のファイルシステムでの動作確認
- エラー処理の堅牢性確認

---

#### **Task 7.2: ベンチマーク実装**
**ファイル**: `benches/parallel_processing_bench.rs`

**実装内容**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use image_dedup::{
    image_loader::standard::StandardImageLoader,
    perceptual_hash::dct_hash::DCTHasher,
    storage::local::LocalStorageBackend,
    processing::{
        ParallelProcessingEngine,
        DefaultProcessingConfig,
        NoOpProgressReporter,
        MemoryHashPersistence,
        ParallelProcessor,
    },
};
use tempfile::TempDir;
use std::fs;
use tokio::runtime::Runtime;

/// テスト用の小さなPNG画像データ
const SMALL_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// ベンチマーク用テストデータ作成
fn create_test_files(temp_dir: &TempDir, count: usize) {
    let base_path = temp_dir.path();
    
    for i in 0..count {
        let file_path = base_path.join(format!("bench_image_{:04}.png", i));
        fs::write(&file_path, SMALL_PNG).unwrap();
    }
}

/// 並列度別パフォーマンスベンチマーク
fn bench_concurrency_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    create_test_files(&temp_dir, 100);
    
    let mut group = c.benchmark_group("concurrency_scaling");
    
    for concurrency in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("parallel_processing", concurrency), 
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let engine = ParallelProcessingEngine::new(
                        StandardImageLoader::new(),
                        DCTHasher::new(8),
                        LocalStorageBackend::new(),
                    );
                    
                    let config = DefaultProcessingConfig::default()
                        .with_max_concurrent(concurrency)
                        .with_batch_size(10)
                        .with_progress_reporting(false);
                    
                    let reporter = NoOpProgressReporter::new();
                    let persistence = MemoryHashPersistence::new();
                    
                    black_box(
                        engine.process_directory(
                            temp_dir.path().to_str().unwrap(),
                            &config,
                            &reporter,
                            &persistence,
                        ).await.unwrap()
                    );
                });
            }
        );
    }
    group.finish();
}

/// データセットサイズ別パフォーマンスベンチマーク
fn bench_dataset_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("dataset_scaling");
    group.sample_size(10); // 大きなデータセットのためサンプル数を減らす
    
    for file_count in [10, 50, 100, 200].iter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(&temp_dir, *file_count);
        
        group.bench_with_input(
            BenchmarkId::new("file_count", file_count),
            file_count,
            |b, &file_count| {
                b.to_async(&rt).iter(|| async {
                    let engine = ParallelProcessingEngine::new(
                        StandardImageLoader::new(),
                        DCTHasher::new(8),
                        LocalStorageBackend::new(),
                    );
                    
                    let config = DefaultProcessingConfig::default()
                        .with_max_concurrent(4)
                        .with_batch_size(20)
                        .with_progress_reporting(false);
                    
                    let reporter = NoOpProgressReporter::new();
                    let persistence = MemoryHashPersistence::new();
                    
                    black_box(
                        engine.process_directory(
                            temp_dir.path().to_str().unwrap(),
                            &config,
                            &reporter,
                            &persistence,
                        ).await.unwrap()
                    );
                });
            }
        );
    }
    group.finish();
}

/// バッチサイズ別パフォーマンスベンチマーク
fn bench_batch_size_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    create_test_files(&temp_dir, 100);
    
    let mut group = c.benchmark_group("batch_size_optimization");
    
    for batch_size in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let engine = ParallelProcessingEngine::new(
                        StandardImageLoader::new(),
                        DCTHasher::new(8),
                        LocalStorageBackend::new(),
                    );
                    
                    let config = DefaultProcessingConfig::default()
                        .with_max_concurrent(4)
                        .with_batch_size(batch_size)
                        .with_progress_reporting(false);
                    
                    let reporter = NoOpProgressReporter::new();
                    let persistence = MemoryHashPersistence::new();
                    
                    black_box(
                        engine.process_directory(
                            temp_dir.path().to_str().unwrap(),
                            &config,
                            &reporter,
                            &persistence,
                        ).await.unwrap()
                    );
                });
            }
        );
    }
    group.finish();
}

/// メモリ使用量最適化ベンチマーク
fn bench_memory_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    create_test_files(&temp_dir, 50);
    
    let mut group = c.benchmark_group("memory_efficiency");
    
    // チャンネルバッファサイズの影響
    for buffer_size in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("channel_buffer", buffer_size),
            buffer_size,
            |b, &buffer_size| {
                b.to_async(&rt).iter(|| async {
                    let engine = ParallelProcessingEngine::new(
                        StandardImageLoader::new(),
                        DCTHasher::new(8),
                        LocalStorageBackend::new(),
                    );
                    
                    let config = DefaultProcessingConfig::default()
                        .with_max_concurrent(4)
                        .with_buffer_size(buffer_size)
                        .with_batch_size(10)
                        .with_progress_reporting(false);
                    
                    let reporter = NoOpProgressReporter::new();
                    let persistence = MemoryHashPersistence::new();
                    
                    black_box(
                        engine.process_directory(
                            temp_dir.path().to_str().unwrap(),
                            &config,
                            &reporter,
                            &persistence,
                        ).await.unwrap()
                    );
                });
            }
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_concurrency_scaling,
    bench_dataset_scaling,
    bench_batch_size_optimization,
    bench_memory_efficiency
);
criterion_main!(benches);
```

**Cargo.toml への追加**:
```toml
[[bench]]
name = "parallel_processing_bench"
harness = false

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }
```

**成功基準**:
- 並列度スケーリングの測定完了
- データセットサイズ影響の測定完了
- 最適なバッチサイズの特定

---

### **Phase 8: 公開API整備**
*推定作業時間: 0.5日*

#### **Task 8.1: モジュール公開設定**
**ファイル**: `src/lib.rs`

**実装内容**:
```rust
//! # 画像重複検出ツール - 並列処理ライブラリ
//! 
//! このライブラリは、大量の画像ファイルを効率的に並列処理し、
//! 知覚ハッシュを生成するための高性能な並列処理エンジンを提供します。
//! 
//! ## 主な機能
//! 
//! - **高性能な並列処理**: Producer-Consumerパターンによる効率的な並列実行
//! - **柔軟な設定**: 並列度、バッチサイズ、バッファサイズの調整可能
//! - **複数の永続化オプション**: JSON、メモリ内保存、ストリーミング対応
//! - **包括的な進捗報告**: リアルタイム進捗とエラー報告
//! - **厳密な型安全性**: Rustの型システムを活用したエラー防止
//! 
//! ## 基本的な使用例
//! 
//! ```rust
//! use image_dedup::{
//!     App,
//!     image_loader::standard::StandardImageLoader,
//!     perceptual_hash::dct_hash::DCTHasher,
//!     storage::local::LocalStorageBackend,
//!     processing::{
//!         ParallelProcessingEngine,
//!         DefaultProcessingConfig,
//!         ConsoleProgressReporter,
//!         StreamingJsonHashPersistence,
//!         ParallelProcessor,
//!     },
//! };
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. 依存関係の構築
//!     let loader = StandardImageLoader::new();
//!     let hasher = DCTHasher::new(8);
//!     let storage = LocalStorageBackend::new();
//!     
//!     // 2. 並列処理エンジンの構築
//!     let engine = ParallelProcessingEngine::new(loader, hasher, storage);
//!     
//!     // 3. 設定とコンポーネントの準備
//!     let config = DefaultProcessingConfig::default()
//!         .with_max_concurrent(8)
//!         .with_batch_size(50);
//!     
//!     let reporter = ConsoleProgressReporter::new();
//!     let persistence = StreamingJsonHashPersistence::new("hashes.json");
//!     
//!     // 4. 処理実行
//!     let summary = engine.process_directory(
//!         "./images",
//!         &config,
//!         &reporter,
//!         &persistence,
//!     ).await?;
//!     
//!     println!("処理完了: {}ファイル処理, {}エラー, {}ms",
//!              summary.processed_files,
//!              summary.error_count,
//!              summary.total_processing_time_ms);
//!     
//!     Ok(())
//! }
//! ```
//! 
//! ## アーキテクチャ
//! 
//! このライブラリは以下の主要コンポーネントで構成されています:
//! 
//! - **[`processing`]**: 並列処理エンジンとコア抽象化
//! - **[`image_loader`]**: 画像読み込み抽象化
//! - **[`perceptual_hash`]**: 知覚ハッシュ生成
//! - **[`storage`]**: ファイルシステム操作抽象化

pub mod image_loader;
pub mod perceptual_hash;
pub mod storage;

// 並列処理モジュールを公開
pub mod processing;

// 便利な再エクスポート
pub use processing::{
    // 主要なトレイト
    ParallelProcessor,
    ProcessingConfig,
    ProgressReporter,
    HashPersistence,
    
    // データ構造
    ProcessingSummary,
    ProcessingMetadata,
    
    // 具象実装
    ParallelProcessingEngine,
    DefaultProcessingConfig,
    ConsoleProgressReporter,
    NoOpProgressReporter,
    MemoryHashPersistence,
    JsonHashPersistence,
    StreamingJsonHashPersistence,
    
    // エラー型
    ProcessingError,
};

// DIコンテナの役割を果たすジェネリックなApp構造体
pub struct App<L, H, S>
where
    L: image_loader::ImageLoaderBackend,
    H: perceptual_hash::PerceptualHashBackend,
    S: storage::StorageBackend,
{
    pub loader: L,
    pub hasher: H,
    pub storage: S,
}

impl<L, H, S> App<L, H, S>
where
    L: image_loader::ImageLoaderBackend,
    H: Perceptual HashBackend,
    S: storage::StorageBackend,
{
    /// 新しいAppインスタンスを作成（コンストラクタインジェクション）
    pub fn new(loader: L, hasher: H, storage: S) -> Self {
        Self {
            loader,
            hasher,
            storage,
        }
    }

    /// 並列処理エンジンに変換
    /// 
    /// # 例
    /// 
    /// ```rust
    /// use image_dedup::*;
    /// 
    /// let app = App::new(
    ///     image_loader::standard::StandardImageLoader::new(),
    ///     perceptual_hash::dct_hash::DCTHasher::new(8),
    ///     storage::local::LocalStorageBackend::new(),
    /// );
    /// 
    /// let engine = app.into_parallel_engine();
    /// ```
    pub fn into_parallel_engine(self) -> ParallelProcessingEngine<L, H, S>
    where
        L: 'static,
        H: 'static,
        S: 'static,
    {
        ParallelProcessingEngine::from_app(self)
    }

    /// アプリケーションの主要なロジックを実行（従来の同期処理）
    /// 
    /// **注意**: この関数は後方互換性のために残されており、
    /// 新しいコードでは `into_parallel_engine()` を使用することを推奨します。
    pub async fn run(&self, path: &str) -> anyhow::Result<()> {
        println!("Starting image deduplication process in: {path}");

        let items = self.storage.list_items(path).await?;
        let image_files = items.iter().filter(|item| self.storage.is_image_file(item));

        for item in image_files {
            println!("Processing: {}", item.name);
            // ここで画像の読み込み、ハッシュ化、比較などの処理を実装
            // let image_data = self.storage.read_item(&item.id).await?;
            // let loaded_image = self.loader.load_from_bytes(&image_data).await?;
            // let hash = self.hasher.generate_hash(&loaded_image.image).await?;
            // println!("  - Hash: {}", hash.to_hex());
        }

        println!("Process finished.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::standard::StandardImageLoader;
    use crate::perceptual_hash::average_hash::AverageHasher;
    use crate::storage::{MockStorageBackend, StorageItem};
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_app_creation() {
        let app = App::new(
            StandardImageLoader::new(),
            AverageHasher::new(8),
            storage::local::LocalStorageBackend::new(),
        );
        
        // アプリケーション作成テスト
        assert!(true);
    }

    #[tokio::test]
    async fn test_app_to_parallel_engine_conversion() {
        let app = App::new(
            StandardImageLoader::new(),
            perceptual_hash::dct_hash::DCTHasher::new(8),
            storage::local::LocalStorageBackend::new(),
        );
        
        let engine = app.into_parallel_engine();
        
        // 変換テスト
        assert!(true);
    }

    #[tokio::test]
    async fn test_run_with_mock_storage() {
        let mut mock_storage = MockStorageBackend::new();

        // `list_items`が呼ばれたときの振る舞いを定義
        mock_storage
            .expect_list_items()
            .with(eq("test_path"))
            .times(1)
            .returning(|_| {
                Ok(vec![
                    StorageItem {
                        id: "image1.jpg".to_string(),
                        name: "image1.jpg".to_string(),
                        size: 1024,
                        is_directory: false,
                        extension: Some("jpg".to_string()),
                    },
                    StorageItem {
                        id: "not_an_image.txt".to_string(),
                        name: "not_an_image.txt".to_string(),
                        size: 100,
                        is_directory: false,
                        extension: Some("txt".to_string()),
                    },
                ])
            });

        // `is_image_file`が呼ばれたときの振る舞いを定義
        mock_storage
            .expect_is_image_file()
            .returning(|item| matches!(item.extension.as_deref(), Some("jpg")));

        let app = App::new(
            StandardImageLoader::new(),
            AverageHasher::new(8),
            mock_storage,
        );

        let result = app.run("test_path").await;
        assert!(result.is_ok());
    }
}
```

**テスト内容**:
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    const SMALL_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[tokio::test]
    async fn test_public_api_usage() {
        let temp_dir = TempDir::new().unwrap();
        
        // テスト画像作成
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("test_{}.png", i));
            fs::write(&file_path, SMALL_PNG).unwrap();
        }
        
        // 公開APIを使った処理
        let engine = ParallelProcessingEngine::new(
            image_loader::standard::StandardImageLoader::new(),
            perceptual_hash::dct_hash::DCTHasher::new(8),
            storage::local::LocalStorageBackend::new(),
        );
        
        let config = DefaultProcessingConfig::default()
            .with_max_concurrent(2)
            .with_batch_size(3);
        
        let reporter = ConsoleProgressReporter::quiet();
        let persistence = MemoryHashPersistence::new();
        
        let summary = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(summary.processed_files, 5);
        assert_eq!(summary.error_count, 0);
    }

    #[tokio::test]
    async fn test_app_to_engine_conversion() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.png"), SMALL_PNG).unwrap();
        
        let app = App::new(
            image_loader::standard::StandardImageLoader::new(),
            perceptual_hash::dct_hash::DCTHasher::new(8),
            storage::local::LocalStorageBackend::new(),
        );
        
        let engine = app.into_parallel_engine();
        
        let config = DefaultProcessingConfig::default();
        let reporter = NoOpProgressReporter::new();
        let persistence = MemoryHashPersistence::new();
        
        let summary = engine.process_directory(
            temp_dir.path().to_str().unwrap(),
            &config,
            &reporter,
            &persistence,
        ).await.unwrap();
        
        assert_eq!(summary.processed_files, 1);
        assert_eq!(summary.error_count, 0);
    }
}
```

**成功基準**:
- 公開APIの適切な露出
- 再エクスポートの動作確認
- 後方互換性の維持

---

#### **Task 8.2: ドキュメンテーション**
**ファイル**: `src/processing/mod.rs`

**実装内容**:
```rust
//! # 並列処理モジュール
//! 
//! このモジュールは、大量の画像ファイルを効率的に並列処理するためのコア機能を提供します。
//! Producer-Consumerパターンを使用して、スケーラブルで信頼性の高い並列処理を実現しています。
//! 
//! ## アーキテクチャ概要
//! 
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │    Producer     │───▶│  Consumer Pool   │───▶│ Result Collector│
//! │                 │    │                  │    │                 │
//! │ ・File Discovery│    │ ・Image Loading  │    │ ・Batch Storage │
//! │ ・Path Queuing  │    │ ・Hash Generation│    │ ・Progress Track│
//! │                 │    │ ・Error Handling │    │ ・Summary Report│
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//! ```
//! 
//! ## 主要コンポーネント
//! 
//! ### トレイト（抽象化レイヤー）
//! 
//! - [`ParallelProcessor`]: 並列処理オーケストレーター
//! - [`ProcessingConfig`]: 処理設定の抽象化  
//! - [`ProgressReporter`]: 進捗報告の抽象化
//! - [`HashPersistence`]: 結果永続化の抽象化
//! 
//! ### 具象実装
//! 
//! - [`ParallelProcessingEngine`]: メイン処理エンジン
//! - [`DefaultProcessingConfig`]: 標準設定実装
//! - [`ConsoleProgressReporter`]: コンソール進捗報告
//! - [`JsonHashPersistence`]: JSON形式永続化
//! - [`StreamingJsonHashPersistence`]: 大量データ対応JSON永続化
//! 
//! ## 使用パターン
//! 
//! ### 基本的な使用例
//! 
//! ```rust
//! use image_dedup::processing::*;
//! 
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // エンジン構築
//! let engine = ParallelProcessingEngine::new(
//!     /* loader */, /* hasher */, /* storage */
//! );
//! 
//! // 設定
//! let config = DefaultProcessingConfig::default()
//!     .with_max_concurrent(8)
//!     .with_batch_size(50);
//! 
//! // 実行
//! let summary = engine.process_directory(
//!     "/path/to/images",
//!     &config,
//!     &ConsoleProgressReporter::new(),
//!     &StreamingJsonHashPersistence::new("output.json"),
//! ).await?;
//! 
//! println!("処理完了: {}", summary.processed_files);
//! # Ok(())
//! # }
//! ```
//! 
//! ### 高度な設定例
//! 
//! ```rust
//! # use image_dedup::processing::*;
//! # async fn advanced_example() -> Result<(), Box<dyn std::error::Error>> {
//! // カスタム設定
//! let config = DefaultProcessingConfig::default()
//!     .with_max_concurrent(16)      // 高い並列度
//!     .with_buffer_size(200)        // 大きなバッファ  
//!     .with_batch_size(100)         // 大きなバッチサイズ
//!     .with_progress_reporting(true);
//! 
//! // カスタム進捗報告（サイレント）
//! let reporter = NoOpProgressReporter::new();
//! 
//! // メモリ内保存（テスト用）
//! let persistence = MemoryHashPersistence::new();
//! 
//! // 処理実行
//! // ...
//! # Ok(())
//! # }
//! ```
//! 
//! ## パフォーマンス調整
//! 
//! ### 並列度の調整
//! 
//! ```rust
//! # use image_dedup::processing::*;
//! // CPU集約的: コア数と同じ
//! let cpu_bound = DefaultProcessingConfig::default()
//!     .with_max_concurrent(num_cpus::get());
//! 
//! // I/Oバウンド: コア数の1.5-2倍
//! let io_bound = DefaultProcessingConfig::default()  
//!     .with_max_concurrent(num_cpus::get() * 2);
//! ```
//! 
//! ### メモリ使用量の制御
//! 
//! ```rust
//! # use image_dedup::processing::*;
//! // 小メモリ環境用
//! let low_memory = DefaultProcessingConfig::default()
//!     .with_buffer_size(50)     // 小さなチャンネルバッファ
//!     .with_batch_size(10);     // 小さなバッチサイズ
//! 
//! // 高メモリ環境用
//! let high_memory = DefaultProcessingConfig::default()
//!     .with_buffer_size(500)    // 大きなチャンネルバッファ  
//!     .with_batch_size(200);    // 大きなバッチサイズ
//! ```
//! 
//! ## エラーハンドリング
//! 
//! このモジュールは包括的なエラー処理を提供します：
//! 
//! ```rust
//! # use image_dedup::processing::*;
//! # async fn error_handling_example() -> Result<(), ProcessingError> {
//! match engine.process_directory("/path", &config, &reporter, &persistence).await {
//!     Ok(summary) => {
//!         println!("成功: {}ファイル処理", summary.processed_files);
//!         if summary.error_count > 0 {
//!             println!("警告: {}ファイルでエラー", summary.error_count);
//!         }
//!     }
//!     Err(ProcessingError::FileDiscoveryError { path, source }) => {
//!         eprintln!("ディレクトリ読み取りエラー: {} - {}", path, source);
//!     }
//!     Err(ProcessingError::ConfigurationError { message }) => {
//!         eprintln!("設定エラー: {}", message);  
//!     }
//!     Err(e) => {
//!         eprintln!("その他のエラー: {}", e);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//! 
//! ## パフォーマンス特性
//! 
//! - **スループット**: 1000ファイル/分 (典型的な環境)
//! - **レイテンシ**: ファイル当たり50-200ms
//! - **メモリ効率**: 設定可能なバッファによる制御
//! - **CPU使用率**: 並列度による線形スケーリング
//! 
//! ## 制限事項
//! 
//! - 最大並列度: 1000（実用的には32程度を推奨）
//! - 最大バッチサイズ: 10000（実用的には100程度を推奨）
//! - メモリ使用量: バッファサイズとバッチサイズに比例

// モジュール内容（実際の実装）
// ... (既存のコード)
```

**各トレイトと構造体のドキュメント追加**:
```rust
/// 並列処理オーケストレーターの抽象化
/// 
/// このトレイトは、ディレクトリ内の画像ファイルを並列処理するための
/// 高レベルなインターフェースを提供します。実装は、ファイル発見、
/// 並列ハッシュ生成、結果収集のパイプライン全体を制御します。
/// 
/// # 型パラメータ
/// 
/// - `Config`: 処理設定の型
/// - `Reporter`: 進捗報告の型  
/// - `Persistence`: 結果永続化の型
/// 
/// # 例
/// 
/// ```rust
/// # use image_dedup::processing::*;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let processor: ParallelProcessingEngine<_, _, _> = /* ... */;
/// let config = DefaultProcessingConfig::default();
/// let reporter = ConsoleProgressReporter::new();
/// let persistence = JsonHashPersistence::new("results.json");
/// 
/// let summary = processor.process_directory(
///     "/images",
///     &config, 
///     &reporter,
///     &persistence
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait ParallelProcessor: Send + Sync {
    // ... 既存のコード
}

/// 並列処理設定の抽象化
/// 
/// 処理の動作パラメータを定義するトレイトです。
/// 並列度、バッファサイズ、バッチサイズなどを制御できます。
/// 
/// # 実装のガイドライン
/// 
/// - `max_concurrent_tasks`: 1以上の値を返す必要があります
/// - `channel_buffer_size`: メモリ使用量とパフォーマンスのバランスを考慮
/// - `batch_size`: I/O効率とメモリ使用量のトレードオフ
/// - `enable_progress_reporting`: falseにするとわずかにパフォーマンス向上
pub trait ProcessingConfig: Send + Sync {
    // ... 既存のコード
}
```

**成功基準**:
- 包括的なAPIドキュメント完成
- `cargo doc` での正常なドキュメント生成
- 使用例の動作確認

---

## 実装完了後の検証

### 全体テスト実行
```bash
# 全テスト実行
cargo test

# クリップ実行
cargo clippy

# ドキュメント生成
cargo doc --open

# ベンチマーク実行
cargo bench

# 統合テスト実行
cargo test --test integration_tests
```

### 推奨最終確認項目

1. **機能テスト**: 各Phaseの機能が正常動作
2. **パフォーマンステスト**: 想定スループットの達成
3. **メモリ効率テスト**: メモリリークの無いこと
4. **エラーハンドリングテスト**: 様々なエラー条件での堅牢性
5. **ドキュメントテスト**: 使用例が正常に動作

---

## まとめ

本レポートは、画像重複検出ツールの並列処理機能を実装するための詳細なタスク計画書です。Phase 1から8まで、合計**25の具体的なタスク**で構成され、各タスクには：

- **詳細な実装コード例**
- **包括的なテストケース**
- **明確な成功基準**
- **必要な依存関係情報**

各タスク完了後は必ず `cargo test && cargo clippy` で品質を確認し、次のタスクに進むようにしてください。

**合計推定作業時間**: 約4.5日
**推奨実装順序**: Phase 1 → Phase 2 → ... → Phase 8

チームメンバーがこのレポートに従って、一貫した品質で段階的に並列処理機能を実装できます。