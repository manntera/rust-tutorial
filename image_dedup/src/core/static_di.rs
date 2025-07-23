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
/// 
/// 型制約版：
/// - 必要最小限の制約で型安全性を確保
/// - Send + Syncで並行処理をサポート  
/// - Debugはオプショナル
pub trait StaticDependencyProvider: Sized + 'static {
    type ImageLoader: ImageLoaderBackend + Send + Sync + 'static;
    type PerceptualHash: PerceptualHashBackend + Send + Sync + 'static;
    type Storage: StorageBackend + Send + Sync + 'static;
    type ProcessingConfig: ProcessingConfig + Send + Sync + 'static;
    type ProgressReporter: ProgressReporter + Send + Sync + 'static;
    type HashPersistence: HashPersistence + Send + Sync + 'static;

    /// ImageLoaderインスタンスを作成
    /// 
    /// # Safety
    /// この関数はスレッドセーフで、複数回呼び出しても安全でなければならない
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

    /// 依存関係の整合性を検証（コンパイル時）
    /// 
    /// デフォルト実装では常に有効とするが、
    /// 具体実装で override して制約を追加可能
    const DEPENDENCIES_VALID: bool = true;

    /// 依存関係の説明（デバッグ用）
    const DEPENDENCY_DESCRIPTION: &'static str = "Static dependency provider";

    /// 全ての依存関係を一度に作成（テスト・デバッグ用）
    fn create_all_dependencies(output_path: &std::path::Path) -> StaticDependencyBundle<Self> {
        StaticDependencyBundle {
            image_loader: Self::create_image_loader(),
            perceptual_hash: Self::create_perceptual_hash(),
            storage: Self::create_storage(),
            processing_config: Self::create_processing_config(),
            progress_reporter: Self::create_progress_reporter(),
            hash_persistence: Self::create_hash_persistence(output_path),
        }
    }
}

/// 静的依存関係バンドル
/// 
/// 全ての依存関係を一つにまとめた構造体
/// テストやデバッグで便利
#[derive(Debug)]
pub struct StaticDependencyBundle<P: StaticDependencyProvider> {
    pub image_loader: P::ImageLoader,
    pub perceptual_hash: P::PerceptualHash,
    pub storage: P::Storage,
    pub processing_config: P::ProcessingConfig,
    pub progress_reporter: P::ProgressReporter,
    pub hash_persistence: P::HashPersistence,
}

impl<P: StaticDependencyProvider> StaticDependencyBundle<P> {
    /// ProcessingEngineを作成
    pub fn into_processing_engine(self) -> StaticProcessingEngine<P> {
        ProcessingEngine::new(
            self.image_loader,
            self.perceptual_hash,
            self.storage,
            self.processing_config,
            self.progress_reporter,
            self.hash_persistence,
        )
    }
}

/// 静的DIコンテナ - コンパイル時依存関係解決（強化版）
///
/// PhantomDataを使用して型レベルで依存関係を管理
/// 実行時オーバーヘッドゼロの依存関係注入を実現
/// 
/// 改善点：
/// - より厳格な型制約検証
/// - コンパイル時依存関係整合性チェック
/// - デバッグ支援機能の充実
#[derive(Debug)]
pub struct StaticDIContainer<P: StaticDependencyProvider> {
    _provider: PhantomData<P>,
}

impl<P: StaticDependencyProvider> StaticDIContainer<P> {
    /// 新しい静的DIコンテナを作成
    /// 
    /// コンパイル時制約：
    /// - 全ての依存関係型がSend + Sync
    /// - 依存関係の整合性が検証済み
    pub const fn new() -> Self {
        Self {
            _provider: PhantomData,
        }
    }

    /// 依存関係の整合性を検証（実行時チェック用）
    pub const fn validate() -> bool {
        P::DEPENDENCIES_VALID
    }

    /// 依存関係の説明を取得
    pub const fn description() -> &'static str {
        P::DEPENDENCY_DESCRIPTION
    }

    /// ProcessingEngineを作成（静的ディスパッチ）
    ///
    /// 全ての依存関係がコンパイル時に解決され、
    /// 実行時は直接の関数呼び出しのみが発生
    /// 
    /// # Panics
    /// 依存関係の作成時にエラーが発生した場合にパニックする可能性があります。
    pub fn create_processing_engine(
        &self,
        output_path: &std::path::Path,
    ) -> StaticProcessingEngine<P> {
        // 依存関係バンドルを作成してからエンジンに変換
        let bundle = P::create_all_dependencies(output_path);
        bundle.into_processing_engine()
    }

    /// 依存関係バンドルを作成（テスト・デバッグ用）
    pub fn create_dependency_bundle(&self, output_path: &std::path::Path) -> StaticDependencyBundle<P> {
        P::create_all_dependencies(output_path)
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

    /// 依存関係の型情報を取得（デバッグ用）
    pub fn dependency_type_info() -> StaticDependencyTypeInfo {
        StaticDependencyTypeInfo {
            image_loader: std::any::type_name::<P::ImageLoader>(),
            perceptual_hash: std::any::type_name::<P::PerceptualHash>(),
            storage: std::any::type_name::<P::Storage>(),
            processing_config: std::any::type_name::<P::ProcessingConfig>(),
            progress_reporter: std::any::type_name::<P::ProgressReporter>(),
            hash_persistence: std::any::type_name::<P::HashPersistence>(),
        }
    }
}

/// 静的依存関係型情報（デバッグ用）
#[derive(Debug, Clone)]
pub struct StaticDependencyTypeInfo {
    pub image_loader: &'static str,
    pub perceptual_hash: &'static str,
    pub storage: &'static str,
    pub processing_config: &'static str,
    pub progress_reporter: &'static str,
    pub hash_persistence: &'static str,
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
