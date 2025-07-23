//! ProgressReporterFactory - 進捗報告の Factory Pattern 実装

use super::{ComponentConfig, ComponentFactory};
use crate::core::ProgressReporter;
use crate::services::{ConsoleProgressReporter, NoOpProgressReporter};
use anyhow::Result;

pub struct ProgressReporterFactory;

impl ProgressReporterFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProgressReporterFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentFactory<Box<dyn ProgressReporter>> for ProgressReporterFactory {
    fn create(&self, config: &ComponentConfig) -> Result<Box<dyn ProgressReporter>> {
        match config.implementation.as_str() {
            "console" => {
                let quiet = config
                    .parameters
                    .get("quiet")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let reporter = if quiet {
                    ConsoleProgressReporter::quiet()
                } else {
                    ConsoleProgressReporter::new()
                };

                Ok(Box::new(reporter))
            }
            "noop" => Ok(Box::new(NoOpProgressReporter::new())),
            _ => anyhow::bail!(
                "未サポートのProgressReporter実装: {}. 利用可能: console, noop",
                config.implementation
            ),
        }
    }

    fn available_implementations(&self) -> Vec<String> {
        vec!["console".to_string(), "noop".to_string()]
    }

    fn get_description(&self, implementation: &str) -> Option<String> {
        match implementation {
            "console" => Some("コンソール出力による進捗報告".to_string()),
            "noop" => Some("何もしない進捗報告 (テスト・ベンチマーク用)".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_console_reporter() {
        let factory = ProgressReporterFactory::new();
        let config = ComponentConfig::new(
            "console",
            json!({
                "quiet": false
            }),
        );

        let reporter = factory.create(&config);
        assert!(reporter.is_ok());
    }

    #[test]
    fn test_create_quiet_console_reporter() {
        let factory = ProgressReporterFactory::new();
        let config = ComponentConfig::new(
            "console",
            json!({
                "quiet": true
            }),
        );

        let reporter = factory.create(&config);
        assert!(reporter.is_ok());
    }

    #[test]
    fn test_create_noop_reporter() {
        let factory = ProgressReporterFactory::new();
        let config = ComponentConfig::new("noop", json!({}));

        let reporter = factory.create(&config);
        assert!(reporter.is_ok());
    }

    #[test]
    fn test_create_with_default_params() {
        let factory = ProgressReporterFactory::new();
        let config = ComponentConfig::new("console", json!({}));

        let reporter = factory.create(&config);
        assert!(reporter.is_ok());
    }

    #[test]
    fn test_unsupported_implementation() {
        let factory = ProgressReporterFactory::new();
        let config = ComponentConfig::new("unsupported", json!({}));

        let result = factory.create(&config);
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error
                .to_string()
                .contains("未サポートのProgressReporter実装"));
        }
    }

    #[test]
    fn test_available_implementations() {
        let factory = ProgressReporterFactory::new();
        let implementations = factory.available_implementations();

        assert_eq!(implementations, vec!["console", "noop"]);
    }

    #[test]
    fn test_get_description() {
        let factory = ProgressReporterFactory::new();

        assert!(factory.get_description("console").is_some());
        assert!(factory.get_description("noop").is_some());
        assert!(factory.get_description("unknown").is_none());
    }
}
