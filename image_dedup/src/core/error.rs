// Custom error types for parallel processing
// 並列処理専用のカスタムエラー型定義

use thiserror::Error;

/// 並列処理固有のエラー型
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("ファイル発見エラー: {path} - {source}")]
    FileDiscoveryError {
        path: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("並列処理エラー: {message}")]
    ParallelExecutionError { message: String },

    #[error("永続化エラー: {source}")]
    PersistenceError {
        #[source]
        source: anyhow::Error,
    },

    #[error("設定エラー: {message}")]
    ConfigurationError { message: String },

    #[error("チャンネルエラー: {message}")]
    ChannelError { message: String },

    #[error("タスクエラー: {source}")]
    TaskError {
        #[source]
        source: tokio::task::JoinError,
    },

    #[error("画像処理エラー: {file_path} - {source}")]
    ImageProcessingError {
        file_path: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("依存関係注入エラー: {message}")]
    DependencyInjectionError { message: String },

    #[error("型安全性エラー: {message} (コンポーネント: {component})")]
    TypeSafetyError { message: String, component: String },

    #[error("バリデーションエラー: {field} - {reason}")]
    ValidationError { field: String, reason: String },

    #[error("リソース不足エラー: {resource_type} - {details}")]
    ResourceExhaustionError {
        resource_type: String,
        details: String,
    },

    #[error("内部エラー: {source}")]
    InternalError {
        #[source]
        source: anyhow::Error,
    },
}

impl ProcessingError {
    /// ファイル発見エラーの作成
    pub fn file_discovery(path: impl Into<String>, source: anyhow::Error) -> Self {
        Self::FileDiscoveryError {
            path: path.into(),
            source,
        }
    }

    /// 並列実行エラーの作成
    pub fn parallel_execution(message: impl Into<String>) -> Self {
        Self::ParallelExecutionError {
            message: message.into(),
        }
    }

    /// 永続化エラーの作成
    pub fn persistence(source: anyhow::Error) -> Self {
        Self::PersistenceError { source }
    }

    /// 設定エラーの作成
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// チャンネルエラーの作成
    pub fn channel(message: impl Into<String>) -> Self {
        Self::ChannelError {
            message: message.into(),
        }
    }

    /// タスクエラーの作成
    pub fn task(source: tokio::task::JoinError) -> Self {
        Self::TaskError { source }
    }

    /// 画像処理エラーの作成
    pub fn image_processing(file_path: impl Into<String>, source: anyhow::Error) -> Self {
        Self::ImageProcessingError {
            file_path: file_path.into(),
            source,
        }
    }

    /// 依存関係注入エラーの作成
    pub fn dependency_injection(message: impl Into<String>) -> Self {
        Self::DependencyInjectionError {
            message: message.into(),
        }
    }

    /// 内部エラーの作成
    pub fn internal(source: anyhow::Error) -> Self {
        Self::InternalError { source }
    }

    /// 型安全性エラーの作成
    pub fn type_safety(message: impl Into<String>, component: impl Into<String>) -> Self {
        Self::TypeSafetyError {
            message: message.into(),
            component: component.into(),
        }
    }

    /// バリデーションエラーの作成
    pub fn validation(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ValidationError {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// リソース不足エラーの作成
    pub fn resource_exhaustion(
        resource_type: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::ResourceExhaustionError {
            resource_type: resource_type.into(),
            details: details.into(),
        }
    }

    /// エラーの重要度を取得
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::TypeSafetyError { .. } | Self::ValidationError { .. } => ErrorSeverity::Critical,
            Self::DependencyInjectionError { .. } | Self::ConfigurationError { .. } => {
                ErrorSeverity::High
            }
            Self::FileDiscoveryError { .. } | Self::ImageProcessingError { .. } => {
                ErrorSeverity::Medium
            }
            Self::ParallelExecutionError { .. } | Self::PersistenceError { .. } => {
                ErrorSeverity::High
            }
            Self::ChannelError { .. } | Self::TaskError { .. } => ErrorSeverity::Medium,
            Self::ResourceExhaustionError { .. } => ErrorSeverity::High,
            Self::InternalError { .. } => ErrorSeverity::Critical,
        }
    }

