// Differenceアルゴリズムの設定

use super::config::{AlgorithmConfig, ParameterInfo, ParameterType};
use super::average_hash::DifferenceHasher;
use super::PerceptualHashBackend;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Differenceハッシュの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferenceConfig {
    /// ハッシュサイズ（通常は8, 16, 32など）
    pub size: u32,
}

impl AlgorithmConfig for DifferenceConfig {
    type Algorithm = DifferenceHasher;
    
    fn create_hasher(&self) -> Result<Self::Algorithm> {
        Ok(DifferenceHasher::new(self.size))
    }
    
    fn algorithm_name(&self) -> &'static str {
        "difference"
    }
    
    fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("JSON変換エラー: {}", e))
    }
    
    fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("JSON解析エラー: {}", e))
    }
    
    fn description(&self) -> &'static str {
        "Difference Hash - Based on adjacent pixel differences. Good for detecting structural changes and edge patterns."
    }
    
    fn default_config() -> Self {
        Self {
            size: 8,
        }
    }
    
    fn validate(&self) -> Result<()> {
        if self.size == 0 {
            anyhow::bail!("Size must be greater than 0");
        }
        
        if self.size > 64 {
            anyhow::bail!("Size must be 64 or less for performance reasons");
        }
        
        Ok(())
    }
    
    fn parameter_info() -> Vec<ParameterInfo> {
        vec![
            ParameterInfo {
                name: "size".to_string(),
                param_type: ParameterType::Integer { min: Some(1), max: Some(64) },
                description: "Hash size (typically 8, 16, or 32)".to_string(),
                default_value: Some("8".to_string()),
                required: true,
            },
        ]
    }
}

impl Default for DifferenceConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_difference_config_creation() {
        let config = DifferenceConfig::default();
        assert_eq!(config.size, 8);
        assert_eq!(config.algorithm_name(), "difference");
    }
    
    #[test]
    fn test_difference_config_validation() {
        // 有効な設定
        let valid_config = DifferenceConfig { size: 16 };
        assert!(valid_config.validate().is_ok());
        
        // 無効なサイズ
        let invalid_size = DifferenceConfig { size: 0 };
        assert!(invalid_size.validate().is_err());
        
        let too_large_size = DifferenceConfig { size: 128 };
        assert!(too_large_size.validate().is_err());
    }
    
    #[test]
    fn test_difference_config_json_serialization() {
        let config = DifferenceConfig { size: 16 };
        
        let json = config.to_json().unwrap();
        assert!(json.contains("\"size\": 16"));
        
        let deserialized = DifferenceConfig::from_json(&json).unwrap();
        assert_eq!(deserialized.size, 16);
    }
    
    #[tokio::test]
    async fn test_difference_config_hasher_creation() {
        let config = DifferenceConfig { size: 8 };
        let hasher = config.create_hasher().unwrap();
        
        assert_eq!(hasher.algorithm_name(), "Difference Hash");
        
        // 簡単な画像でテスト
        use image::DynamicImage;
        let test_image = DynamicImage::new_rgb8(64, 64);
        let result = hasher.generate_hash(&test_image).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_difference_config_parameter_info() {
        let params = DifferenceConfig::parameter_info();
        assert_eq!(params.len(), 1);
        
        let size_param = &params[0];
        assert_eq!(size_param.name, "size");
        assert!(size_param.required);
        assert_eq!(size_param.default_value, Some("8".to_string()));
    }
}