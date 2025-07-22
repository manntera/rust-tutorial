#[cfg(test)]
mod double_finalize_test {
    use crate::processing::data_persistence::StreamingJsonHashPersistence;
    use crate::processing::traits::HashPersistence;
    use crate::processing::types::ProcessingMetadata;
    use tempfile::TempDir;
    use serde_json::Value;

    #[tokio::test]
    async fn test_double_finalize_bug_fix() {
        // This test reproduces the original bug scenario:
        // finalize() being called twice, which was overwriting the JSON file
        
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("test_double_finalize.json");
        
        let persistence = StreamingJsonHashPersistence::new(&json_file);
        
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 100,
            image_dimensions: (512, 512),
            was_resized: false,
        };
        
        // Add some test data
        persistence.store_hash("/test1.jpg", "hash1", &metadata).await.unwrap();
        persistence.store_hash("/test2.jpg", "hash2", &metadata).await.unwrap();
        
        // First finalize (from pipeline.execute())
        persistence.finalize().await.unwrap();
        
        // Read content after first finalize
        let content_after_first = tokio::fs::read_to_string(&json_file).await.unwrap();
        let json_after_first: Value = serde_json::from_str(&content_after_first).unwrap();
        let array_after_first = json_after_first.as_array().unwrap();
        
        // Should have 2 entries after first finalize
        assert_eq!(array_after_first.len(), 2);
        
        // Second finalize (from engine.process_directory_with_config())
        // This used to overwrite the file with empty array - should be safe now
        persistence.finalize().await.unwrap();
        
        // Read content after second finalize
        let content_after_second = tokio::fs::read_to_string(&json_file).await.unwrap();
        let json_after_second: Value = serde_json::from_str(&content_after_second).unwrap();
        let array_after_second = json_after_second.as_array().unwrap();
        
        // CRITICAL: Should still have 2 entries after second finalize
        // Before the fix, this would be 0 (empty array)
        assert_eq!(array_after_second.len(), 2, 
                   "Double finalize should not overwrite existing data!");
        
        // Verify the actual data is preserved
        assert_eq!(array_after_second[0]["file_path"], "/test1.jpg");
        assert_eq!(array_after_second[0]["hash"], "hash1");
        assert_eq!(array_after_second[1]["file_path"], "/test2.jpg");
        assert_eq!(array_after_second[1]["hash"], "hash2");
        
        println!("✅ Double finalize bug fixed: Data preserved after second finalize call!");
    }
}