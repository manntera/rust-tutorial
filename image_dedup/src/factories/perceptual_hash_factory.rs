//! PerceptualHashFactory - パーセプチュアルハッシュの Factory Pattern 実装

use super::{ComponentConfig, ComponentFactory};
use crate::perceptual_hash::{
    average_config::AverageConfig, config::AlgorithmConfig, dct_config::DctConfig,
    difference_config::DifferenceConfig, PerceptualHashBackend,
};
use anyhow::Result;

pub struct PerceptualHashFactory;

impl PerceptualHashFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PerceptualHashFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentFactory<Box<dyn PerceptualHashBackend>> for PerceptualHashFactory {
    fn create(&self, config: &ComponentConfig) -> Result<Box<dyn PerceptualHashBackend>> {
        match config.implementation.as_str() {
            "dct" => {
                let size = config
                    .parameters
                    .get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8) as u32;

                let quality_factor = config
                    .parameters
                    .get("quality_factor")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;

                let dct_config = DctConfig {
                    size,
                    quality_factor,
                };

                dct_config.validate()?;
                let hasher = dct_config.create_hasher()?;
                Ok(Box::new(hasher))
            }
            "average" => {
                let size = config
                    .parameters
                    .get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8) as u32;

                let avg_config = AverageConfig { size };
                avg_config.validate()?;
                let hasher = avg_config.create_hasher()?;
                Ok(Box::new(hasher))
            }
            "difference" => {
                let size = config
                    .parameters
                    .get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8) as u32;

                let diff_config = DifferenceConfig { size };
                diff_config.validate()?;
                let hasher = diff_config.create_hasher()?;
                Ok(Box::new(hasher))
            }
            _ => anyhow::bail!(
                "未サポートのPerceptualHash実装: {}. 利用可能: dct, average, difference",
                config.implementation
            ),
        }
    }

    fn available_implementations(&self) -> Vec<String> {
        vec![
            "dct".to_string(),
            "average".to_string(),
            "difference".to_string(),
        ]
    }

    fn get_description(&self, implementation: &str) -> Option<String> {
        match implementation {
            "dct" => Some("DCTベースのパーセプチュアルハッシュ (高精度)".to_string()),
            "average" => Some("平均値ベースのパーセプチュアルハッシュ (高速)".to_string()),
            "difference" => Some("差分ベースのパーセプチュアルハッシュ (シンプル)".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_dct_hasher() {
        let factory = PerceptualHashFactory::new();
        let config = ComponentConfig::new(
            "dct",
            json!({
                "size": 16,
                "quality_factor": 0.8
            }),
        );

        let hasher = factory.create(&config);
        assert!(hasher.is_ok());
    }

    #[test]
    fn test_create_average_hasher() {
        let factory = PerceptualHashFactory::new();
        let config = ComponentConfig::new(
            "average",
            json!({
                "size": 8
            }),
        );

        let hasher = factory.create(&config);
        assert!(hasher.is_ok());
    }

    #[test]
    fn test_create_difference_hasher() {
        let factory = PerceptualHashFactory::new();
        let config = ComponentConfig::new(
            "difference",
            json!({
                "size": 8
            }),
        );

        let hasher = factory.create(&config);
        assert!(hasher.is_ok());
    }

    #[test]
    fn test_create_with_default_params() {
        let factory = PerceptualHashFactory::new();

        // DCTでデフォルトパラメータ
        let config = ComponentConfig::new("dct", json!({}));
        let hasher = factory.create(&config);
        assert!(hasher.is_ok());

        // Averageでデフォルトパラメータ
        let config = ComponentConfig::new("average", json!({}));
        let hasher = factory.create(&config);
        assert!(hasher.is_ok());
    }

    #[test]
    fn test_unsupported_implementation() {
        let factory = PerceptualHashFactory::new();
        let config = ComponentConfig::new("unsupported", json!({}));

        let result = factory.create(&config);
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.to_string().contains("未サポートのPerceptualHash実装"));
        }
    }

    #[test]
    fn test_available_implementations() {
        let factory = PerceptualHashFactory::new();
        let implementations = factory.available_implementations();

        assert_eq!(implementations, vec!["dct", "average", "difference"]);
    }

    #[test]
    fn test_get_description() {
        let factory = PerceptualHashFactory::new();

        assert!(factory.get_description("dct").is_some());
        assert!(factory.get_description("average").is_some());
        assert!(factory.get_description("difference").is_some());
        assert!(factory.get_description("unknown").is_none());
    }
}
