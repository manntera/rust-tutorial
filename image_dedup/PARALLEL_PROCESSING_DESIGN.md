# 画像重複検出ツール - 並列処理設計書

## 概要

本書は、画像重複検出ツールにおいて3万枚以上の大量画像を効率的に処理するための並列処理アーキテクチャの設計を詳述します。厳密な依存性注入（DI）原則に基づいた、拡張性・保守性・テスタビリティを重視した設計となっています。

## 設計方針

### 1. アーキテクチャパターン
- **Producer-Consumer パターン**: 効率的な並列処理とバックプレッシャー制御
- **依存性注入（DI）**: 全コンポーネントの抽象化と疎結合
- **単一責任原則**: 各コンポーネントの明確な役割分離
- **開放閉鎖原則**: 新機能追加時の既存コード非変更

### 2. 品質属性
- **パフォーマンス**: CPUコア数×2の並列処理で最適化
- **スケーラビリティ**: 設定による動的な並列数調整
- **信頼性**: エラー隔離と継続処理
- **保守性**: トレイトベースの抽象化
- **テスタビリティ**: モックによる単体テスト

## アーキテクチャ設計

### 1. モジュール構造

```
src/processing/
├── mod.rs           # 公開API・トレイト定義
├── engine.rs        # コア処理エンジン
├── pipeline.rs      # Producer-Consumerパイプライン
└── implementations.rs # 具象実装
```

### 2. 抽象化レイヤー

#### 2.1 ProcessingConfig トレイト
```rust
pub trait ProcessingConfig: Send + Sync {
    fn max_concurrent_tasks(&self) -> usize;
    fn channel_buffer_size(&self) -> usize;
    fn batch_size(&self) -> usize;
    fn enable_progress_reporting(&self) -> bool;
}
```

**責務**: 並列処理の動作パラメータを抽象化
- 同時実行タスク数の制御
- チャンネルバッファサイズの調整
- バッチ処理サイズの指定
- 進捗報告の有効/無効

#### 2.2 ProgressReporter トレイト
```rust
#[async_trait]
pub trait ProgressReporter: Send + Sync {
    async fn report_started(&self, total_files: usize);
    async fn report_progress(&self, completed: usize, total: usize);
    async fn report_error(&self, file_path: &str, error: &str);
    async fn report_completed(&self, total_processed: usize, total_errors: usize);
}
```

**責務**: 処理進捗の報告を抽象化
- 処理開始/完了の通知
- リアルタイム進捗更新
- エラー情報の報告
- 複数出力先対応（コンソール、ログファイル、GUI等）

#### 2.3 HashPersistence トレイト
```rust
#[async_trait]
pub trait HashPersistence: Send + Sync {
    async fn store_hash(&self, file_path: &str, hash: &str, metadata: &ProcessingMetadata) -> Result<()>;
    async fn store_batch(&self, results: &[(String, String, ProcessingMetadata)]) -> Result<()>;
    async fn finalize(&self) -> Result<()>;
}
```

**責務**: 処理結果の永続化を抽象化
- 単一結果の保存
- バッチ保存による効率化
- トランザクション制御
- 複数フォーマット対応（JSON、SQLite、CSV等）

#### 2.4 ParallelProcessor トレイト
```rust
#[async_trait]
pub trait ParallelProcessor: Send + Sync {
    type Config: ProcessingConfig;
    type Reporter: ProgressReporter;  
    type Persistence: HashPersistence;

    async fn process_directory(
        &self, 
        path: &str,
        config: &Self::Config,
        reporter: &Self::Reporter,
        persistence: &Self::Persistence,
    ) -> Result<ProcessingSummary>;
}
```

**責務**: 並列処理オーケストレーション
- ディレクトリ単位の処理制御
- 依存関係の組み立て
- 処理サマリーの提供

### 3. コア実装

#### 3.1 ParallelProcessingEngine
```rust
pub struct ParallelProcessingEngine<L, H, S> {
    loader: Arc<L>,
    hasher: Arc<H>,
    storage: Arc<S>,
}
```

