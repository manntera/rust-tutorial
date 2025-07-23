/// 統一DIコンテナ - 全ての依存関係を設定駆動で管理
/// 
/// Rustの哲学に従った型安全な依存関係注入システム
/// - Zero-cost abstraction: コンパイル時に最適化される
/// - Ownership-based design: Rustの所有権システムを活用
/// - Trait-based architecture: 抽象化により拡張性を確保
use crate::core::{
    HashPersistence, ProcessingConfig, ProgressReporter, ProcessingError, ProcessingResult,
};
use crate::factories::{ComponentFactory, ComponentFactoryWithPath};
use crate::image_loader::ImageLoaderBackend;
use crate::perceptual_hash::PerceptualHashBackend;
use crate::storage::StorageBackend;
use serde::{Deserialize, Serialize};

/// DIコンテナ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConfig {
    pub image_loader: ComponentConfig,
    pub perceptual_hash: ComponentConfig,
    pub storage: ComponentConfig,
    pub processing_config: ComponentConfig,
    pub progress_reporter: ComponentConfig,
    pub hash_persistence: ComponentConfig,
}

/// 各コンポーネントの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    pub implementation: String,
    pub parameters: serde_json::Value,
}

impl ComponentConfig {
    pub fn new(implementation: &str, parameters: serde_json::Value) -> Self {
        Self {
            implementation: implementation.to_string(),
            parameters,
        }
    }
}

impl Default for DependencyConfig {
    fn default() -> Self {
        Self {
            image_loader: ComponentConfig::new("standard", serde_json::json!({
                "max_dimension": 512
            })),
            perceptual_hash: ComponentConfig::new("dct", serde_json::json!({
                "size": 8,
                "quality_factor": 1.0
            })),
            storage: ComponentConfig::new("local", serde_json::json!({})),
            processing_config: ComponentConfig::new("default", serde_json::json!({
                "max_concurrent": null,
                "buffer_size": 100,
                "batch_size": 50,
                "enable_progress": true
            })),
            progress_reporter: ComponentConfig::new("console", serde_json::json!({
                "quiet": false
            })),
            hash_persistence: ComponentConfig::new("streaming_json", serde_json::json!({
                "buffer_size": 100
            })),
        }
    }
}

impl DependencyConfig {
    /// 高性能設定を生成
    pub fn high_performance() -> Self {
        Self {
            image_loader: ComponentConfig::new("standard", serde_json::json!({
                "max_dimension": 2048
            })),
            perceptual_hash: ComponentConfig::new("dct", serde_json::json!({
                "size": 32,
                "quality_factor": 1.0
            })),
            storage: ComponentConfig::new("local", serde_json::json!({})),
            processing_config: ComponentConfig::new("default", serde_json::json!({
                "max_concurrent": 16,
                "buffer_size": 500,
                "batch_size": 100,
                "enable_progress": true
            })),
            progress_reporter: ComponentConfig::new("console", serde_json::json!({
                "quiet": false
            })),
            hash_persistence: ComponentConfig::new("streaming_json", serde_json::json!({
                "buffer_size": 300
            })),
        }
    }

    /// テスト用設定を生成
    pub fn for_testing() -> Self {
        Self {
            image_loader: ComponentConfig::new("standard", serde_json::json!({
                "max_dimension": 512
            })),
            perceptual_hash: ComponentConfig::new("average", serde_json::json!({
                "size": 8
            })),
            storage: ComponentConfig::new("local", serde_json::json!({})),
            processing_config: ComponentConfig::new("default", serde_json::json!({
                "max_concurrent": 2,
                "buffer_size": 10,
                "batch_size": 5,
                "enable_progress": false
            })),
            progress_reporter: ComponentConfig::new("noop", serde_json::json!({})),
            hash_persistence: ComponentConfig::new("memory", serde_json::json!({})),
        }
    }

