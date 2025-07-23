//! 静的ディスパッチ中心の依存関係注入システム
//!
//! Rustの哲学に基づく真のゼロコスト抽象化を実現：
//! - コンパイル時依存関係解決
//! - 静的ディスパッチによる最適化
//! - 型安全性の最大化

use crate::{
    core::{HashPersistence, ProcessingConfig, ProgressReporter},
    engine::ProcessingEngine,
    image_loader::ImageLoaderBackend,
    perceptual_hash::PerceptualHashBackend,
    storage::StorageBackend,
};
use std::marker::PhantomData;

/// 静的ProcessingEngineの型エイリアス
pub type StaticProcessingEngine<P> = ProcessingEngine<
    <P as StaticDependencyProvider>::ImageLoader,
    <P as StaticDependencyProvider>::PerceptualHash,
    <P as StaticDependencyProvider>::Storage,
    <P as StaticDependencyProvider>::ProcessingConfig,
    <P as StaticDependencyProvider>::ProgressReporter,
    <P as StaticDependencyProvider>::HashPersistence,
>;

/// 型レベル依存関係提供者
///
/// 各コンポーネントの具象型を型パラメータで指定し、
/// コンパイル時に全ての依存関係を解決
pub trait StaticDependencyProvider {
    type ImageLoader: ImageLoaderBackend + Send + Sync + 'static;
    type PerceptualHash: PerceptualHashBackend + Send + Sync + 'static;
    type Storage: StorageBackend + Send + Sync + 'static;
    type ProcessingConfig: ProcessingConfig + Send + Sync + 'static;
    type ProgressReporter: ProgressReporter + Send + Sync + 'static;
    type HashPersistence: HashPersistence + Send + Sync + 'static;

    /// ImageLoaderインスタンスを作成
    fn create_image_loader() -> Self::ImageLoader;

    /// PerceptualHashインスタンスを作成
    fn create_perceptual_hash() -> Self::PerceptualHash;

    /// Storageインスタンスを作成
    fn create_storage() -> Self::Storage;

    /// ProcessingConfigインスタンスを作成
    fn create_processing_config() -> Self::ProcessingConfig;

    /// ProgressReporterインスタンスを作成
    fn create_progress_reporter() -> Self::ProgressReporter;

    /// HashPersistenceインスタンスを作成
    fn create_hash_persistence(output_path: &std::path::Path) -> Self::HashPersistence;
}

/// 静的DIコンテナ - コンパイル時依存関係解決
///
/// PhantomDataを使用して型レベルで依存関係を管理
/// 実行時オーバーヘッドゼロの依存関係注入を実現
pub struct StaticDIContainer<P: StaticDependencyProvider> {
    _provider: PhantomData<P>,
}

impl<P: StaticDependencyProvider> StaticDIContainer<P> {
    /// 新しい静的DIコンテナを作成
    pub const fn new() -> Self {
        Self {
            _provider: PhantomData,
        }
    }

    /// ProcessingEngineを作成（静的ディスパッチ）
    ///
    /// 全ての依存関係がコンパイル時に解決され、
    /// 実行時は直接の関数呼び出しのみが発生
    pub fn create_processing_engine(
        &self,
        output_path: &std::path::Path,
    ) -> StaticProcessingEngine<P> {
        ProcessingEngine::new(
            P::create_image_loader(),
            P::create_perceptual_hash(),
            P::create_storage(),
            P::create_processing_config(),
            P::create_progress_reporter(),
            P::create_hash_persistence(output_path),
        )
    }

    /// 個別の依存関係を作成（テスト用）
    pub fn create_image_loader(&self) -> P::ImageLoader {
        P::create_image_loader()
    }

    pub fn create_perceptual_hash(&self) -> P::PerceptualHash {
        P::create_perceptual_hash()
    }

    pub fn create_storage(&self) -> P::Storage {
        P::create_storage()
    }

    pub fn create_processing_config(&self) -> P::ProcessingConfig {
        P::create_processing_config()
    }

    pub fn create_progress_reporter(&self) -> P::ProgressReporter {
        P::create_progress_reporter()
    }

    pub fn create_hash_persistence(&self, output_path: &std::path::Path) -> P::HashPersistence {
        P::create_hash_persistence(output_path)
    }
}

impl<P: StaticDependencyProvider> Default for StaticDIContainer<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: StaticDependencyProvider> Clone for StaticDIContainer<P> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image_loader::standard::StandardImageLoader,
        perceptual_hash::{
            average_config::AverageConfig, average_hash::AverageHasher, config::AlgorithmConfig,
        },
        services::{ConsoleProgressReporter, DefaultProcessingConfig, MemoryHashPersistence},
        storage::local::LocalStorageBackend,
    };

    /// テスト用依存関係プロバイダー
    struct TestDependencyProvider;

    impl StaticDependencyProvider for TestDependencyProvider {
        type ImageLoader = StandardImageLoader;
        type PerceptualHash = AverageHasher;
        type Storage = LocalStorageBackend;
        type ProcessingConfig = DefaultProcessingConfig;
        type ProgressReporter = ConsoleProgressReporter;
        type HashPersistence = MemoryHashPersistence;

        fn create_image_loader() -> Self::ImageLoader {
            StandardImageLoader::new()
        }

        fn create_perceptual_hash() -> Self::PerceptualHash {
            let config = AverageConfig { size: 8 };
            config
                .create_hasher()
                .expect("Failed to create Average hasher")
        }

        fn create_storage() -> Self::Storage {
            LocalStorageBackend::new()
        }

        fn create_processing_config() -> Self::ProcessingConfig {
            DefaultProcessingConfig::new(4)
        }

        fn create_progress_reporter() -> Self::ProgressReporter {
            ConsoleProgressReporter::new()
        }

        fn create_hash_persistence(_output_path: &std::path::Path) -> Self::HashPersistence {
            MemoryHashPersistence::new()
        }
    }

    #[test]
    fn test_static_di_container_creation() {
        let container = StaticDIContainer::<TestDependencyProvider>::new();

        // コンパイル時に型が確定していることを確認
        let _loader = container.create_image_loader();
        let _hasher = container.create_perceptual_hash();
        let _storage = container.create_storage();
        let _config = container.create_processing_config();
        let _reporter = container.create_progress_reporter();
    }

    #[test]
    fn test_static_processing_engine_creation() {
        let container = StaticDIContainer::<TestDependencyProvider>::new();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

        let _engine = container.create_processing_engine(&output_path);

        // 静的型エイリアスの確認
        let _: StaticProcessingEngine<TestDependencyProvider> =
            container.create_processing_engine(&output_path);
    }

    #[test]
    fn test_zero_runtime_overhead() {
        // PhantomDataのサイズ確認（ゼロコスト）
        assert_eq!(
            std::mem::size_of::<StaticDIContainer<TestDependencyProvider>>(),
            0
        );
    }

    #[test]
    fn test_const_creation() {
        // コンパイル時作成の確認
        const _CONTAINER: StaticDIContainer<TestDependencyProvider> = StaticDIContainer::new();
    }
}
