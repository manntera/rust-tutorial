//! ImageLoaderFactory - 画像ローダーの Factory Pattern 実装

use super::{ComponentConfig, ComponentFactory};
use crate::image_loader::{standard::StandardImageLoader, ImageLoaderBackend};
use anyhow::Result;

pub struct ImageLoaderFactory;

impl ImageLoaderFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImageLoaderFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentFactory<Box<dyn ImageLoaderBackend>> for ImageLoaderFactory {
    fn create(&self, config: &ComponentConfig) -> Result<Box<dyn ImageLoaderBackend>> {
        match config.implementation.as_str() {
            "standard" => {
                let max_dimension = config
                    .parameters
                    .get("max_dimension")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(512) as u32;

                Ok(Box::new(StandardImageLoader::with_max_dimension(
                    max_dimension,
                )))
            }
            _ => anyhow::bail!(
                "未サポートのImageLoader実装: {}. 利用可能: standard",
                config.implementation
            ),
        }
    }

    fn available_implementations(&self) -> Vec<String> {
        vec!["standard".to_string()]
    }

    fn get_description(&self, implementation: &str) -> Option<String> {
        match implementation {
            "standard" => Some("標準的な画像ローダー (最大次元数制限付き)".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_standard_image_loader() {
        let factory = ImageLoaderFactory::new();
        let config = ComponentConfig::new(
            "standard",
            json!({
                "max_dimension": 1024
            }),
        );

        let loader = factory.create(&config);
        assert!(loader.is_ok());
    }

    #[test]
    fn test_create_standard_image_loader_with_default_params() {
        let factory = ImageLoaderFactory::new();
        let config = ComponentConfig::new("standard", json!({}));

        let loader = factory.create(&config);
        assert!(loader.is_ok());
    }

    #[test]
    fn test_unsupported_implementation() {
        let factory = ImageLoaderFactory::new();
        let config = ComponentConfig::new("unsupported", json!({}));

        let result = factory.create(&config);
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.to_string().contains("未サポートのImageLoader実装"));
        }
    }

    #[test]
    fn test_available_implementations() {
        let factory = ImageLoaderFactory::new();
        let implementations = factory.available_implementations();

        assert_eq!(implementations, vec!["standard"]);
    }

    #[test]
    fn test_get_description() {
        let factory = ImageLoaderFactory::new();

        assert!(factory.get_description("standard").is_some());
        assert!(factory.get_description("unknown").is_none());
    }
}
