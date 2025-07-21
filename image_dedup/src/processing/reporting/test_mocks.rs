// テスト用の進捗報告モック実装

use super::traits::ProgressReporter;

pub struct MockProgressReporter {
    pub started_called: std::sync::Arc<std::sync::Mutex<bool>>,
    pub progress_calls: std::sync::Arc<std::sync::Mutex<Vec<(usize, usize)>>>,
    pub error_calls: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
    pub completed_called: std::sync::Arc<std::sync::Mutex<Option<(usize, usize)>>>,
}

impl MockProgressReporter {
    pub fn new() -> Self {
        Self {
            started_called: std::sync::Arc::new(std::sync::Mutex::new(false)),
            progress_calls: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            error_calls: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            completed_called: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl ProgressReporter for MockProgressReporter {
    async fn report_started(&self, _total_files: usize) {
        *self.started_called.lock().unwrap() = true;
    }
    
    async fn report_progress(&self, completed: usize, total: usize) {
        self.progress_calls.lock().unwrap().push((completed, total));
    }
    
    async fn report_error(&self, file_path: &str, error: &str) {
        self.error_calls.lock().unwrap().push((file_path.to_string(), error.to_string()));
    }
    
    async fn report_completed(&self, total_processed: usize, total_errors: usize) {
        *self.completed_called.lock().unwrap() = Some((total_processed, total_errors));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_reporter_trait() {
        let reporter = MockProgressReporter::new();
        
        reporter.report_started(100).await;
        assert!(*reporter.started_called.lock().unwrap());
        
        reporter.report_progress(50, 100).await;
        reporter.report_progress(100, 100).await;
        let progress_calls = reporter.progress_calls.lock().unwrap();
        assert_eq!(progress_calls.len(), 2);
        assert_eq!(progress_calls[0], (50, 100));
        assert_eq!(progress_calls[1], (100, 100));
        
        reporter.report_error("/path/to/file.jpg", "Failed to load").await;
        let error_calls = reporter.error_calls.lock().unwrap();
        assert_eq!(error_calls.len(), 1);
        assert_eq!(error_calls[0], ("/path/to/file.jpg".to_string(), "Failed to load".to_string()));
        
        reporter.report_completed(95, 5).await;
        let completed = reporter.completed_called.lock().unwrap();
        assert_eq!(*completed, Some((95, 5)));
    }

    #[tokio::test]
    async fn test_progress_reporter_thread_safety() {
        let reporter = MockProgressReporter::new();
        let reporter_ref: &dyn ProgressReporter = &reporter;
        
        reporter_ref.report_started(10).await;
        reporter_ref.report_progress(5, 10).await;
        reporter_ref.report_error("test.jpg", "test error").await;
        reporter_ref.report_completed(9, 1).await;
        
        // Verify all calls were made
        assert!(*reporter.started_called.lock().unwrap());
        assert_eq!(reporter.progress_calls.lock().unwrap().len(), 1);
        assert_eq!(reporter.error_calls.lock().unwrap().len(), 1);
        assert!(reporter.completed_called.lock().unwrap().is_some());
    }
}