    /// エラーが回復可能かどうかを判定
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::TypeSafetyError { .. } | Self::ValidationError { .. } => false,
            Self::DependencyInjectionError { .. } | Self::ConfigurationError { .. } => false,
            Self::FileDiscoveryError { .. } => true,
            Self::ImageProcessingError { .. } => true,
            Self::ParallelExecutionError { .. } => true,
            Self::PersistenceError { .. } => true,
            Self::ChannelError { .. } => true,
            Self::TaskError { .. } => true,
            Self::ResourceExhaustionError { .. } => true,
            Self::InternalError { .. } => false,
        }
    }

    /// エラーコンテキストを取得
    pub fn context(&self) -> ErrorContext {
        match self {
            Self::FileDiscoveryError { path, .. } => ErrorContext {
                operation: "file_discovery".to_string(),
                resource: Some(path.clone()),
                suggestion: Some("ファイルパスとアクセス権限を確認してください".to_string()),
            },
            Self::ImageProcessingError { file_path, .. } => ErrorContext {
                operation: "image_processing".to_string(),
                resource: Some(file_path.clone()),
                suggestion: Some("画像ファイルの形式と整合性を確認してください".to_string()),
            },
            Self::ConfigurationError { message } => ErrorContext {
                operation: "configuration".to_string(),
                resource: None,
                suggestion: Some(format!("設定を確認してください: {message}")),
            },
            Self::TypeSafetyError { component, .. } => ErrorContext {
                operation: "type_safety_check".to_string(),
                resource: Some(component.clone()),
                suggestion: Some("型制約を確認し、適切な実装を使用してください".to_string()),
            },
            _ => ErrorContext {
                operation: "unknown".to_string(),
                resource: None,
                suggestion: None,
            },
        }
    }
}

/// エラーの重要度レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// 低重要度 - ログ出力程度
    Low,
    /// 中重要度 - 警告レベル
    Medium,
    /// 高重要度 - 要対応
    High,
    /// 致命的 - システム停止レベル
    Critical,
}

impl ErrorSeverity {
    /// 重要度の数値表現を取得
    pub const fn as_level(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }

    /// 重要度の文字列表現を取得
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

/// エラーコンテキスト情報
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 実行していた操作
    pub operation: String,
    /// 関連するリソース（ファイルパス等）
    pub resource: Option<String>,
    /// エラー解決のための提案
    pub suggestion: Option<String>,
}

impl ErrorContext {
    /// 新しいエラーコンテキストを作成
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            resource: None,
            suggestion: None,
        }
    }

    /// リソース情報を追加
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// 提案を追加
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// 並列処理の結果型
pub type ProcessingResult<T> = std::result::Result<T, ProcessingError>;

/// 型安全な結果型 - 特定のエラー型を明示
pub type TypeSafeResult<T, E> = std::result::Result<T, E>;

/// 検証結果 - バリデーション専用の結果型
pub type ValidationResult<T> = std::result::Result<T, ValidationError>;

/// バリデーション専用エラー型
#[derive(Error, Debug, Clone)]
#[error("バリデーションエラー: {field} - {reason}")]
pub struct ValidationError {
    pub field: String,
    pub reason: String,
}

