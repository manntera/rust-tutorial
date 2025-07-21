// テスト用の設定モック実装

use super::traits::ProcessingConfig;

pub struct MockProcessingConfig {
    pub max_concurrent: usize,
    pub buffer_size: usize,
    pub batch_size: usize,
    pub enable_progress: bool,
}

impl ProcessingConfig for MockProcessingConfig {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_config_trait() {
        let config = MockProcessingConfig {
            max_concurrent: 8,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        };
        
        assert_eq!(config.max_concurrent_tasks(), 8);
        assert_eq!(config.channel_buffer_size(), 100);
        assert_eq!(config.batch_size(), 50);
        assert!(config.enable_progress_reporting());
    }

    #[test]
    fn test_processing_config_thread_safety() {
        let config = MockProcessingConfig {
            max_concurrent: 4,
            buffer_size: 50,
            batch_size: 25,
            enable_progress: false,
        };
        
        // Test that config can be shared across threads
        let config_ref: &dyn ProcessingConfig = &config;
        assert_eq!(config_ref.max_concurrent_tasks(), 4);
        assert_eq!(config_ref.channel_buffer_size(), 50);
        assert_eq!(config_ref.batch_size(), 25);
        assert!(!config_ref.enable_progress_reporting());
    }
}