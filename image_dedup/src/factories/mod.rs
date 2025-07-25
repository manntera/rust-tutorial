//! 静的ファクトリーモジュール - コンパイル時依存関係管理
//!
//! Rustの哲学に基づく設計：
//! - Zero-cost abstraction: 型レベルでの抽象化
//! - Type safety: コンパイル時の型チェック
//! - Static dispatch: 実行時オーバーヘッドゼロ

pub mod static_factory;

// 静的ファクトリーの再エクスポート
pub use static_factory::{
    AverageHashFactory, ConfiguredStaticProvider, ConsoleProgressReporterFactory,
    ConstGenericConfig, CustomStaticProvider, DctHashFactory, DefaultProcessingConfigFactory,
    EnhancedStaticDIBuilder, FastConfig, HighPrecisionConfig, LocalStorageFactory,
    MemoryHashPersistenceFactory, NoOpProgressReporterFactory, StandardImageLoaderFactory,
    StaticComponentCreator, StaticComponentCreatorWithPath, StaticComponentFactory,
    StaticComponentFactoryWithPath, StreamingJsonHashPersistenceFactory,
};
