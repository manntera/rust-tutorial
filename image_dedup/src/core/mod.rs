// コアレイヤー - 基盤となるトレイト、型、エラー定義
// 他のレイヤーから参照される基本的な抽象化を提供

pub mod error;
pub mod traits;
pub mod types;

// 公開API - 明示的にエクスポートして曖昧性を回避
pub use error::{ProcessingError, ProcessingResult};
pub use traits::{HashPersistence, ParallelProcessor, ProcessingConfig, ProgressReporter};
pub use types::ProcessingOutcome;
pub use types::{ProcessingMetadata, ProcessingSummary};
