// コアレイヤー - 基盤となるトレイト、型、エラー定義
// 他のレイヤーから参照される基本的な抽象化を提供

pub mod error;
pub mod static_config;
pub mod static_di;
pub mod traits;
pub mod types;

// 公開API - 明示的にエクスポートして曖昧性を回避
pub use error::{ProcessingError, ProcessingResult};
pub use static_config::{
    CustomConfig, CustomConfigBuilder, DefaultConfig, HighPerformanceConfig, PerformanceLevel,
    TestingConfig, TypeConfig,
};
pub use static_di::{StaticDIContainer, StaticDependencyProvider, StaticProcessingEngine};
pub use traits::{HashPersistence, ParallelProcessor, ProcessingConfig, ProgressReporter};
pub use types::ProcessingOutcome;
pub use types::{ProcessingMetadata, ProcessingSummary};
