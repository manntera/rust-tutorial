//! Factory抽象化モジュール - 全ての依存関係をFactory Patternで管理
//! 
//! Rustの哲学に基づく設計：
//! - Zero-cost abstraction: トレイトによる抽象化
//! - Type safety: コンパイル時の型チェック
//! - Ownership-based: 明確な所有権管理

pub mod image_loader_factory;
pub mod perceptual_hash_factory;
pub mod storage_factory;
pub mod processing_config_factory;
pub mod progress_reporter_factory;
pub mod hash_persistence_factory;
pub mod static_factory;

// 各Factoryの再エクスポート
pub use image_loader_factory::ImageLoaderFactory;
pub use perceptual_hash_factory::PerceptualHashFactory;
pub use storage_factory::StorageFactory;
pub use processing_config_factory::ProcessingConfigFactory;
pub use progress_reporter_factory::ProgressReporterFactory;
pub use hash_persistence_factory::HashPersistenceFactory;

// 静的ファクトリーの再エクスポート
pub use static_factory::{
    StaticComponentFactory, StaticComponentFactoryWithPath,
    StandardImageLoaderFactory, AverageHashFactory, DctHashFactory,
    LocalStorageFactory, DefaultProcessingConfigFactory,
    ConsoleProgressReporterFactory, NoOpProgressReporterFactory,
    StreamingJsonHashPersistenceFactory, MemoryHashPersistenceFactory,
    StaticDIBuilder, CustomStaticProvider,
};

use crate::core::ComponentConfig;
use anyhow::Result;

/// 共通Factory trait - 全てのFactoryが実装すべき基底インターフェース
pub trait ComponentFactory<T> {
    /// コンポーネント設定から実装を作成
    fn create(&self, config: &ComponentConfig) -> Result<T>;
    
    /// 利用可能な実装の一覧を取得
    fn available_implementations(&self) -> Vec<String>;
    
    /// 実装の説明を取得
    fn get_description(&self, implementation: &str) -> Option<String>;
}

/// 出力パス付きFactory trait - HashPersistenceなど出力先が必要なコンポーネント用
pub trait ComponentFactoryWithPath<T> {
    /// コンポーネント設定と出力パスから実装を作成
    fn create(&self, config: &ComponentConfig, output_path: &std::path::Path) -> Result<T>;
    
    /// 利用可能な実装の一覧を取得
    fn available_implementations(&self) -> Vec<String>;
    
    /// 実装の説明を取得
    fn get_description(&self, implementation: &str) -> Option<String>;
}