    /// 設定をバリデーション
    pub fn validate(&self) -> ProcessingResult<()> {
        // 各コンポーネントの設定をバリデーション
        self.validate_image_loader()?;
        self.validate_perceptual_hash()?;
        self.validate_storage()?;
        self.validate_processing_config()?;
        self.validate_progress_reporter()?;
        self.validate_hash_persistence()?;
        
        Ok(())
    }

    fn validate_image_loader(&self) -> ProcessingResult<()> {
        let factory = crate::factories::ImageLoaderFactory::new();
        let available = factory.available_implementations();
        
        if !available.contains(&self.image_loader.implementation) {
            return Err(ProcessingError::configuration(format!(
                "無効なImageLoader実装: {}. 利用可能: {}",
                self.image_loader.implementation,
                available.join(", ")
            )));
        }
        
        Ok(())
    }

    fn validate_perceptual_hash(&self) -> ProcessingResult<()> {
        let factory = crate::factories::PerceptualHashFactory::new();
        let available = factory.available_implementations();
        
        if !available.contains(&self.perceptual_hash.implementation) {
            return Err(ProcessingError::configuration(format!(
                "無効なPerceptualHash実装: {}. 利用可能: {}",
                self.perceptual_hash.implementation,
                available.join(", ")
            )));
        }
        
        Ok(())
    }

    fn validate_storage(&self) -> ProcessingResult<()> {
        let factory = crate::factories::StorageFactory::new();
        let available = factory.available_implementations();
        
        if !available.contains(&self.storage.implementation) {
            return Err(ProcessingError::configuration(format!(
                "無効なStorage実装: {}. 利用可能: {}",
                self.storage.implementation,
                available.join(", ")
            )));
        }
        
        Ok(())
    }

    fn validate_processing_config(&self) -> ProcessingResult<()> {
        let factory = crate::factories::ProcessingConfigFactory::new();
        let available = factory.available_implementations();
        
        if !available.contains(&self.processing_config.implementation) {
            return Err(ProcessingError::configuration(format!(
                "無効なProcessingConfig実装: {}. 利用可能: {}",
                self.processing_config.implementation,
                available.join(", ")
            )));
        }
        
        Ok(())
    }

    fn validate_progress_reporter(&self) -> ProcessingResult<()> {
        let factory = crate::factories::ProgressReporterFactory::new();
        let available = factory.available_implementations();
        
        if !available.contains(&self.progress_reporter.implementation) {
            return Err(ProcessingError::configuration(format!(
                "無効なProgressReporter実装: {}. 利用可能: {}",
                self.progress_reporter.implementation,
                available.join(", ")
            )));
        }
        
        Ok(())
    }

    fn validate_hash_persistence(&self) -> ProcessingResult<()> {
        let factory = crate::factories::HashPersistenceFactory::new();
        let available = factory.available_implementations();
        
        if !available.contains(&self.hash_persistence.implementation) {
            return Err(ProcessingError::configuration(format!(
                "無効なHashPersistence実装: {}. 利用可能: {}",
                self.hash_persistence.implementation,
                available.join(", ")
            )));
        }
        
        Ok(())
    }
}

/// DIコンテナ - 型安全で高性能な依存関係管理
pub struct DependencyContainer {
    config: DependencyConfig,
}

impl DependencyContainer {
    /// 新しいDIコンテナを作成
    pub fn new(config: DependencyConfig) -> Self {
        Self { config }
    }

    /// デフォルト設定でDIコンテナを作成
    pub fn with_default_config() -> Self {
        Self::new(DependencyConfig::default())
    }

    /// 設定ファイルからDIコンテナを作成
    pub fn from_config_file(path: &std::path::Path) -> ProcessingResult<Self> {
        let config_str = std::fs::read_to_string(path)
            .map_err(|e| ProcessingError::configuration(format!("設定ファイル読み込みエラー: {e}")))?;
        
        let config: DependencyConfig = serde_json::from_str(&config_str)
            .map_err(|e| ProcessingError::configuration(format!("設定ファイル解析エラー: {e}")))?;
        
        // 設定をバリデーション
        config.validate()?;
        
        Ok(Self::new(config))
    }