**特徴**:
- ジェネリック型による柔軟な実装選択
- Arc\<T\>による効率的な参照共有
- 既存のAppインスタンスからの構築対応

**主要メソッド**:
- `new(loader, hasher, storage)`: コンストラクタインジェクション
- `from_app(app)`: 既存App構造体からの変換
- `process_directory()`: メイン処理エントリーポイント

#### 3.2 ProcessingPipeline
Producer-Consumer パターンの中核実装

**コンポーネント分離**:
1. **Producer**: ファイルパスの配信
2. **Consumer Pool**: 並列ワーカーによる処理
3. **Result Collector**: 結果収集と永続化

**同期制御**:
- `mpsc::channel`: 非同期メッセージパッシング
- `Semaphore`: 同時実行数制御
- `Arc<RwLock>`: 共有カウンタ管理

## データフロー

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│    Producer     │───▶│  Consumer Pool   │───▶│ Result Collector│
│                 │    │                  │    │                 │
│ ・File Discovery│    │ ・Image Loading  │    │ ・Batch Storage │
│ ・Path Queuing  │    │ ・Hash Generation│    │ ・Progress Track│
│                 │    │ ・Error Handling │    │ ・Summary Report│
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Work Channel    │    │   Semaphore      │    │ Result Channel  │
│ (Buffer: 100)   │    │ (CPU Cores × 2)  │    │ (Buffer: 100)   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## パフォーマンス最適化

### 1. 並列度調整
- **デフォルト**: `CPU コア数 × 2`
- **I/Oバウンド**: ディスク読み込み待機時の効率化
- **動的調整**: システム負荷に応じた実行時変更

### 2. メモリ管理
- **画像リサイズ**: 512×512ピクセル以下で処理高速化
- **チャンネルバッファ**: バックプレッシャー制御
- **バッチ処理**: データベース書き込み効率化

### 3. エラー処理戦略
- **隔離原則**: 個別ファイルエラーが全体処理を停止させない
- **継続処理**: 処理可能なファイルの並列実行継続
- **詳細ログ**: トラブルシューティング用の情報保持

## 具象実装例

### 1. 設定実装
```rust
pub struct DefaultProcessingConfig {
    max_concurrent: usize,     // 並列数
    buffer_size: usize,        // チャンネルバッファ
    batch_size: usize,         // バッチサイズ  
    enable_progress: bool,     // 進捗報告
}

impl Default for DefaultProcessingConfig {
    fn default() -> Self {
        Self {
            max_concurrent: num_cpus::get() * 2,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        }
    }
}
```

### 2. 進捗報告実装
```rust
pub struct ConsoleProgressReporter;

#[async_trait]
impl ProgressReporter for ConsoleProgressReporter {
    async fn report_progress(&self, completed: usize, total: usize) {
        if completed % 100 == 0 {
            println!("📊 Progress: {}/{} ({:.1}%)", 
                completed, total, (completed as f64 / total as f64) * 100.0);
        }
    }
    // 他のメソッド実装...
}
```

### 3. JSON永続化実装
```rust
pub struct JsonHashPersistence {
    file_path: String,
    buffer: Vec<(String, String, ProcessingMetadata)>,
}

#[async_trait]  
impl HashPersistence for JsonHashPersistence {
    async fn store_batch(&self, results: &[(String, String, ProcessingMetadata)]) -> Result<()> {
        // JSON形式でファイルに追記保存
        // トランザクション制御で安全性確保
    }
}
```

## 使用例

### 基本的な使用パターン
```rust
async fn execute_parallel_scan() -> Result<()> {
    // 1. 依存関係の構築
    let loader = StandardImageLoader::with_max_dimension(512);
    let hasher = DCTHasher::new(8);
    let storage = LocalStorageBackend::new();
    
    // 2. 処理エンジンの構築
    let engine = ParallelProcessingEngine::new(loader, hasher, storage);
    
    // 3. 設定とレポーターの注入
    let config = DefaultProcessingConfig::default();
    let reporter = Box::new(ConsoleProgressReporter) as Box<dyn ProgressReporter>;
    let persistence = Box::new(JsonHashPersistence::new("hashes.json")) 
                         as Box<dyn HashPersistence>;
    
    // 4. 処理実行
    let summary = engine.process_directory(
        "./images",
        &config, 
        &reporter,
        &persistence,
    ).await?;
    
    println!("処理完了 - 成功: {}, エラー: {}, 処理時間: {}ms", 
             summary.processed_files, 
             summary.error_count,
             summary.total_processing_time_ms);
    
    Ok(())
}
```

