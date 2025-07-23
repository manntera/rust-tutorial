//! HashPersistenceFactory - ハッシュ永続化の Factory Pattern 実装

use super::{ComponentConfig, ComponentFactoryWithPath};
use crate::core::HashPersistence;
use crate::services::{JsonHashPersistence, MemoryHashPersistence, StreamingJsonHashPersistence};
use anyhow::Result;
use std::path::Path;

pub struct HashPersistenceFactory;

impl HashPersistenceFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HashPersistenceFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentFactoryWithPath<Box<dyn HashPersistence>> for HashPersistenceFactory {
    fn create(
        &self,
        config: &ComponentConfig,
        output_path: &Path,
    ) -> Result<Box<dyn HashPersistence>> {
        match config.implementation.as_str() {
            "json" => Ok(Box::new(JsonHashPersistence::new(output_path))),
            "streaming_json" => {
                let buffer_size = config
                    .parameters
                    .get("buffer_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                Ok(Box::new(StreamingJsonHashPersistence::with_buffer_size(
                    output_path,
                    buffer_size,
                )))
            }
            "memory" => Ok(Box::new(MemoryHashPersistence::new())),
            _ => anyhow::bail!(
                "未サポートのHashPersistence実装: {}. 利用可能: json, streaming_json, memory",
                config.implementation
            ),
        }
    }

    fn available_implementations(&self) -> Vec<String> {
        vec![
            "json".to_string(),
            "streaming_json".to_string(),
            "memory".to_string(),
        ]
    }

    fn get_description(&self, implementation: &str) -> Option<String> {
        match implementation {
            "json" => Some("JSON形式でファイル出力 (全データメモリ保持)".to_string()),
            "streaming_json" => Some("ストリーミングJSON出力 (メモリ効率重視)".to_string()),
            "memory" => Some("メモリ内保存 (テスト・開発用)".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_create_json_persistence() {
        let factory = HashPersistenceFactory::new();
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");
        let config = ComponentConfig::new("json", json!({}));

        let persistence = factory.create(&config, &output_path);
        assert!(persistence.is_ok());
    }

    #[test]
    fn test_create_streaming_json_persistence() {
        let factory = HashPersistenceFactory::new();
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_streaming.json");
        let config = ComponentConfig::new(
            "streaming_json",
            json!({
                "buffer_size": 50
            }),
        );

        let persistence = factory.create(&config, &output_path);
        assert!(persistence.is_ok());
    }

    #[test]
    fn test_create_memory_persistence() {
        let factory = HashPersistenceFactory::new();
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("dummy.json");
        let config = ComponentConfig::new("memory", json!({}));

        let persistence = factory.create(&config, &output_path);
        assert!(persistence.is_ok());
    }

    #[test]
    fn test_create_with_default_params() {
        let factory = HashPersistenceFactory::new();
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");
        let config = ComponentConfig::new("streaming_json", json!({}));

        let persistence = factory.create(&config, &output_path);
        assert!(persistence.is_ok());
    }

    #[test]
    fn test_unsupported_implementation() {
        let factory = HashPersistenceFactory::new();
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");
        let config = ComponentConfig::new("unsupported", json!({}));

        let result = factory.create(&config, &output_path);
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error
                .to_string()
                .contains("未サポートのHashPersistence実装"));
        }
    }

    #[test]
    fn test_available_implementations() {
        let factory = HashPersistenceFactory::new();
        let implementations = factory.available_implementations();

        assert_eq!(implementations, vec!["json", "streaming_json", "memory"]);
    }

    #[test]
    fn test_get_description() {
        let factory = HashPersistenceFactory::new();

        assert!(factory.get_description("json").is_some());
        assert!(factory.get_description("streaming_json").is_some());
        assert!(factory.get_description("memory").is_some());
        assert!(factory.get_description("unknown").is_none());
    }
}
