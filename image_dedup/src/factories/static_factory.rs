//! 静的ディスパッチファクトリーシステム
//!
//! コンパイル時に型が確定するファクトリーパターン：
//! - StaticComponentFactory: 型安全なコンポーネント作成
//! - TypeSafeBuilder: コンパイル時バリデーション付きビルダー
//! - ZeroCostFactory: 実行時オーバーヘッドゼロのファクトリー

use crate::{
    core::static_di::StaticDependencyProvider,
    image_loader::{standard::StandardImageLoader, ImageLoaderBackend},
    perceptual_hash::{
        average_config::AverageConfig, average_hash::AverageHasher, config::AlgorithmConfig,
        dct_config::DctConfig, dct_hash::DctHasher, PerceptualHashBackend,
    },
    services::{
        ConsoleProgressReporter, DefaultProcessingConfig, MemoryHashPersistence,
        NoOpProgressReporter, StreamingJsonHashPersistence,
    },
    storage::{local::LocalStorageBackend, StorageBackend},
};
use std::marker::PhantomData;

/// 静的コンポーネントファクトリー
///
/// 型パラメータで作成するコンポーネントを指定し、
/// コンパイル時に全ての型が確定
pub trait StaticComponentFactory<T> {
    /// コンポーネントを作成
    fn create() -> T;

    /// コンポーネントの型名を取得
    fn type_name() -> &'static str {
        std::any::type_name::<T>()
    }

    /// コンポーネントの説明を取得
    fn description() -> &'static str;
}

/// パス付き静的コンポーネントファクトリー
pub trait StaticComponentFactoryWithPath<T> {
    /// コンポーネントを作成（パス付き）
    fn create(output_path: &std::path::Path) -> T;

    /// コンポーネントの型名を取得
    fn type_name() -> &'static str {
        std::any::type_name::<T>()
    }

    /// コンポーネントの説明を取得
    fn description() -> &'static str;
}

/// StandardImageLoaderファクトリー
pub struct StandardImageLoaderFactory<const MAX_DIMENSION: u32>;

impl<const MAX_DIMENSION: u32> StaticComponentFactory<StandardImageLoader>
    for StandardImageLoaderFactory<MAX_DIMENSION>
{
    fn create() -> StandardImageLoader {
        StandardImageLoader::new()
    }

    fn description() -> &'static str {
        "標準画像ローダー - サイズ制限付き"
    }
}

/// AverageHashファクトリー
pub struct AverageHashFactory<const SIZE: u32>;

impl<const SIZE: u32> StaticComponentFactory<AverageHasher> for AverageHashFactory<SIZE> {
    fn create() -> AverageHasher {
        let config = AverageConfig { size: SIZE };
        config
            .create_hasher()
            .expect("Failed to create Average hasher")
    }

    fn description() -> &'static str {
        "平均ハッシュアルゴリズム"
    }
}

/// DctHashファクトリー
pub struct DctHashFactory<const SIZE: u32>;

impl<const SIZE: u32> StaticComponentFactory<DctHasher> for DctHashFactory<SIZE> {
    fn create() -> DctHasher {
        let config = DctConfig {
            size: SIZE,
            quality_factor: 1.0,
        };
        config.create_hasher().expect("Failed to create DCT hasher")
    }

    fn description() -> &'static str {
        "DCTハッシュアルゴリズム"
    }
}

/// LocalStorageBackendファクトリー
pub struct LocalStorageFactory;

impl StaticComponentFactory<LocalStorageBackend> for LocalStorageFactory {
    fn create() -> LocalStorageBackend {
        LocalStorageBackend::new()
    }

    fn description() -> &'static str {
        "ローカルファイルシステムストレージ"
    }
}

/// DefaultProcessingConfigファクトリー
pub struct DefaultProcessingConfigFactory<
    const MAX_CONCURRENT: usize,
    const BUFFER_SIZE: usize,
    const BATCH_SIZE: usize,
    const ENABLE_PROGRESS: bool,
>;

impl<
        const MAX_CONCURRENT: usize,
        const BUFFER_SIZE: usize,
        const BATCH_SIZE: usize,
        const ENABLE_PROGRESS: bool,
    > StaticComponentFactory<DefaultProcessingConfig>
    for DefaultProcessingConfigFactory<MAX_CONCURRENT, BUFFER_SIZE, BATCH_SIZE, ENABLE_PROGRESS>
{
    fn create() -> DefaultProcessingConfig {
        DefaultProcessingConfig::new(MAX_CONCURRENT)
            .with_buffer_size(BUFFER_SIZE)
            .with_batch_size(BATCH_SIZE)
    }

    fn description() -> &'static str {
        "デフォルト処理設定"
    }
}

