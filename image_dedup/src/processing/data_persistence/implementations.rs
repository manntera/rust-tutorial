// データ永続化の具象実装

use super::super::traits::HashPersistence;
use super::super::types::ProcessingMetadata;
use async_trait::async_trait;
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