    /// プリセット設定でDIコンテナを作成
    pub fn with_preset(preset: &str) -> ProcessingResult<Self> {
        let config = match preset {
            "default" => DependencyConfig::default(),
            "high_performance" => DependencyConfig::high_performance(),
            "testing" => DependencyConfig::for_testing(),
            _ => return Err(ProcessingError::configuration(format!(
                "無効なプリセット: {preset}. 利用可能: default, high_performance, testing"
            ))),
        };
        
        Ok(Self::new(config))
    }

    /// ImageLoaderを作成
    pub fn create_image_loader(&self) -> ProcessingResult<Box<dyn ImageLoaderBackend>> {
        let factory = crate::factories::ImageLoaderFactory::new();
        factory.create(&self.config.image_loader)
            .map_err(|e| ProcessingError::dependency_injection(format!("ImageLoader作成エラー: {e}")))
    }

    /// PerceptualHashBackendを作成
    pub fn create_perceptual_hash(&self) -> ProcessingResult<Box<dyn PerceptualHashBackend>> {
        let factory = crate::factories::PerceptualHashFactory::new();
        factory.create(&self.config.perceptual_hash)
            .map_err(|e| ProcessingError::dependency_injection(format!("PerceptualHash作成エラー: {e}")))
    }

    /// StorageBackendを作成
    pub fn create_storage(&self) -> ProcessingResult<Box<dyn StorageBackend>> {
        let factory = crate::factories::StorageFactory::new();
        factory.create(&self.config.storage)
            .map_err(|e| ProcessingError::dependency_injection(format!("Storage作成エラー: {e}")))
    }

    /// ProcessingConfigを作成
    pub fn create_processing_config(&self) -> ProcessingResult<Box<dyn ProcessingConfig>> {
        let factory = crate::factories::ProcessingConfigFactory::new();
        factory.create(&self.config.processing_config)
            .map_err(|e| ProcessingError::dependency_injection(format!("ProcessingConfig作成エラー: {e}")))
    }

    /// ProgressReporterを作成
    pub fn create_progress_reporter(&self) -> ProcessingResult<Box<dyn ProgressReporter>> {
        let factory = crate::factories::ProgressReporterFactory::new();
        factory.create(&self.config.progress_reporter)
            .map_err(|e| ProcessingError::dependency_injection(format!("ProgressReporter作成エラー: {e}")))
    }

    /// HashPersistenceを作成
    pub fn create_hash_persistence(&self, output_path: &std::path::Path) -> ProcessingResult<Box<dyn HashPersistence>> {
        let factory = crate::factories::HashPersistenceFactory::new();
        factory.create(&self.config.hash_persistence, output_path)
            .map_err(|e| ProcessingError::dependency_injection(format!("HashPersistence作成エラー: {e}")))
    }

    /// 全ての依存関係を一度に解決してタプルで返す
    pub fn resolve_all_dependencies(&self, output_path: &std::path::Path) -> ProcessingResult<DependencyBundle> {
        Ok(DependencyBundle {
            image_loader: self.create_image_loader()?,
            perceptual_hash: self.create_perceptual_hash()?,
            storage: self.create_storage()?,
            processing_config: self.create_processing_config()?,
            progress_reporter: self.create_progress_reporter()?,
            hash_persistence: self.create_hash_persistence(output_path)?,
        })
    }

    /// 設定を取得
    pub fn config(&self) -> &DependencyConfig {
        &self.config
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: DependencyConfig) {
        self.config = config;
    }

    /// 特定のコンポーネント設定を更新
    pub fn update_component_config(&mut self, component: &str, config: ComponentConfig) -> ProcessingResult<()> {
        match component {
            "image_loader" => self.config.image_loader = config,
            "perceptual_hash" => self.config.perceptual_hash = config,
            "storage" => self.config.storage = config,
            "processing_config" => self.config.processing_config = config,
            "progress_reporter" => self.config.progress_reporter = config,
            "hash_persistence" => self.config.hash_persistence = config,
            _ => return Err(ProcessingError::configuration(format!("不明なコンポーネント: {component}"))),
        }
        Ok(())
    }
}

