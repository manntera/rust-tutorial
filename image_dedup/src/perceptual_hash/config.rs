// アルゴリズム設定の抽象化

use crate::perceptual_hash::PerceptualHashBackend;
use anyhow::Result;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;

/// アルゴリズム設定の抽象化トレイト
pub trait AlgorithmConfig: Send + Sync + Clone {
    type Algorithm: PerceptualHashBackend;

    /// 設定からハッシュアルゴリズムを作成
    fn create_hasher(&self) -> Result<Self::Algorithm>;

    /// アルゴリズム名を取得
    fn algorithm_name(&self) -> &'static str;

    /// 設定をJSONにシリアライズ
    fn to_json(&self) -> Result<String>;

    /// JSONから設定をデシリアライズ
    fn from_json(json: &str) -> Result<Self>
    where
        Self: Sized;

    /// アルゴリズムの説明を取得
    fn description(&self) -> &'static str;

    /// デフォルト設定を取得
    fn default_config() -> Self
    where
        Self: Sized;

    /// 設定の妥当性をチェック
    fn validate(&self) -> Result<()>;

    /// 設定のパラメータ情報を取得（CLI生成用）
    fn parameter_info() -> Vec<ParameterInfo>
    where
        Self: Sized;
}

/// パラメータ情報（CLI生成用）
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    pub default_value: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub enum ParameterType {
    Integer { min: Option<i64>, max: Option<i64> },
    Float { min: Option<f64>, max: Option<f64> },
    String,
    Path,
    Boolean,
}

/// 動的アルゴリズム設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicAlgorithmConfig {
    pub algorithm: String,
    pub parameters: serde_json::Value,
}

impl DynamicAlgorithmConfig {
    pub fn new(algorithm: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            algorithm: algorithm.into(),
            parameters,
        }
    }
}

// Type alias for complex creator function type
type CreatorFunction =
    Box<dyn Fn(&serde_json::Value) -> Result<Box<dyn PerceptualHashBackend>> + Send + Sync>;

/// アルゴリズム設定レジストリ
pub struct AlgorithmRegistry {
    creators: HashMap<String, CreatorFunction>,
    descriptions: HashMap<String, String>,
    parameter_infos: HashMap<String, Vec<ParameterInfo>>,
}

impl AlgorithmRegistry {
    pub fn new() -> Self {
        Self {
            creators: HashMap::new(),
            descriptions: HashMap::new(),
            parameter_infos: HashMap::new(),
        }
    }

    /// アルゴリズムを登録
    pub fn register<T: AlgorithmConfig + DeserializeOwned + 'static>(&mut self)
    where
        T::Algorithm: 'static,
    {
        let name = T::default_config().algorithm_name().to_string();
        let description = T::default_config().description().to_string();
        let param_info = T::parameter_info();

        // ハッシュアルゴリズム作成関数を登録
        self.creators.insert(
            name.clone(),
            Box::new(|params| {
                let config: T = serde_json::from_value(params.clone())
                    .map_err(|e| anyhow::anyhow!("パラメータ解析エラー: {}", e))?;
                config.validate()?;
                let hasher = config.create_hasher()?;
                Ok(Box::new(hasher) as Box<dyn PerceptualHashBackend>)
            }),
        );

        self.descriptions.insert(name.clone(), description);
        self.parameter_infos.insert(name, param_info);
    }

    /// 設定からハッシュアルゴリズムを作成
    pub fn create_hasher(
        &self,
        config: &DynamicAlgorithmConfig,
    ) -> Result<Box<dyn PerceptualHashBackend>> {
        let creator = self
            .creators
            .get(&config.algorithm)
            .ok_or_else(|| anyhow::anyhow!("未知のアルゴリズム: {}", config.algorithm))?;

        creator(&config.parameters)
    }

    /// 利用可能なアルゴリズムの一覧を取得
    pub fn available_algorithms(&self) -> Vec<String> {
        self.creators.keys().cloned().collect()
    }

    /// アルゴリズムの説明を取得
    pub fn get_description(&self, algorithm: &str) -> Option<&String> {
        self.descriptions.get(algorithm)
    }

    /// アルゴリズムのパラメータ情報を取得
    pub fn get_parameter_info(&self, algorithm: &str) -> Option<&Vec<ParameterInfo>> {
        self.parameter_infos.get(algorithm)
    }
}

impl Default for AlgorithmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// デフォルトのアルゴリズムレジストリを作成
pub fn create_default_registry() -> AlgorithmRegistry {
    AlgorithmRegistry::new()

    // デフォルトアルゴリズムを登録
    // Note: 現在はモジュールが完全でないため、後で追加
    // registry.register::<super::dct_config::DctConfig>();
    // registry.register::<super::average_config::AverageConfig>();
    // registry.register::<super::difference_config::DifferenceConfig>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_algorithm_config_creation() {
        let params = serde_json::json!({
            "size": 8,
            "quality_factor": 0.95
        });

        let config = DynamicAlgorithmConfig::new("dct", params);
        assert_eq!(config.algorithm, "dct");
        assert_eq!(config.parameters["size"], 8);
        assert_eq!(config.parameters["quality_factor"], 0.95);
    }

    #[test]
    fn test_algorithm_registry_operations() {
        let registry = AlgorithmRegistry::new();

        // 初期状態では空
        assert!(registry.available_algorithms().is_empty());
        assert!(registry.get_description("nonexistent").is_none());
        assert!(registry.get_parameter_info("nonexistent").is_none());
    }

    #[test]
    fn test_parameter_info_creation() {
        let param = ParameterInfo {
            name: "size".to_string(),
            param_type: ParameterType::Integer {
                min: Some(1),
                max: Some(64),
            },
            description: "Hash size".to_string(),
            default_value: Some("8".to_string()),
            required: true,
        };

        assert_eq!(param.name, "size");
        assert!(param.required);
        assert_eq!(param.default_value, Some("8".to_string()));
    }
}