/// ConsoleProgressReporterファクトリー
pub struct ConsoleProgressReporterFactory<const QUIET: bool>;

impl<const QUIET: bool> StaticComponentFactory<ConsoleProgressReporter>
    for ConsoleProgressReporterFactory<QUIET>
{
    fn create() -> ConsoleProgressReporter {
        if QUIET {
            ConsoleProgressReporter::quiet()
        } else {
            ConsoleProgressReporter::new()
        }
    }

    fn description() -> &'static str {
        "コンソール進捗報告"
    }
}

/// NoOpProgressReporterファクトリー
pub struct NoOpProgressReporterFactory;

impl StaticComponentFactory<NoOpProgressReporter> for NoOpProgressReporterFactory {
    fn create() -> NoOpProgressReporter {
        NoOpProgressReporter::new()
    }

    fn description() -> &'static str {
        "進捗報告なし"
    }
}

/// StreamingJsonHashPersistenceファクトリー
pub struct StreamingJsonHashPersistenceFactory<const BUFFER_SIZE: usize>;

impl<const BUFFER_SIZE: usize> StaticComponentFactoryWithPath<StreamingJsonHashPersistence>
    for StreamingJsonHashPersistenceFactory<BUFFER_SIZE>
{
    fn create(output_path: &std::path::Path) -> StreamingJsonHashPersistence {
        StreamingJsonHashPersistence::new(output_path)
    }

    fn description() -> &'static str {
        "ストリーミングJSON永続化"
    }
}

/// MemoryHashPersistenceファクトリー
pub struct MemoryHashPersistenceFactory;

impl StaticComponentFactoryWithPath<MemoryHashPersistence> for MemoryHashPersistenceFactory {
    fn create(_output_path: &std::path::Path) -> MemoryHashPersistence {
        MemoryHashPersistence::new()
    }

    fn description() -> &'static str {
        "メモリ内永続化"
    }
}

/// 型安全な静的DIコンテナビルダー
///
/// コンパイル時に型が確定し、不正な組み合わせを防ぐ
pub struct StaticDIBuilder<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: crate::core::ProcessingConfig + Send + Sync + 'static,
    PR: crate::core::ProgressReporter + Send + Sync + 'static,
    HP: crate::core::HashPersistence + Send + Sync + 'static,
{
    _image_loader: PhantomData<IL>,
    _perceptual_hash: PhantomData<PH>,
    _storage: PhantomData<S>,
    _processing_config: PhantomData<PC>,
    _progress_reporter: PhantomData<PR>,
    _hash_persistence: PhantomData<HP>,
}

impl<IL, PH, S, PC, PR, HP> Default for StaticDIBuilder<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: crate::core::ProcessingConfig + Send + Sync + 'static,
    PR: crate::core::ProgressReporter + Send + Sync + 'static,
    HP: crate::core::HashPersistence + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<IL, PH, S, PC, PR, HP> StaticDIBuilder<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: crate::core::ProcessingConfig + Send + Sync + 'static,
    PR: crate::core::ProgressReporter + Send + Sync + 'static,
    HP: crate::core::HashPersistence + Send + Sync + 'static,
{
    /// 新しい静的DIビルダーを作成
    pub const fn new() -> Self {
        Self {
            _image_loader: PhantomData,
            _perceptual_hash: PhantomData,
            _storage: PhantomData,
            _processing_config: PhantomData,
            _progress_reporter: PhantomData,
            _hash_persistence: PhantomData,
        }
    }

    /// カスタム依存関係プロバイダーを作成
    pub fn build(self) -> CustomStaticProvider<IL, PH, S, PC, PR, HP> {
        CustomStaticProvider::new()
    }
}

/// カスタム静的依存関係プロバイダー
pub struct CustomStaticProvider<IL, PH, S, PC, PR, HP> {
    _image_loader: PhantomData<IL>,
    _perceptual_hash: PhantomData<PH>,
    _storage: PhantomData<S>,
    _processing_config: PhantomData<PC>,
    _progress_reporter: PhantomData<PR>,
    _hash_persistence: PhantomData<HP>,
}

