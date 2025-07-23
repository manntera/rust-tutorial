// アルゴリズム作成ファクトリー

use super::config::{AlgorithmRegistry, DynamicAlgorithmConfig, AlgorithmConfig};
use super::{PerceptualHashBackend, dct_config::DctConfig, average_config::AverageConfig, difference_config::DifferenceConfig};
use anyhow::Result;

/// アルゴリズムファクトリー
pub struct AlgorithmFactory {
    registry: AlgorithmRegistry,
}

impl AlgorithmFactory {
    /// 新しいファクトリーを作成
    pub fn new() -> Self {
        let mut registry = AlgorithmRegistry::new();
        
        // デフォルトアルゴリズムを登録
        registry.register::<DctConfig>();
        registry.register::<AverageConfig>();
        registry.register::<DifferenceConfig>();
        
        Self { registry }
    }
    
    /// 設定からハッシュアルゴリズムを作成
    pub fn create_hasher(&self, config: &DynamicAlgorithmConfig) -> Result<Box<dyn PerceptualHashBackend>> {
        self.registry.create_hasher(config)
    }
    
    /// アルゴリズム名からデフォルト設定でハッシュアルゴリズムを作成
    pub fn create_hasher_by_name(&self, algorithm: &str) -> Result<Box<dyn PerceptualHashBackend>> {
        match algorithm {
            "dct" => {
                let config = DctConfig::default();
                Ok(Box::new(config.create_hasher()?))
            }
            "average" => {
                let config = AverageConfig::default();
                Ok(Box::new(config.create_hasher()?))
            }
            "difference" => {
                let config = DifferenceConfig::default();
                Ok(Box::new(config.create_hasher()?))
            }
            _ => anyhow::bail!("Unknown algorithm: {}", algorithm),
        }
    }
    
    /// 利用可能なアルゴリズムの一覧を取得
    pub fn available_algorithms(&self) -> Vec<String> {
        self.registry.available_algorithms()
    }
    
    /// アルゴリズムの説明を取得
    pub fn get_description(&self, algorithm: &str) -> Option<&String> {
        self.registry.get_description(algorithm)
    }
    
    /// JSON設定からハッシュアルゴリズムを作成
    pub fn create_hasher_from_json(&self, json: &str) -> Result<Box<dyn PerceptualHashBackend>> {
        let config: DynamicAlgorithmConfig = serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("JSON解析エラー: {}", e))?;
        
        self.create_hasher(&config)
    }
}

impl Default for AlgorithmFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルファクトリーのシングルトン
static GLOBAL_FACTORY: std::sync::OnceLock<AlgorithmFactory> = std::sync::OnceLock::new();

/// グローバルアルゴリズムファクトリーを取得
pub fn get_algorithm_factory() -> &'static AlgorithmFactory {
    GLOBAL_FACTORY.get_or_init(AlgorithmFactory::new)
}

/// 便利関数：アルゴリズム名からハッシャーを作成
pub fn create_hasher(algorithm: &str) -> Result<Box<dyn PerceptualHashBackend>> {
    get_algorithm_factory().create_hasher_by_name(algorithm)
}

/// 便利関数：JSON設定からハッシャーを作成
pub fn create_hasher_from_json(json: &str) -> Result<Box<dyn PerceptualHashBackend>> {
    get_algorithm_factory().create_hasher_from_json(json)
}

/// 便利関数：設定からハッシャーを作成
pub fn create_hasher_from_config(config: &DynamicAlgorithmConfig) -> Result<Box<dyn PerceptualHashBackend>> {
    get_algorithm_factory().create_hasher(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_algorithm_factory_creation() {
        let factory = AlgorithmFactory::new();
        let algorithms = factory.available_algorithms();
        
        assert!(algorithms.contains(&"dct".to_string()));
        assert!(algorithms.contains(&"average".to_string()));
        assert!(algorithms.contains(&"difference".to_string()));
    }
    
    #[test]
    fn test_create_hasher_by_name() {
        let factory = AlgorithmFactory::new();
        
        // DCTハッシャー作成
        let dct_hasher = factory.create_hasher_by_name("dct");
        assert!(dct_hasher.is_ok());
        
        // Averageハッシャー作成
        let avg_hasher = factory.create_hasher_by_name("average");
        assert!(avg_hasher.is_ok());
        
        // Differenceハッシャー作成
        let diff_hasher = factory.create_hasher_by_name("difference");
        assert!(diff_hasher.is_ok());
        
        // 存在しないアルゴリズム
        let unknown_hasher = factory.create_hasher_by_name("unknown");
        assert!(unknown_hasher.is_err());
    }
    
    #[test]
    fn test_create_hasher_from_config() {
        let factory = AlgorithmFactory::new();
        
        let config = DynamicAlgorithmConfig::new(
            "dct",
            json!({
                "size": 16,
                "quality_factor": 0.9
            })
        );
        
        let hasher = factory.create_hasher(&config);
        assert!(hasher.is_ok());
    }
    
    #[test]
    fn test_create_hasher_from_json() {
        let factory = AlgorithmFactory::new();
        
        let json = r#"{
            "algorithm": "average",
            "parameters": {
                "size": 8
            }
        }"#;
        
        let hasher = factory.create_hasher_from_json(json);
        assert!(hasher.is_ok());
    }
    
    #[test]
    fn test_global_factory() {
        let factory = get_algorithm_factory();
        let algorithms = factory.available_algorithms();
        assert!(!algorithms.is_empty());
    }
    
    #[test]
    fn test_convenience_functions() {
        // アルゴリズム名から作成
        let hasher1 = create_hasher("dct");
        assert!(hasher1.is_ok());
        
        // JSON設定から作成
        let json = r#"{
            "algorithm": "average",
            "parameters": {
                "size": 16
            }
        }"#;
        let hasher2 = create_hasher_from_json(json);
        assert!(hasher2.is_ok());
        
        // 設定オブジェクトから作成
        let config = DynamicAlgorithmConfig::new("difference", json!({"size": 8}));
        let hasher3 = create_hasher_from_config(&config);
        assert!(hasher3.is_ok());
    }
}