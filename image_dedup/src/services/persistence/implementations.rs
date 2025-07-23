// データ永続化の具象実装

use crate::core::HashPersistence;
use crate::core::ProcessingMetadata;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex as AsyncMutex;

/// メモリ内保存の永続化実装（テスト用および開発用）
/// モックテストにも使用可能な完全機能実装
#[derive(Debug, Clone)]
pub struct MemoryHashPersistence {
    storage: Arc<Mutex<HashMap<String, (String, String, u64, ProcessingMetadata)>>>,
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
        self.storage
            .lock()
            .unwrap()
            .iter()
            .map(|(k, (hash, _alg, _bits, meta))| (k.clone(), (hash.clone(), meta.clone())))
            .collect()
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

    /// テスト用：特定のファイルが保存されているかチェック
    pub fn contains_file(&self, file_path: &str) -> bool {
        self.storage.lock().unwrap().contains_key(file_path)
    }

    /// テスト用：保存されたファイル数を取得
    pub fn stored_count(&self) -> usize {
        self.storage.lock().unwrap().len()
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
        self.storage.lock().unwrap().insert(
            file_path.to_string(),
            (hash.to_string(), "DCT".to_string(), 0u64, metadata.clone()),
        );
        Ok(())
    }

    async fn store_batch(
        &self,
        results: &[(String, String, String, u64, ProcessingMetadata)],
    ) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();
        for (path, hash, algorithm, hash_bits, metadata) in results {
            storage.insert(
                path.clone(),
                (
                    hash.clone(),
                    algorithm.clone(),
                    *hash_bits,
                    metadata.clone(),
                ),
            );
        }
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        *self.finalized.lock().unwrap() = true;
        Ok(())
    }
}

/// JSON形式で保存するハッシュデータ（画像単位）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    pub file_path: String,
    pub hash: String,
    pub hash_bits: u64,
    pub metadata: ProcessingMetadata,
}

/// スキャン情報（アルゴリズムとパラメーター）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanInfo {
    pub algorithm: String,
    pub parameters: serde_json::Value,
    pub timestamp: String,
    pub total_files: usize,
}

/// 新しいJSON出力フォーマット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub scan_info: ScanInfo,
    pub images: Vec<HashEntry>,
}

/// JSON形式での永続化実装
pub struct JsonHashPersistence {
    file_path: String,
    writer: Arc<AsyncMutex<Option<BufWriter<File>>>>,
    entries_written: Arc<AsyncMutex<usize>>,
}

