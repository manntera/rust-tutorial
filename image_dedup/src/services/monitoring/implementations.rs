// 進捗監視の具象実装

use crate::core::ProgressReporter;
use async_trait::async_trait;

/// コンソール出力による進捗報告実装
#[derive(Debug, Default, Clone)]
pub struct ConsoleProgressReporter {
    quiet: bool,
}

impl ConsoleProgressReporter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quiet() -> Self {
        Self { quiet: true }
    }
}

#[async_trait]
impl ProgressReporter for ConsoleProgressReporter {
    async fn report_started(&self, total_files: usize) {
        if !self.quiet {
            println!("🚀 Starting processing {total_files} files...");
        }
    }

    async fn report_progress(&self, completed: usize, total: usize) {
        if !self.quiet && (completed % 100 == 0 || completed == total) {
            let percentage = (completed as f64 / total as f64) * 100.0;
            println!("📊 Progress: {completed}/{total} ({percentage:.1}%)");
        }
    }

    async fn report_error(&self, file_path: &str, error: &str) {
        if !self.quiet {
            eprintln!("❌ Error processing {file_path}: {error}");
        }
    }

    async fn report_completed(&self, total_processed: usize, total_errors: usize) {
        if !self.quiet {
            println!("✅ Completed! Processed: {total_processed}, Errors: {total_errors}");
        }
    }
}

/// 何もしない進捗報告実装（テスト・ベンチマーク用）
#[derive(Debug, Default, Clone)]
pub struct NoOpProgressReporter;

impl NoOpProgressReporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProgressReporter for NoOpProgressReporter {
    async fn report_started(&self, _total_files: usize) {
        // 何もしない
    }

    async fn report_progress(&self, _completed: usize, _total: usize) {
        // 何もしない
    }

    async fn report_error(&self, _file_path: &str, _error: &str) {
        // 何もしない
    }

    async fn report_completed(&self, _total_processed: usize, _total_errors: usize) {
        // 何もしない
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_console_progress_reporter() {
        // 出力キャプチャは複雑なため、基本的な呼び出しテストのみ
        let reporter = ConsoleProgressReporter::quiet(); // quiet modeでテスト

        reporter.report_started(100).await;
        reporter.report_progress(50, 100).await;
        reporter.report_error("/test.jpg", "test error").await;
        reporter.report_completed(99, 1).await;

        // 基本的な呼び出しが成功することを確認
    }

    #[tokio::test]
    async fn test_console_progress_reporter_creation() {
        let reporter1 = ConsoleProgressReporter::new();
        let reporter2 = ConsoleProgressReporter::quiet();

        assert!(!reporter1.quiet);
        assert!(reporter2.quiet);
    }

    #[tokio::test]
    async fn test_noop_progress_reporter() {
        let reporter = NoOpProgressReporter::new();

        // 全てのメソッドを呼び出してもパニックしない
        reporter.report_started(100).await;
        reporter.report_progress(50, 100).await;
        reporter.report_error("/test.jpg", "test error").await;
        reporter.report_completed(99, 1).await;

        // 基本的な呼び出しが成功することを確認
    }
}
