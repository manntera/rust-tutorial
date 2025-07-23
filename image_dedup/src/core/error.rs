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

    /// 内部エラーの作成
    pub fn internal(source: anyhow::Error) -> Self {
        Self::InternalError { source }
    }
}

/// 並列処理の結果型
pub type ProcessingResult<T> = std::result::Result<T, ProcessingError>;

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
            panic!("テスト用のパニック");
        });

        let join_error = task.await.unwrap_err();
        let processing_error = ProcessingError::task(join_error);

        assert!(processing_error.to_string().contains("タスクエラー"));
    }
}