impl JsonHashPersistence {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            writer: Arc::new(AsyncMutex::new(None)),
            entries_written: Arc::new(AsyncMutex::new(0)),
        }
    }

    /// ファイルを初期化（JSON配列開始）
    async fn initialize_file(&self) -> Result<()> {
        let mut writer_guard = self.writer.lock().await;
        if writer_guard.is_some() {
            return Ok(());
        }

        // 親ディレクトリが存在しない場合は作成
        if let Some(parent) = Path::new(&self.file_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow::anyhow!("ディレクトリ作成エラー: {e}"))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)
            .await
            .map_err(|e| anyhow::anyhow!("ファイル作成エラー: {e}"))?;

        let mut writer = BufWriter::new(file);

        // JSON配列開始
        writer
            .write_all(b"[\n")
            .await
            .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;

        *writer_guard = Some(writer);
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
        self.store_batch(&[(
            file_path.to_string(),
            hash.to_string(),
            "DCT".to_string(),
            0u64,
            metadata.clone(),
        )])
        .await
    }

    async fn store_batch(
        &self,
        results: &[(String, String, String, u64, ProcessingMetadata)],
    ) -> Result<()> {
        if results.is_empty() {
            return Ok(());
        }

        self.initialize_file().await?;

        // エントリごとに個別にロックを取得
        for (file_path, hash, algorithm, hash_bits, metadata) in results {
            let entry = HashEntry {
                file_path: file_path.clone(),
                hash: hash.clone(),
                hash_bits: *hash_bits,
                metadata: metadata.clone(),
            };

            // JSON文字列を準備
            let json_str = serde_json::to_string_pretty(&entry)
                .map_err(|e| anyhow::anyhow!("JSON変換エラー: {e}"))?;

            let indented = json_str
                .lines()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n");

            // 書き込み前の状態チェックとライター取得
            let (writer_exists, needs_comma) = {
                let writer_opt = self.writer.lock().await;
                let entries_written = *self.entries_written.lock().await;
                (writer_opt.is_some(), entries_written > 0)
            };

            if !writer_exists {
                return Err(anyhow::anyhow!("ファイルが初期化されていません"));
            }

            // 実際の書き込み（ロックを短時間だけ保持）
            {
                let mut writer_opt = self.writer.lock().await;
                let writer = writer_opt.as_mut().unwrap();

                if needs_comma {
                    writer
                        .write_all(b",\n")
                        .await
                        .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                }

                writer
                    .write_all(indented.as_bytes())
                    .await
                    .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
            }

            *self.entries_written.lock().await += 1;
        }

        // 最後にフラッシュ
        {
            let mut writer_opt = self.writer.lock().await;
            if let Some(writer) = writer_opt.as_mut() {
                writer
                    .flush()
                    .await
                    .map_err(|e| anyhow::anyhow!("フラッシュエラー: {e}"))?;
            }
        }

        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        let writer_opt = {
            let mut guard = self.writer.lock().await;
            guard.take()
        };

        if let Some(mut writer) = writer_opt {
            // JSON配列終了
            writer
                .write_all(b"\n]")
                .await
                .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;

            writer
                .flush()
                .await
                .map_err(|e| anyhow::anyhow!("フラッシュエラー: {e}"))?;
        } else {
            // ファイルが初期化されていない場合（何も保存されていない）
            // 空のJSON配列ファイルを作成
            self.initialize_file().await?;
            let mut guard = self.writer.lock().await;
            if let Some(mut writer) = guard.take() {
                writer
                    .write_all(b"\n]")
                    .await
                    .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                writer
                    .flush()
                    .await
                    .map_err(|e| anyhow::anyhow!("フラッシュエラー: {e}"))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::TempDir;
    use tokio::fs;

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
        persistence
            .store_hash("/test1.jpg", "hash1", &metadata)
            .await
            .unwrap();

        let stored = persistence.get_stored_data();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored["/test1.jpg"].0, "hash1");
        assert_eq!(stored["/test1.jpg"].1, metadata);

        // バッチ保存テスト
        let batch = vec![
            (
                "/test2.jpg".to_string(),
                "hash2".to_string(),
                metadata.clone(),
            ),
            (
                "/test3.jpg".to_string(),
                "hash3".to_string(),
                metadata.clone(),
            ),
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

        persistence
            .store_hash("/test.jpg", "hash", &metadata)
            .await
            .unwrap();
        persistence.finalize().await.unwrap();

        assert_eq!(persistence.get_stored_data().len(), 1);
        assert!(persistence.is_finalized());

        persistence.clear();

        assert_eq!(persistence.get_stored_data().len(), 0);
        assert!(!persistence.is_finalized());
    }

    #[tokio::test]
    async fn test_json_hash_persistence_single_entry() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("test_hashes.json");

        let persistence = JsonHashPersistence::new(&json_file);

        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 150,
            image_dimensions: (512, 512),
            was_resized: false,
        };

        // 単一エントリ保存
        persistence
            .store_hash("/test.jpg", "abcd1234", &metadata)
            .await
            .unwrap();

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

        let persistence = JsonHashPersistence::new(&json_file);

        let metadata = ProcessingMetadata {
            file_size: 2048,
            processing_time_ms: 200,
            image_dimensions: (1024, 1024),
            was_resized: true,
        };

        // バッチ保存
        let batch = vec![
            (
                "/test1.jpg".to_string(),
                "hash1".to_string(),
                metadata.clone(),
            ),
            (
                "/test2.png".to_string(),
                "hash2".to_string(),
                metadata.clone(),
            ),
            (
                "/test3.gif".to_string(),
                "hash3".to_string(),
                metadata.clone(),
            ),
        ];

        persistence.store_batch(&batch).await.unwrap();
        persistence.finalize().await.unwrap();

        // ファイル内容確認
        let content = fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();

        assert!(json_value.is_array());
        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), 3);

        let expected_extensions = ["jpg", "png", "gif"];
        for (i, entry) in array.iter().enumerate() {
            let expected_ext = expected_extensions[i];
            assert_eq!(entry["file_path"], format!("/test{}.{expected_ext}", i + 1));
            assert_eq!(entry["hash"], format!("hash{}", i + 1));
            assert_eq!(entry["metadata"]["was_resized"], true);
        }
    }

    #[tokio::test]
    async fn test_json_hash_persistence_multiple_batches() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("multi_batch.json");

        let persistence = JsonHashPersistence::new(&json_file);

        let metadata = ProcessingMetadata {
            file_size: 512,
            processing_time_ms: 100,
            image_dimensions: (256, 256),
            was_resized: false,
        };

        // 複数バッチ保存
        let batch1 = vec![
            (
                "/batch1_1.jpg".to_string(),
                "hash1_1".to_string(),
                metadata.clone(),
            ),
            (
                "/batch1_2.jpg".to_string(),
                "hash1_2".to_string(),
                metadata.clone(),
            ),
        ];

        let batch2 = vec![
            (
                "/batch2_1.jpg".to_string(),
                "hash2_1".to_string(),
                metadata.clone(),
            ),
            (
                "/batch2_2.jpg".to_string(),
                "hash2_2".to_string(),
                metadata.clone(),
            ),
            (
                "/batch2_3.jpg".to_string(),
                "hash2_3".to_string(),
                metadata.clone(),
            ),
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

        let persistence = JsonHashPersistence::new(&json_file);

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
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("directory")
            .join("hashes.json");

        let persistence = JsonHashPersistence::new(&nested_path);

        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };

        persistence
            .store_hash("/test.jpg", "hash", &metadata)
            .await
            .unwrap();
        persistence.finalize().await.unwrap();

        // ファイルが作成されていることを確認
        assert!(nested_path.exists());

        let content = fs::read_to_string(&nested_path).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json_value.as_array().unwrap().len(), 1);
    }
}