impl Default for DependencyContainer {
    fn default() -> Self {
        Self::new(DependencyConfig::default())
    }
}

/// 依存関係バンドル - 全ての依存関係を一つにまとめる
pub struct DependencyBundle {
    pub image_loader: Box<dyn ImageLoaderBackend>,
    pub perceptual_hash: Box<dyn PerceptualHashBackend>,
    pub storage: Box<dyn StorageBackend>,
    pub processing_config: Box<dyn ProcessingConfig>,
    pub progress_reporter: Box<dyn ProgressReporter>,
    pub hash_persistence: Box<dyn HashPersistence>,
}

/// 複雑な型のエイリアス
pub type BoxedProcessingEngine = crate::engine::ProcessingEngine<
    Box<dyn ImageLoaderBackend>,
    Box<dyn PerceptualHashBackend>,
    Box<dyn StorageBackend>,
    Box<dyn ProcessingConfig>,
    Box<dyn ProgressReporter>,
    Box<dyn HashPersistence>,
>;

impl DependencyBundle {
    /// ProcessingEngineを作成
    pub fn create_processing_engine(self) -> BoxedProcessingEngine {
        crate::engine::ProcessingEngine::new(
            self.image_loader,
            self.perceptual_hash,
            self.storage,
            self.processing_config,
            self.progress_reporter,
            self.hash_persistence,
        )
    }
}

/// DIコンテナビルダー - 流れるような設定API
pub struct DependencyContainerBuilder {
    config: DependencyConfig,
}

impl DependencyContainerBuilder {
    pub fn new() -> Self {
        Self {
            config: DependencyConfig::default(),
        }
    }

    /// 高性能設定でビルダーを作成
    pub fn high_performance() -> Self {
        Self {
            config: DependencyConfig::high_performance(),
        }
    }

    /// テスト用設定でビルダーを作成
    pub fn for_testing() -> Self {
        Self {
            config: DependencyConfig::for_testing(),
        }
    }

    pub fn with_image_loader(mut self, implementation: &str, parameters: serde_json::Value) -> Self {
        self.config.image_loader = ComponentConfig::new(implementation, parameters);
        self
    }

    pub fn with_perceptual_hash(mut self, implementation: &str, parameters: serde_json::Value) -> Self {
        self.config.perceptual_hash = ComponentConfig::new(implementation, parameters);
        self
    }

    pub fn with_storage(mut self, implementation: &str, parameters: serde_json::Value) -> Self {
        self.config.storage = ComponentConfig::new(implementation, parameters);
        self
    }

    pub fn with_processing_config(mut self, implementation: &str, parameters: serde_json::Value) -> Self {
        self.config.processing_config = ComponentConfig::new(implementation, parameters);
        self
    }

    pub fn with_progress_reporter(mut self, implementation: &str, parameters: serde_json::Value) -> Self {
        self.config.progress_reporter = ComponentConfig::new(implementation, parameters);
        self
    }

    pub fn with_hash_persistence(mut self, implementation: &str, parameters: serde_json::Value) -> Self {
        self.config.hash_persistence = ComponentConfig::new(implementation, parameters);
        self
    }

    pub fn build(self) -> DependencyContainer {
        DependencyContainer::new(self.config)
    }
}

impl Default for DependencyContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dependency_config_creation() {
        let config = DependencyConfig::default();
        