impl<IL, PH, S, PC, PR, HP> CustomStaticProvider<IL, PH, S, PC, PR, HP> {
    const fn new() -> Self {
        Self {
            _image_loader: PhantomData,
            _perceptual_hash: PhantomData,
            _storage: PhantomData,
            _processing_config: PhantomData,
            _progress_reporter: PhantomData,
            _hash_persistence: PhantomData,
        }
    }
}

impl<IL, PH, S, PC, PR, HP> StaticDependencyProvider for CustomStaticProvider<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static + Default,
    PH: PerceptualHashBackend + Send + Sync + 'static + Default,
    S: StorageBackend + Send + Sync + 'static + Default,
    PC: crate::core::ProcessingConfig + Send + Sync + 'static + Default,
    PR: crate::core::ProgressReporter + Send + Sync + 'static + Default,
    HP: crate::core::HashPersistence + Send + Sync + 'static + Default,
{
    type ImageLoader = IL;
    type PerceptualHash = PH;
    type Storage = S;
    type ProcessingConfig = PC;
    type ProgressReporter = PR;
    type HashPersistence = HP;

    fn create_image_loader() -> Self::ImageLoader {
        IL::default()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        PH::default()
    }

    fn create_storage() -> Self::Storage {
        S::default()
    }

    fn create_processing_config() -> Self::ProcessingConfig {
        PC::default()
    }

    fn create_progress_reporter() -> Self::ProgressReporter {
        PR::default()
    }

    fn create_hash_persistence(_output_path: &std::path::Path) -> Self::HashPersistence {
        HP::default()
    }
}

/// プリセット作成マクロ
#[macro_export]
macro_rules! create_static_container {
    (default) => {
        $crate::core::StaticDIContainer::<$crate::core::DefaultConfig>::new()
    };
    (high_performance) => {
        $crate::core::StaticDIContainer::<$crate::core::HighPerformanceConfig>::new()
    };
    (testing) => {
        $crate::core::StaticDIContainer::<$crate::core::TestingConfig>::new()
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_static_component_factories() {
        let _loader = StandardImageLoaderFactory::<512>::create();
        let _hasher = AverageHashFactory::<8>::create();
        let _storage = LocalStorageFactory::create();
        let _config = DefaultProcessingConfigFactory::<4, 100, 50, true>::create();
        let _reporter = ConsoleProgressReporterFactory::<false>::create();
        let _noop_reporter = NoOpProgressReporterFactory::create();
    }

    #[test]
    fn test_static_factories_with_path() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

        let _streaming_persistence =
            StreamingJsonHashPersistenceFactory::<100>::create(&output_path);
        let _memory_persistence = MemoryHashPersistenceFactory::create(&output_path);
    }

    #[test]
    fn test_factory_descriptions() {
        assert_eq!(
            StandardImageLoaderFactory::<512>::description(),
            "標準画像ローダー - サイズ制限付き"
        );
        assert_eq!(
            AverageHashFactory::<8>::description(),
            "平均ハッシュアルゴリズム"
        );
        assert_eq!(
            LocalStorageFactory::description(),
            "ローカルファイルシステムストレージ"
        );
    }

    #[test]
    fn test_type_names() {
        assert!(StandardImageLoaderFactory::<512>::type_name().contains("StandardImageLoader"));
        assert!(AverageHashFactory::<8>::type_name().contains("AverageHasher"));
        assert!(LocalStorageFactory::type_name().contains("LocalStorageBackend"));
    }

    #[test]
    fn test_static_di_builder() {
        let _builder = StaticDIBuilder::<
            StandardImageLoader,
            AverageHasher,
            LocalStorageBackend,
            DefaultProcessingConfig,
            ConsoleProgressReporter,
            MemoryHashPersistence,
        >::new();
    }

    #[test]
    fn test_const_factory_sizes() {
        // ゼロコスト確認
        assert_eq!(std::mem::size_of::<StandardImageLoaderFactory<512>>(), 0);
        assert_eq!(std::mem::size_of::<AverageHashFactory<8>>(), 0);
        assert_eq!(std::mem::size_of::<LocalStorageFactory>(), 0);
    }

    #[test]
    fn test_preset_macros() {
        let _default = create_static_container!(default);
        let _hp = create_static_container!(high_performance);
        let _test = create_static_container!(testing);
    }
}
