// 処理に関連するデータ型定義

/// 処理時のメタデータ
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProcessingMetadata {
    pub file_size: u64,
    pub processing_time_ms: u64,
    pub image_dimensions: (u32, u32),
    pub was_resized: bool,
}

/// 処理全体のサマリー
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessingSummary {
    pub total_files: usize,
    pub processed_files: usize,
    pub error_count: usize,
    pub total_processing_time_ms: u64,
    pub average_time_per_file_ms: f64,
}

/// 個別処理の結果
#[derive(Debug)]
pub enum ProcessingOutcome {
    Success {
        file_path: String,
        hash: String,
        algorithm: String,
        hash_bits: u64,
        metadata: ProcessingMetadata,
    },
    Error {
        file_path: String,
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_metadata_creation() {
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 150,
            image_dimensions: (512, 512),
            was_resized: false,
        };

        assert_eq!(metadata.file_size, 1024);
        assert_eq!(metadata.processing_time_ms, 150);
        assert_eq!(metadata.image_dimensions, (512, 512));
        assert!(!metadata.was_resized);
    }

    #[test]
    fn test_processing_summary_creation() {
        let summary = ProcessingSummary {
            total_files: 100,
            processed_files: 95,
            error_count: 5,
            total_processing_time_ms: 30000,
            average_time_per_file_ms: 315.79,
        };

        assert_eq!(summary.total_files, 100);
        assert_eq!(summary.processed_files, 95);
        assert_eq!(summary.error_count, 5);
        assert_eq!(summary.total_processing_time_ms, 30000);
        assert!((summary.average_time_per_file_ms - 315.79).abs() < 0.01);
    }

    #[test]
    fn test_processing_result_success() {
        let metadata = ProcessingMetadata {
            file_size: 2048,
            processing_time_ms: 200,
            image_dimensions: (1024, 1024),
            was_resized: true,
        };

        let result = ProcessingOutcome::Success {
            file_path: "/test/image.jpg".to_string(),
            hash: "abcd1234".to_string(),
            algorithm: "DCT".to_string(),
            hash_bits: 0x12345678,
            metadata,
        };

        match result {
            ProcessingOutcome::Success {
                file_path,
                hash,
                algorithm: _,
                hash_bits: _,
                metadata,
            } => {
                assert_eq!(file_path, "/test/image.jpg");
                assert_eq!(hash, "abcd1234");
                assert_eq!(metadata.file_size, 2048);
                assert!(metadata.was_resized);
            }
            ProcessingOutcome::Error { .. } => panic!("Expected Success variant"),
        }
    }

    #[test]
    fn test_processing_result_error() {
        let result = ProcessingOutcome::Error {
            file_path: "/test/invalid.jpg".to_string(),
            error: "Failed to load image".to_string(),
        };

        match result {
            ProcessingOutcome::Success { .. } => panic!("Expected Error variant"),
            ProcessingOutcome::Error { file_path, error } => {
                assert_eq!(file_path, "/test/invalid.jpg");
                assert_eq!(error, "Failed to load image");
            }
        }
    }

    #[test]
    fn test_processing_metadata_debug() {
        let metadata = ProcessingMetadata {
            file_size: 1024,
            processing_time_ms: 150,
            image_dimensions: (512, 512),
            was_resized: false,
        };

        let debug_str = format!("{metadata:?}");
        assert!(debug_str.contains("file_size: 1024"));
        assert!(debug_str.contains("processing_time_ms: 150"));
        assert!(debug_str.contains("image_dimensions: (512, 512)"));
        assert!(debug_str.contains("was_resized: false"));
    }
}
