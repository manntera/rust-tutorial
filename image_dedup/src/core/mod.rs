// コアレイヤー - 基盤となるトレイト、型、エラー定義
// 他のレイヤーから参照される基本的な抽象化を提供

pub mod traits;
pub mod types;
pub mod error;

// 公開API - 明示的にエクスポートして曖昧性を回避
pub use traits::{ProcessingConfig, ProgressReporter, HashPersistence, ParallelProcessor};
pub use types::{ProcessingMetadata, ProcessingSummary};
pub use types::ProcessingResult as ProcessingResultEnum;
pub use error::{ProcessingError, ProcessingResult};