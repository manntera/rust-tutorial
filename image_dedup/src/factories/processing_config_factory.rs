//! ProcessingConfigFactory - 処理設定の Factory Pattern 実装

use super::{ComponentConfig, ComponentFactory};
use crate::core::ProcessingConfig;
use crate::services::DefaultProcessingConfig;
use anyhow::Result;

pub struct ProcessingConfigFactory;

impl ProcessingConfigFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProcessingConfigFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentFactory<Box<dyn ProcessingConfig>> for ProcessingConfigFactory {
    fn create(&self, config: &ComponentConfig) -> Result<Box<dyn ProcessingConfig>> {
        match config.implementation.as_str() {
            "default" => {
                let max_concurrent = config
                    .parameters
                    .get("max_concurrent")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or_else(|| num_cpus::get().max(1) * 2);

                let buffer_size = config
                    .parameters
                    .get("buffer_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                let batch_size = config
                    .parameters
                    .get("batch_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50) as usize;

                let enable_progress = config
                    .parameters
                    .get("enable_progress")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let processing_config = DefaultProcessingConfig::new(num_cpus::get())
                    .with_max_concurrent(max_concurrent)
                    .with_buffer_size(buffer_size)
                    .with_batch_size(batch_size)
                    .with_progress_reporting(enable_progress);

                Ok(Box::new(processing_config))
            }
            _ => anyhow::bail!(
                "未サポートのProcessingConfig実装: {}. 利用可能: default",
                config.implementation
            ),
        }
    }

    fn available_implementations(&self) -> Vec<String> {
        vec!["default".to_string()]
    }

    fn get_description(&self, implementation: &str) -> Option<String> {
        match implementation {
            "default" => Some("デフォルトの処理設定 (CPU数ベースの自動調整)".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_default_processing_config() {
        let factory = ProcessingConfigFactory::new();
        let config = ComponentConfig::new(
            "default",
            json!({
                "max_concurrent": 4,
                "buffer_size": 200,
                "batch_size": 25,
                "enable_progress": false
            }),
        );

        let processing_config = factory.create(&config);
        assert!(processing_config.is_ok());

        let config_instance = processing_config.unwrap();
        assert_eq!(config_instance.max_concurrent_tasks(), 4);
        assert_eq!(config_instance.channel_buffer_size(), 200);
        assert_eq!(config_instance.batch_size(), 25);
        assert!(!config_instance.enable_progress_reporting());
    }

    #[test]
    fn test_create_with_default_params() {
        let factory = ProcessingConfigFactory::new();
        let config = ComponentConfig::new("default", json!({}));

        let processing_config = factory.create(&config);
        assert!(processing_config.is_ok());

        let config_instance = processing_config.unwrap();
        assert!(config_instance.max_concurrent_tasks() > 0);
        assert_eq!(config_instance.channel_buffer_size(), 100);
        assert_eq!(config_instance.batch_size(), 50);
        assert!(config_instance.enable_progress_reporting());
    }

    #[test]
    fn test_unsupported_implementation() {
        let factory = ProcessingConfigFactory::new();
        let config = ComponentConfig::new("unsupported", json!({}));

        let result = factory.create(&config);
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error
                .to_string()
                .contains("未サポートのProcessingConfig実装"));
        }
    }

    #[test]
    fn test_available_implementations() {
        let factory = ProcessingConfigFactory::new();
        let implementations = factory.available_implementations();

        assert_eq!(implementations, vec!["default"]);
    }

    #[test]
    fn test_get_description() {
        let factory = ProcessingConfigFactory::new();

        assert!(factory.get_description("default").is_some());
        assert!(factory.get_description("unknown").is_none());
    }
}