/// ストリーミングJSON書き込み対応版（新フォーマット）
/// より効率的なメモリ使用量と高速書き込みを実現
#[derive(Debug, Clone)]
pub struct StreamingJsonHashPersistence {
    file_path: String,
    writer: Arc<AsyncMutex<Option<BufWriter<File>>>>,
    entries_written: Arc<AsyncMutex<usize>>,
    buffer: Arc<AsyncMutex<Vec<(String, String, String, u64, ProcessingMetadata)>>>,
    buffer_size: usize,
    scan_info: Arc<AsyncMutex<Option<ScanInfo>>>,
}

impl StreamingJsonHashPersistence {
    /// 新しいストリーミング永続化インスタンスを作成
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            writer: Arc::new(AsyncMutex::new(None)),
            entries_written: Arc::new(AsyncMutex::new(0)),
            buffer: Arc::new(AsyncMutex::new(Vec::new())),
            buffer_size: 100, // デフォルトバッファサイズ
            scan_info: Arc::new(AsyncMutex::new(None)),
        }
    }

    /// カスタムバッファサイズで作成
    pub fn with_buffer_size<P: AsRef<Path>>(file_path: P, buffer_size: usize) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            writer: Arc::new(AsyncMutex::new(None)),
            entries_written: Arc::new(AsyncMutex::new(0)),
            buffer: Arc::new(AsyncMutex::new(Vec::with_capacity(buffer_size))),
            buffer_size,
            scan_info: Arc::new(AsyncMutex::new(None)),
        }
    }

    /// スキャン情報を設定
    pub async fn set_scan_info(
        &self,
        algorithm: String,
        parameters: serde_json::Value,
    ) -> Result<()> {
        let scan_info = ScanInfo {
            algorithm,
            parameters,
            timestamp: chrono::Utc::now().to_rfc3339(),
            total_files: 0, // 後で更新
        };

        *self.scan_info.lock().await = Some(scan_info);
        Ok(())
    }

    /// JSONファイルのtotal_filesを更新
    async fn update_json_total_files(&self, total: usize) -> Result<()> {
        // JSONファイルを読み込み
        let content = tokio::fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| anyhow::anyhow!("ファイル読み込みエラー: {e}"))?;

        // JSONをパース
        let mut json_value: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("JSON解析エラー: {e}"))?;

        // total_filesを更新
        if let Some(scan_info) = json_value.get_mut("scan_info") {
            if let Some(scan_info_obj) = scan_info.as_object_mut() {
                scan_info_obj.insert(
                    "total_files".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(total)),
                );
            }
        }

        // 更新されたJSONをファイルに書き戻し
        let updated_content = serde_json::to_string_pretty(&json_value)
            .map_err(|e| anyhow::anyhow!("JSON変換エラー: {e}"))?;

        tokio::fs::write(&self.file_path, updated_content)
            .await
            .map_err(|e| anyhow::anyhow!("ファイル書き込みエラー: {e}"))?;

        Ok(())
    }

    /// ファイルを初期化（新しいJSONフォーマット）
    async fn initialize_file(&self) -> Result<()> {
        let mut writer_guard = self.writer.lock().await;
        if writer_guard.is_some() {
            return Ok(());
        }

        // 親ディレクトリ作成
        if let Some(parent) = Path::new(&self.file_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow::anyhow!("ディレクトリ作成エラー: {e}"))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)
            .await
            .map_err(|e| anyhow::anyhow!("ファイル作成エラー: {e}"))?;

        let mut writer = BufWriter::new(file);

        // 新しいJSONオブジェクト形式で開始
        writer
            .write_all(b"{\n")
            .await
            .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;

        *writer_guard = Some(writer);
        Ok(())
    }

    /// バッファをフラッシュ（images配列への書き込み）
    async fn flush_buffer(&self) -> Result<()> {
        let mut buffer_guard = self.buffer.lock().await;
        if buffer_guard.is_empty() {
            return Ok(());
        }

        self.initialize_file().await?;

        let mut writer_guard = self.writer.lock().await;
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("ファイルが初期化されていません"))?;

        let mut entries_written = self.entries_written.lock().await;

        // scan_infoセクションをまだ書いていない場合
        if *entries_written == 0 {
            let scan_info_guard = self.scan_info.lock().await;
            if let Some(scan_info) = scan_info_guard.as_ref() {
                // scan_infoセクションを書き込み
                let scan_info_json = serde_json::to_string_pretty(scan_info)
                    .map_err(|e| anyhow::anyhow!("scan_info JSON変換エラー: {e}"))?;

                // scan_infoを2スペースでインデント
                let indented_scan_info = scan_info_json
                    .lines()
                    .enumerate()
                    .map(|(i, line)| {
                        if i == 0 {
                            line.to_string() // 最初の行はインデントしない
                        } else {
                            format!("  {line}") // 後続行は2スペースでインデント
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                writer
                    .write_all(b"  \"scan_info\": ")
                    .await
                    .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                writer
                    .write_all(indented_scan_info.as_bytes())
                    .await
                    .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                writer
                    .write_all(b",\n  \"images\": [\n")
                    .await
                    .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
            } else {
                return Err(anyhow::anyhow!("scan_infoが設定されていません"));
            }
        }

        for (file_path, hash, _algorithm, hash_bits, metadata) in buffer_guard.drain(..) {
            let entry = HashEntry {
                file_path,
                hash,
                hash_bits,
                metadata,
            };

            // カンマ追加（最初のエントリ以外）
            if *entries_written > 0 {
                writer
                    .write_all(b",\n")
                    .await
                    .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
            }

            // JSON エントリを書き込み（4スペースでインデント）
            let json_str = serde_json::to_string_pretty(&entry)
                .map_err(|e| anyhow::anyhow!("JSON変換エラー: {e}"))?;

            let indented = json_str
                .lines()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n");

            writer
                .write_all(indented.as_bytes())
                .await
                .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;

            *entries_written += 1;
        }

        writer
            .flush()
            .await
            .map_err(|e| anyhow::anyhow!("フラッシュエラー: {e}"))?;

        Ok(())
    }
}

#[async_trait]
impl HashPersistence for StreamingJsonHashPersistence {
    async fn store_hash(
        &self,
        file_path: &str,
        hash: &str,
        metadata: &ProcessingMetadata,
    ) -> Result<()> {
        self.store_batch(&[(
            file_path.to_string(),
            hash.to_string(),
            "DCT".to_string(),
            0u64,
            metadata.clone(),
        )])
        .await
    }

    async fn store_batch(
        &self,
        results: &[(String, String, String, u64, ProcessingMetadata)],
    ) -> Result<()> {
        if results.is_empty() {
            return Ok(());
        }

        let mut buffer_guard = self.buffer.lock().await;

        for (file_path, hash, algorithm, hash_bits, metadata) in results {
            buffer_guard.push((
                file_path.clone(),
                hash.clone(),
                algorithm.clone(),
                *hash_bits,
                metadata.clone(),
            ));
        }

        // バッファがいっぱいになったらフラッシュ
        if buffer_guard.len() >= self.buffer_size {
            drop(buffer_guard); // ロックを先に解放
            self.flush_buffer().await?
        }

        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        // 残りのバッファをフラッシュ
        self.flush_buffer().await?;

        let entries_written = *self.entries_written.lock().await;

        let mut writer_guard = self.writer.lock().await;
        if let Some(mut writer) = writer_guard.take() {
            // images配列を閉じる
            writer
                .write_all(b"\n  ]")
                .await
                .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;

            // JSONオブジェクト終了
            writer
                .write_all(b"\n}")
                .await
                .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;

            writer
                .flush()
                .await
                .map_err(|e| anyhow::anyhow!("フラッシュエラー: {e}"))?;

            // ファイルを閉じた後、JSONを読み込んでtotal_filesを更新
            drop(writer_guard);
            self.update_json_total_files(entries_written).await?;
        } else {
            // ファイルが存在しない場合（何も保存されていない）
            if !tokio::fs::try_exists(&self.file_path)
                .await
                .unwrap_or(false)
            {
                drop(writer_guard);

                self.initialize_file().await?;
                let mut writer_guard = self.writer.lock().await;
                if let Some(mut writer) = writer_guard.take() {
                    // 空のファイルの場合、scan_infoだけ書いて空のimages配列を作成
                    let scan_info_guard = self.scan_info.lock().await;
                    if let Some(scan_info) = scan_info_guard.as_ref() {
                        let scan_info_json = serde_json::to_string_pretty(scan_info)
                            .map_err(|e| anyhow::anyhow!("scan_info JSON変換エラー: {e}"))?;

                        let indented_scan_info = scan_info_json
                            .lines()
                            .enumerate()
                            .map(|(i, line)| {
                                if i == 0 {
                                    line.to_string()
                                } else {
                                    format!("  {line}")
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        writer
                            .write_all(b"  \"scan_info\": ")
                            .await
                            .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                        writer
                            .write_all(indented_scan_info.as_bytes())
                            .await
                            .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                        writer
                            .write_all(b",\n  \"images\": []\n}")
                            .await
                            .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                    } else {
                        writer
                            .write_all(b"  \"scan_info\": null,\n  \"images\": []\n}")
                            .await
                            .map_err(|e| anyhow::anyhow!("書き込みエラー: {e}"))?;
                    }

                    writer
                        .flush()
                        .await
                        .map_err(|e| anyhow::anyhow!("フラッシュエラー: {e}"))?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    use serde_json::Value;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_streaming_json_hash_persistence_basic() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("streaming_test.json");

        let persistence = StreamingJsonHashPersistence::with_buffer_size(&json_file, 2);

        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };

        // 複数のエントリを追加（バッファサイズを超える）
        persistence
            .store_hash("/test1.jpg", "hash1", &metadata)
            .await
            .unwrap();
        persistence
            .store_hash("/test2.png", "hash2", &metadata)
            .await
            .unwrap();
        persistence
            .store_hash("/test3.gif", "hash3", &metadata)
            .await
            .unwrap();

        persistence.finalize().await.unwrap();

        // ファイル内容確認
        let content = tokio::fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();

        assert!(json_value.is_array());
        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), 3);

        for (i, entry) in array.iter().enumerate() {
            let expected_path = format!(
                "/test{}.{}",
                i + 1,
                match i {
                    0 => "jpg",
                    1 => "png",
                    2 => "gif",
                    _ => unreachable!(),
                }
            );
            let expected_hash = format!("hash{}", i + 1);

            assert_eq!(entry["file_path"], expected_path);
            assert_eq!(entry["hash"], expected_hash);
            assert_eq!(entry["metadata"]["file_size"], 1024);
        }
    }

    #[tokio::test]
    async fn test_streaming_batch_processing() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("batch_streaming.json");

        let persistence = StreamingJsonHashPersistence::with_buffer_size(&json_file, 5);

        let metadata = ProcessingMetadata {
            file_size: 2048,
            processing_time_ms: 150,
            image_dimensions: (1024, 1024),
            was_resized: true,
        };

        // 大きなバッチを処理
        let batch: Vec<_> = (0..10)
            .map(|i| {
                (
                    format!("/batch{i:02}.jpg"),
                    format!("batchhash{i:02}"),
                    metadata.clone(),
                )
            })
            .collect();

        persistence.store_batch(&batch).await.unwrap();
        persistence.finalize().await.unwrap();

        let content = tokio::fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();
        let array = json_value.as_array().unwrap();

        assert_eq!(array.len(), 10);

        for (i, entry) in array.iter().enumerate() {
            assert_eq!(entry["file_path"], format!("/batch{i:02}.jpg"));
            assert_eq!(entry["hash"], format!("batchhash{i:02}"));
            assert_eq!(entry["metadata"]["file_size"], 2048);
        }
    }

    #[tokio::test]
    async fn test_streaming_empty_finalize() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("empty_streaming.json");

        let persistence = StreamingJsonHashPersistence::new(&json_file);
        persistence.finalize().await.unwrap();

        let content = tokio::fs::read_to_string(&json_file).await.unwrap();
        let json_value: Value = serde_json::from_str(&content).unwrap();

        assert!(json_value.is_array());
        assert_eq!(json_value.as_array().unwrap().len(), 0);
    }
}