### カスタム実装の例
```rust
// 独自の進捗報告実装
pub struct FileProgressReporter {
    log_file: tokio::fs::File,
}

// 独自の永続化実装  
pub struct SqliteHashPersistence {
    connection: sqlx::SqlitePool,
}

// 独自設定での実行
let custom_config = CustomProcessingConfig {
    max_concurrent: 16,
    buffer_size: 200,
    batch_size: 100,
    enable_progress: true,
};

let file_reporter = Box::new(FileProgressReporter::new("progress.log"))
                       as Box<dyn ProgressReporter>;
let sqlite_persistence = Box::new(SqliteHashPersistence::new("hashes.db"))
                            as Box<dyn HashPersistence>;
```

## テスト戦略

### 1. 単体テスト
各トレイトの実装に対する個別テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    
    mock! {
        TestProgressReporter {}
        
        #[async_trait]
        impl ProgressReporter for TestProgressReporter {
            async fn report_started(&self, total_files: usize);
            async fn report_progress(&self, completed: usize, total: usize);
            async fn report_error(&self, file_path: &str, error: &str);
            async fn report_completed(&self, total_processed: usize, total_errors: usize);
        }
    }
    
    #[tokio::test]
    async fn test_progress_reporting() {
        let mut mock_reporter = MockTestProgressReporter::new();
        mock_reporter
            .expect_report_started()
            .times(1)
            .with(eq(1000))
            .return_const(());
            
        // テスト実装...
    }
}
```

### 2. 統合テスト
実際のファイルシステムとの統合テスト
```rust
#[tokio::test]
async fn test_end_to_end_processing() {
    let temp_dir = tempfile::tempdir()?;
    // テスト用画像ファイル生成
    // 処理実行
    // 結果検証
}
```

### 3. パフォーマンステスト
```rust
#[tokio::test]
async fn test_performance_with_large_dataset() {
    // 1000ファイルでのパフォーマンス測定
    // メモリ使用量監視
    // 処理時間測定
}
```

## 実装スケジュール

### Phase 1: 基盤実装 (1-2日)
- [ ] トレイト定義の実装
- [ ] 基本的な具象クラス実装
- [ ] 単体テスト作成

### Phase 2: パイプライン実装 (2-3日)  
- [ ] Producer-Consumerパイプラインの実装
- [ ] エラーハンドリング強化
- [ ] パフォーマンステスト

### Phase 3: 統合・最適化 (1-2日)
- [ ] 既存コードベースとの統合
- [ ] パフォーマンス最適化
- [ ] ドキュメント整備

## 想定される拡張

### 1. GPU処理対応
```rust
pub struct GpuProcessingEngine<L, H, S> {
    // WGPU実装
}
```

### 2. 分散処理対応
```rust
pub struct DistributedProcessingEngine {
    // 複数マシンでの分散処理
}
```

### 3. リアルタイム監視
```rust  
pub struct MetricsReporter {
    // Prometheus メトリクス送信
}
```

## まとめ

本設計により以下の利点が実現されます：

1. **高パフォーマンス**: 3万枚の画像を効率的に並列処理
2. **保守性**: 明確な責任分離と抽象化
3. **拡張性**: 新機能追加時の既存コード非変更
4. **テスタビリティ**: 各コンポーネントの独立テスト
5. **設定可能性**: 実行時パラメータ調整

厳密なDI原則により、将来の要件変更や新技術導入に柔軟に対応可能な、堅牢で拡張性の高いアーキテクチャとなっています。