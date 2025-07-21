// 並列処理制御のトレイト定義

use crate::processing::types::ProcessingSummary;
use crate::processing::config::ProcessingConfig;
use crate::processing::reporting::ProgressReporter;
use crate::processing::persistence::HashPersistence;

/// 並列処理オーケストレーションを抽象化するトレイト
#[async_trait::async_trait]
pub trait ParallelProcessor: Send + Sync {
    type Config: ProcessingConfig;
    type Reporter: ProgressReporter;
    type Persistence: HashPersistence;

    /// ディレクトリの並列処理実行
    async fn process_directory(
        &self,
        path: &str,
        config: &Self::Config,
        reporter: &Self::Reporter,
        persistence: &Self::Persistence,
    ) -> anyhow::Result<ProcessingSummary>;
}