impl ValidationError {
    /// 新しいバリデーションエラーを作成
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

// From実装を個別に追加
impl From<anyhow::Error> for ProcessingError {
    fn from(error: anyhow::Error) -> Self {
        ProcessingError::InternalError { source: error }
    }
}

impl From<tokio::task::JoinError> for ProcessingError {
    fn from(error: tokio::task::JoinError) -> Self {
        ProcessingError::TaskError { source: error }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_processing_error_creation() {
        let file_error = ProcessingError::file_discovery(
            "/test/path",
            anyhow::anyhow!("ファイルが見つかりません"),
        );
        assert!(file_error.to_string().contains("/test/path"));
        assert!(file_error.to_string().contains("ファイル発見エラー"));

        let parallel_error = ProcessingError::parallel_execution("並列処理が失敗しました");
        assert!(parallel_error.to_string().contains("並列処理エラー"));

        let config_error = ProcessingError::configuration("無効な設定です");
        assert!(config_error.to_string().contains("設定エラー"));

        let channel_error = ProcessingError::channel("チャンネルが閉じられました");
        assert!(channel_error.to_string().contains("チャンネルエラー"));

        let persistence_error = ProcessingError::persistence(anyhow::anyhow!("書き込み失敗"));
        assert!(persistence_error.to_string().contains("永続化エラー"));

        let internal_error = ProcessingError::internal(anyhow::anyhow!("予期しないエラー"));
        assert!(internal_error.to_string().contains("内部エラー"));
    }

    #[test]
    fn test_error_source_chain() {
        let source_error = anyhow::anyhow!("ルートエラー");
        let processing_error = ProcessingError::persistence(source_error);

        // エラーチェーンが正しく設定されていることを確認
        assert!(processing_error.source().is_some());
    }

    #[test]
    fn test_error_display() {
        let error = ProcessingError::configuration("並列数は1以上である必要があります");
        let error_string = format!("{error}");

        assert!(error_string.contains("設定エラー"));
        assert!(error_string.contains("並列数は1以上である必要があります"));
    }

    #[test]
    fn test_image_processing_error() {
        let source = anyhow::anyhow!("画像ファイルが破損しています");
        let error = ProcessingError::image_processing("/path/to/image.jpg", source);

        assert!(error.to_string().contains("画像処理エラー"));
        assert!(error.to_string().contains("/path/to/image.jpg"));
        assert!(error.to_string().contains("画像ファイルが破損しています"));
    }

    #[tokio::test]
    async fn test_task_error() {
        // タスクエラーのテスト用にわざと失敗するタスクを作成
        let task = tokio::spawn(async {
            // 意図的にTaskエラーを発生させるためにタスクを中断
            tokio::task::yield_now().await;
            std::future::pending::<()>().await;
        });
        // タスクをキャンセルしてJoinErrorを発生させる
        task.abort();

        let join_result = task.await;
        assert!(join_result.is_err(), "タスクは失敗するべきです");
        let join_error = join_result.expect_err("タスクエラーが期待されます");
        let processing_error = ProcessingError::task(join_error);

        assert!(processing_error.to_string().contains("タスクエラー"));
    }

    #[test]
    fn test_error_severity() {
        let type_safety_error = ProcessingError::type_safety("型制約違反", "ImageLoader");
        assert_eq!(type_safety_error.severity(), ErrorSeverity::Critical);

        let validation_error = ProcessingError::validation("batch_size", "値が範囲外です");
        assert_eq!(validation_error.severity(), ErrorSeverity::Critical);

        let file_error = ProcessingError::file_discovery("/test", anyhow::anyhow!("Not found"));
        assert_eq!(file_error.severity(), ErrorSeverity::Medium);

        // 重要度の順序テスト
        assert!(ErrorSeverity::Critical > ErrorSeverity::High);
        assert!(ErrorSeverity::High > ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium > ErrorSeverity::Low);
    }

    #[test]
    fn test_error_recoverability() {
        let type_safety_error = ProcessingError::type_safety("型制約違反", "ImageLoader");
        assert!(!type_safety_error.is_recoverable());

        let file_error = ProcessingError::file_discovery("/test", anyhow::anyhow!("Not found"));
        assert!(file_error.is_recoverable());

        let config_error = ProcessingError::configuration("Invalid config");
        assert!(!config_error.is_recoverable());
    }

    #[test]
    fn test_error_context() {
        let file_error =
            ProcessingError::file_discovery("/test/path", anyhow::anyhow!("Not found"));
        let context = file_error.context();

        assert_eq!(context.operation, "file_discovery");
        assert_eq!(context.resource, Some("/test/path".to_string()));
        assert!(context.suggestion.is_some());

        let type_error = ProcessingError::type_safety("型制約違反", "ImageLoader");
        let context = type_error.context();

        assert_eq!(context.operation, "type_safety_check");
        assert_eq!(context.resource, Some("ImageLoader".to_string()));
        assert!(context.suggestion.is_some());
    }

    #[test]
    fn test_validation_error() {
        let validation_error = ValidationError::new("batch_size", "値は1以上である必要があります");

        assert_eq!(validation_error.field, "batch_size");
        assert_eq!(validation_error.reason, "値は1以上である必要があります");
        assert!(validation_error
            .to_string()
            .contains("バリデーションエラー"));
    }

    #[test]
    fn test_error_severity_levels() {
        assert_eq!(ErrorSeverity::Low.as_level(), 1);
        assert_eq!(ErrorSeverity::Medium.as_level(), 2);
        assert_eq!(ErrorSeverity::High.as_level(), 3);
        assert_eq!(ErrorSeverity::Critical.as_level(), 4);

        assert_eq!(ErrorSeverity::Low.as_str(), "LOW");
        assert_eq!(ErrorSeverity::Critical.as_str(), "CRITICAL");
    }

    #[test]
    fn test_error_context_builder() {
        let context = ErrorContext::new("test_operation")
            .with_resource("/test/resource")
            .with_suggestion("Try restarting");

        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.resource, Some("/test/resource".to_string()));
        assert_eq!(context.suggestion, Some("Try restarting".to_string()));
    }
}
