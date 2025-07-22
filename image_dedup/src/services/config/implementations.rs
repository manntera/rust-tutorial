// 設定管理の具象実装

use crate::core::ProcessingConfig;

/// デフォルト設定実装
#[derive(Debug, Clone)]
pub struct DefaultProcessingConfig {
    max_concurrent: usize,
    buffer_size: usize,
    batch_size: usize,
    enable_progress: bool,
}

impl DefaultProcessingConfig {
    pub fn new(cpu_count: usize) -> Self {
        Self {
            max_concurrent: cpu_count.max(1) * 2,
            buffer_size: 100,
            batch_size: 50,
            enable_progress: true,
        }
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
        let config = DefaultProcessingConfig::new(4)
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