        assert_eq!(config.image_loader.implementation, "standard");
        assert_eq!(config.perceptual_hash.implementation, "dct");
        assert_eq!(config.storage.implementation, "local");
        assert_eq!(config.processing_config.implementation, "default");
        assert_eq!(config.progress_reporter.implementation, "console");
        assert_eq!(config.hash_persistence.implementation, "streaming_json");
    }

    #[test]
    fn test_dependency_container_creation() {
        let container = DependencyContainer::default();
        assert_eq!(container.config().image_loader.implementation, "standard");
    }

    #[test]
    fn test_dependency_container_builder() {
        let container = DependencyContainerBuilder::new()
            .with_image_loader("standard", serde_json::json!({"max_dimension": 1024}))
            .with_perceptual_hash("average", serde_json::json!({"size": 16}))
            .build();

        assert_eq!(container.config().image_loader.implementation, "standard");
        assert_eq!(container.config().perceptual_hash.implementation, "average");
    }

    #[test]
    fn test_component_config_update() {
        let mut container = DependencyContainer::default();
        
        let new_config = ComponentConfig::new("memory", serde_json::json!({}));
        container.update_component_config("hash_persistence", new_config).unwrap();
        
        assert_eq!(container.config().hash_persistence.implementation, "memory");
    }

    #[test]
    fn test_invalid_component_update() {
        let mut container = DependencyContainer::default();
        
        let new_config = ComponentConfig::new("test", serde_json::json!({}));
        let result = container.update_component_config("invalid_component", new_config);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = DependencyConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DependencyConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(config.image_loader.implementation, deserialized.image_loader.implementation);
        assert_eq!(config.perceptual_hash.implementation, deserialized.perceptual_hash.implementation);
    }

    #[test]
    fn test_config_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");
        
        let config = DependencyConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&config_path, json).unwrap();
        
        let loaded_container = DependencyContainer::from_config_file(&config_path).unwrap();
        assert_eq!(loaded_container.config().image_loader.implementation, "standard");
    }

    #[test]
    fn test_config_presets() {
        // デフォルトプリセット
        let default_container = DependencyContainer::with_preset("default").unwrap();
        assert_eq!(default_container.config().image_loader.implementation, "standard");
        
        // 高性能プリセット
        let high_perf_container = DependencyContainer::with_preset("high_performance").unwrap();
        assert_eq!(high_perf_container.config().perceptual_hash.implementation, "dct");
        
        // テスト用プリセット
        let test_container = DependencyContainer::with_preset("testing").unwrap();
        assert_eq!(test_container.config().progress_reporter.implementation, "noop");
        
        // 無効なプリセット
        let invalid_result = DependencyContainer::with_preset("invalid");
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = DependencyConfig::default();
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_invalid_image_loader() {
        let mut config = DependencyConfig::default();
        config.image_loader.implementation = "invalid_loader".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("無効なImageLoader実装"));
    }

    #[test]
    fn test_config_validation_invalid_perceptual_hash() {
        let mut config = DependencyConfig::default();
        config.perceptual_hash.implementation = "invalid_hasher".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("無効なPerceptualHash実装"));
    }

    #[test]
    fn test_high_performance_config() {
        let config = DependencyConfig::high_performance();
        
        assert_eq!(config.perceptual_hash.implementation, "dct");
        assert_eq!(config.perceptual_hash.parameters["size"], 32);
        assert_eq!(config.processing_config.parameters["max_concurrent"], 16);
        assert_eq!(config.hash_persistence.parameters["buffer_size"], 300);
    }

    #[test]
    fn test_testing_config() {
        let config = DependencyConfig::for_testing();
        
        assert_eq!(config.perceptual_hash.implementation, "average");
        assert_eq!(config.progress_reporter.implementation, "noop");
        assert_eq!(config.hash_persistence.implementation, "memory");
        assert_eq!(config.processing_config.parameters["enable_progress"], false);
    }

    #[test]
    fn test_builder_presets() {
        let high_perf_builder = DependencyContainerBuilder::high_performance();
        let container = high_perf_builder.build();
        assert_eq!(container.config().processing_config.parameters["max_concurrent"], 16);

        let test_builder = DependencyContainerBuilder::for_testing();
        let container = test_builder.build();
        assert_eq!(container.config().progress_reporter.implementation, "noop");
    }
}