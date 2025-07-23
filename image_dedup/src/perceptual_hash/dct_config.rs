// DCTアルゴリズムの設定

use super::config::{AlgorithmConfig, ParameterInfo, ParameterType};
use super::dct_hash::DctHasher;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// DCTハッシュの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DctConfig {
    /// ハッシュサイズ（通常は8, 16, 32など）
    pub size: u32,
    /// 品質係数（オプション、将来の拡張用）
    #[serde(default = "default_quality_factor")]
    pub quality_factor: f32,
}

fn default_quality_factor() -> f32 {
    1.0
}

impl AlgorithmConfig for DctConfig {
    type Algorithm = DctHasher;

    fn create_hasher(&self) -> Result<Self::Algorithm> {
        Ok(DctHasher::with_quality_factor(self.size, self.quality_factor))
    }

    fn algorithm_name(&self) -> &'static str {
        "dct"
    }

    fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| anyhow::anyhow!("JSON変換エラー: {}", e))
    }

    fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| anyhow::anyhow!("JSON解析エラー: {}", e))
    }

    fn description(&self) -> &'static str {
        "DCT (Discrete Cosine Transform) based perceptual hash. High accuracy but computationally expensive."
    }

    fn default_config() -> Self {
        Self {
            size: 8,
            quality_factor: 1.0,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.size == 0 {
            anyhow::bail!("Size must be greater than 0");
        }

        if self.size > 64 {
            anyhow::bail!("Size must be 64 or less for performance reasons");
        }

        if !(self.quality_factor > 0.0 && self.quality_factor <= 1.0) {
            anyhow::bail!("Quality factor must be between 0.0 and 1.0");
        }

        Ok(())
    }

    fn parameter_info() -> Vec<ParameterInfo> {
        vec![
            ParameterInfo {
                name: "size".to_string(),
                param_type: ParameterType::Integer {
                    min: Some(1),
                    max: Some(64),
                },
                description: "Hash size (typically 8, 16, or 32)".to_string(),
                default_value: Some("8".to_string()),
                required: true,
            },
            ParameterInfo {
                name: "quality_factor".to_string(),
                param_type: ParameterType::Float {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                description: "Quality factor for hash generation (0.0-1.0)".to_string(),
                default_value: Some("1.0".to_string()),
                required: false,
            },
        ]
    }
}

impl Default for DctConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perceptual_hash::PerceptualHashBackend;

    #[test]
    fn test_dct_config_creation() {
        let config = DctConfig::default();
        assert_eq!(config.size, 8);
        assert_eq!(config.quality_factor, 1.0);
        assert_eq!(config.algorithm_name(), "dct");
    }

    #[test]
    fn test_dct_config_validation() {
        // 有効な設定
        let valid_config = DctConfig {
            size: 16,
            quality_factor: 0.8,
        };
        assert!(valid_config.validate().is_ok());

        // 無効なサイズ
        let invalid_size = DctConfig {
            size: 0,
            quality_factor: 1.0,
        };
        assert!(invalid_size.validate().is_err());

        let too_large_size = DctConfig {
            size: 128,
            quality_factor: 1.0,
        };
        assert!(too_large_size.validate().is_err());

        // 無効な品質係数
        let invalid_quality = DctConfig {
            size: 8,
            quality_factor: 1.5,
        };
        assert!(invalid_quality.validate().is_err());

        let zero_quality = DctConfig {
            size: 8,
            quality_factor: 0.0,
        };
        assert!(zero_quality.validate().is_err());
    }

    #[test]
    fn test_dct_config_json_serialization() {
        let config = DctConfig {
            size: 16,
            quality_factor: 0.9,
        };

        let json = config.to_json().unwrap();
        assert!(json.contains("\"size\": 16"));
        assert!(json.contains("\"quality_factor\": 0.9"));

        let deserialized = DctConfig::from_json(&json).unwrap();
        assert_eq!(deserialized.size, 16);
        assert_eq!(deserialized.quality_factor, 0.9);
    }

    #[tokio::test]
    async fn test_dct_config_hasher_creation() {
        let config = DctConfig {
            size: 8,
            quality_factor: 1.0,
        };
        let hasher = config.create_hasher().unwrap();

        assert_eq!(hasher.algorithm_name(), "DCT (Discrete Cosine Transform)");

        // 簡単な画像でテスト
        use image::DynamicImage;
        let test_image = DynamicImage::new_rgb8(64, 64);
        let result = hasher.generate_hash(&test_image).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_dct_config_parameter_info() {
        let params = DctConfig::parameter_info();
        assert_eq!(params.len(), 2);

        let size_param = &params[0];
        assert_eq!(size_param.name, "size");
        assert!(size_param.required);

        let quality_param = &params[1];
        assert_eq!(quality_param.name, "quality_factor");
        assert!(!quality_param.required);
    }

    #[test]
    fn test_dct_config_serde_with_defaults() {
        // quality_factorを省略したJSON
        let json_without_quality = r#"{"size": 16}"#;
        let config = DctConfig::from_json(json_without_quality).unwrap();

        assert_eq!(config.size, 16);
        assert_eq!(config.quality_factor, 1.0); // デフォルト値
    }
}
