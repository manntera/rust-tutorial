//! StorageFactory - ストレージバックエンドの Factory Pattern 実装

use super::{ComponentConfig, ComponentFactory};
use crate::storage::{local::LocalStorageBackend, StorageBackend};
use anyhow::Result;

pub struct StorageFactory;

impl StorageFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StorageFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentFactory<Box<dyn StorageBackend>> for StorageFactory {
    fn create(&self, config: &ComponentConfig) -> Result<Box<dyn StorageBackend>> {
        match config.implementation.as_str() {
            "local" => Ok(Box::new(LocalStorageBackend::new())),
            _ => anyhow::bail!(
                "未サポートのStorage実装: {}. 利用可能: local",
                config.implementation
            ),
        }
    }

    fn available_implementations(&self) -> Vec<String> {
        vec!["local".to_string()]
    }

    fn get_description(&self, implementation: &str) -> Option<String> {
        match implementation {
            "local" => Some("ローカルファイルシステムストレージ".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_local_storage() {
        let factory = StorageFactory::new();
        let config = ComponentConfig::new("local", json!({}));

        let storage = factory.create(&config);
        assert!(storage.is_ok());
    }

    #[test]
    fn test_unsupported_implementation() {
        let factory = StorageFactory::new();
        let config = ComponentConfig::new("unsupported", json!({}));

        let result = factory.create(&config);
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.to_string().contains("未サポートのStorage実装"));
        }
    }

    #[test]
    fn test_available_implementations() {
        let factory = StorageFactory::new();
        let implementations = factory.available_implementations();

        assert_eq!(implementations, vec!["local"]);
    }

    #[test]
    fn test_get_description() {
        let factory = StorageFactory::new();

        assert!(factory.get_description("local").is_some());
        assert!(factory.get_description("unknown").is_none());
    }
}
