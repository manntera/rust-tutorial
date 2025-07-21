// テスト用の永続化モック実装

use super::traits::HashPersistence;
use crate::processing::types::ProcessingMetadata;

pub struct MockHashPersistence {
    pub stored_hashes: std::sync::Arc<std::sync::Mutex<Vec<(String, String, ProcessingMetadata)>>>,
    pub batch_calls: std::sync::Arc<std::sync::Mutex<usize>>,
    pub finalize_called: std::sync::Arc<std::sync::Mutex<bool>>,
}

impl MockHashPersistence {
    pub fn new() -> Self {
        Self {
            stored_hashes: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            batch_calls: std::sync::Arc::new(std::sync::Mutex::new(0)),
            finalize_called: std::sync::Arc::new(std::sync::Mutex::new(false)),
        }
    }
}

#[async_trait::async_trait]
impl HashPersistence for MockHashPersistence {
    async fn store_hash(&self, file_path: &str, hash: &str, metadata: &ProcessingMetadata) -> anyhow::Result<()> {
        self.stored_hashes.lock().unwrap().push((
            file_path.to_string(),
            hash.to_string(),
            metadata.clone(),
        ));
        Ok(())
    }
    
    async fn store_batch(&self, results: &[(String, String, ProcessingMetadata)]) -> anyhow::Result<()> {
        self.stored_hashes.lock().unwrap().extend(results.iter().cloned());
        *self.batch_calls.lock().unwrap() += 1;
        Ok(())
    }
    
    async fn finalize(&self) -> anyhow::Result<()> {
        *self.finalize_called.lock().unwrap() = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_persistence_trait() {
        let persistence = MockHashPersistence::new();
        
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 150,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // Test single hash storage
        persistence.store_hash("/path/file1.jpg", "hash123", &metadata).await.unwrap();
        
        let stored = persistence.stored_hashes.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, "/path/file1.jpg");
        assert_eq!(stored[0].1, "hash123");
        assert_eq!(stored[0].2, metadata);
        
        drop(stored);
        
        // Test batch storage
        let batch_data = vec![
            ("file2.jpg".to_string(), "hash456".to_string(), metadata.clone()),
            ("file3.jpg".to_string(), "hash789".to_string(), metadata.clone()),
        ];
        
        persistence.store_batch(&batch_data).await.unwrap();
        
        let stored = persistence.stored_hashes.lock().unwrap();
        assert_eq!(stored.len(), 3);
        assert_eq!(*persistence.batch_calls.lock().unwrap(), 1);
        
        drop(stored);
        
        // Test finalize
        persistence.finalize().await.unwrap();
        assert!(*persistence.finalize_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_hash_persistence_thread_safety() {
        let persistence = MockHashPersistence::new();
        let persistence_ref: &dyn HashPersistence = &persistence;
        
        let metadata = ProcessingMetadata {
            file_size: 2048,
            processing_time_ms: 200,
            image_dimensions: (1024, 768),
            was_resized: true,
        };
        
        persistence_ref.store_hash("test.jpg", "testhash", &metadata).await.unwrap();
        
        let batch = vec![("batch.jpg".to_string(), "batchhash".to_string(), metadata)];
        persistence_ref.store_batch(&batch).await.unwrap();
        
        persistence_ref.finalize().await.unwrap();
        
        // Verify all operations completed
        assert_eq!(persistence.stored_hashes.lock().unwrap().len(), 2);
        assert_eq!(*persistence.batch_calls.lock().unwrap(), 1);
        assert!(*persistence.finalize_called.lock().unwrap());
    }
}