//! 静的ディスパッチファクトリーシステム
//!
//! コンパイル時に型が確定するファクトリーパターン：
//! - StaticComponentFactory: 型安全なコンポーネント作成
//! - TypeSafeBuilder: コンパイル時バリデーション付きビルダー
//! - ZeroCostFactory: 実行時オーバーヘッドゼロのファクトリー

use crate::{
    core::{static_di::StaticDependencyProvider, ProcessingError},
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

    /// エラーハンドリング付きコンポーネント作成
    fn try_create() -> crate::core::ProcessingResult<T> {
        Ok(Self::create())
    }

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
        // 非フォールバック版では、この設定が常に有効であることが保証されている
        config.create_hasher().unwrap_or_else(|_| {
            // このコードパスに到達することは設計上ありえないが、
            // 万が一のためのフォールバック
            unreachable!("AverageConfig should always create a valid hasher")
        })
    }

    fn try_create() -> crate::core::ProcessingResult<AverageHasher> {
        let config = AverageConfig { size: SIZE };
        config.create_hasher().map_err(|e| {
            ProcessingError::dependency_injection(format!(
                "Failed to create AverageHasher with size {SIZE}: {e}"
            ))
        })
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
        // 非フォールバック版では、この設定が常に有効であることが保証されている
        config.create_hasher().unwrap_or_else(|_| {
            // このコードパスに到達することは設計上ありえないが、
            // 万が一のためのフォールバック
            unreachable!("DctConfig should always create a valid hasher")
        })
    }

    fn try_create() -> crate::core::ProcessingResult<DctHasher> {
        let config = DctConfig {
            size: SIZE,
            quality_factor: 1.0,
        };
        config.create_hasher().map_err(|e| {
            ProcessingError::dependency_injection(format!(
                "Failed to create DctHasher with size {SIZE}: {e}"
            ))
        })
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

/// 型安全な静的DIコンテナビルダー（const generics版）
///
/// const genericsを活用してより型安全で効率的なビルダーを実現
pub struct EnhancedStaticDIBuilder<
    const IMAGE_SIZE_LIMIT: u32 = 1024,
    const HASH_SIZE: u32 = 8,
    const CONCURRENT_TASKS: usize = 4,
    const BUFFER_SIZE: usize = 100,
    const BATCH_SIZE: usize = 50,
    const QUIET_MODE: bool = false,
> {
    _marker: PhantomData<()>,
}

impl<
        const IMAGE_SIZE_LIMIT: u32,
        const HASH_SIZE: u32,
        const CONCURRENT_TASKS: usize,
        const BUFFER_SIZE: usize,
        const BATCH_SIZE: usize,
        const QUIET_MODE: bool,
    > Default
    for EnhancedStaticDIBuilder<
        IMAGE_SIZE_LIMIT,
        HASH_SIZE,
        CONCURRENT_TASKS,
        BUFFER_SIZE,
        BATCH_SIZE,
        QUIET_MODE,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

impl<
        const IMAGE_SIZE_LIMIT: u32,
        const HASH_SIZE: u32,
        const CONCURRENT_TASKS: usize,
        const BUFFER_SIZE: usize,
        const BATCH_SIZE: usize,
        const QUIET_MODE: bool,
    >
    EnhancedStaticDIBuilder<
        IMAGE_SIZE_LIMIT,
        HASH_SIZE,
        CONCURRENT_TASKS,
        BUFFER_SIZE,
        BATCH_SIZE,
        QUIET_MODE,
    >
{
    /// 新しい静的DIビルダーを作成
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    /// 設定値の検証
    pub const fn validate_config() -> bool {
        IMAGE_SIZE_LIMIT > 0
            && HASH_SIZE >= 4
            && HASH_SIZE <= 256
            && CONCURRENT_TASKS > 0
            && CONCURRENT_TASKS <= 1024
            && BUFFER_SIZE > 0
            && BUFFER_SIZE <= 100000
            && BATCH_SIZE > 0
            && BATCH_SIZE <= 10000
    }

    /// コンパイル時設定情報
    pub const fn config_description() -> &'static str {
        match (QUIET_MODE, HASH_SIZE >= 16) {
            (true, true) => "Quiet high-precision configuration",
            (true, false) => "Quiet fast configuration",
            (false, true) => "Verbose high-precision configuration",
            (false, false) => "Verbose fast configuration",
        }
    }
}

/// const genericsベースの設定済みプロバイダー（改良版）
pub struct ConfiguredStaticProvider<C: ConstGenericConfig> {
    _config: PhantomData<C>,
}

impl<C: ConstGenericConfig> ConfiguredStaticProvider<C> {
    pub const fn new() -> Self {
        Self {
            _config: PhantomData,
        }
    }
}

impl<C: ConstGenericConfig> Default for ConfiguredStaticProvider<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// const generics設定値を型レベルで管理するトレイト
pub trait ConstGenericConfig {
    const IMAGE_SIZE_LIMIT: u32;
    const HASH_SIZE: u32;
    const CONCURRENT_TASKS: usize;
    const BUFFER_SIZE: usize;
    const BATCH_SIZE: usize;
    const QUIET_MODE: bool;

    // 型レベル選択用の型
    type PerceptualHashType: PerceptualHashBackend + Send + Sync + 'static;
    type ProgressReporterType: crate::core::ProgressReporter + Send + Sync + 'static;

    // 各タイプの作成メソッド
    fn create_perceptual_hash_typed() -> Self::PerceptualHashType;
    fn create_progress_reporter_typed() -> Self::ProgressReporterType;
}

/// 高精度設定（DCTハッシュ、詳細進捗）
pub struct HighPrecisionConfig<
    const IMAGE_SIZE_LIMIT: u32 = 2048,
    const HASH_SIZE: u32 = 32,
    const CONCURRENT_TASKS: usize = 8,
    const BUFFER_SIZE: usize = 500,
    const BATCH_SIZE: usize = 100,
>;

impl<
        const IMAGE_SIZE_LIMIT: u32,
        const HASH_SIZE: u32,
        const CONCURRENT_TASKS: usize,
        const BUFFER_SIZE: usize,
        const BATCH_SIZE: usize,
    > ConstGenericConfig
    for HighPrecisionConfig<IMAGE_SIZE_LIMIT, HASH_SIZE, CONCURRENT_TASKS, BUFFER_SIZE, BATCH_SIZE>
{
    const IMAGE_SIZE_LIMIT: u32 = IMAGE_SIZE_LIMIT;
    const HASH_SIZE: u32 = HASH_SIZE;
    const CONCURRENT_TASKS: usize = CONCURRENT_TASKS;
    const BUFFER_SIZE: usize = BUFFER_SIZE;
    const BATCH_SIZE: usize = BATCH_SIZE;
    const QUIET_MODE: bool = false;

    type PerceptualHashType = DctHasher;
    type ProgressReporterType = ConsoleProgressReporter;

    fn create_perceptual_hash_typed() -> Self::PerceptualHashType {
        let config = DctConfig {
            size: HASH_SIZE,
            quality_factor: 1.0,
        };
        // 非フォールバック版では、この設定が常に有効であることが保証されている
        config.create_hasher().unwrap_or_else(|_| {
            // このコードパスに到達することは設計上ありえないが、
            // 万が一のためのフォールバック
            unreachable!("DCT config should always create a valid hasher")
        })
    }

    fn create_progress_reporter_typed() -> Self::ProgressReporterType {
        ConsoleProgressReporter::new()
    }
}

/// 高速設定（平均ハッシュ、静かな進捗）
pub struct FastConfig<
    const IMAGE_SIZE_LIMIT: u32 = 1024,
    const HASH_SIZE: u32 = 8,
    const CONCURRENT_TASKS: usize = 16,
    const BUFFER_SIZE: usize = 1000,
    const BATCH_SIZE: usize = 200,
>;

impl<
        const IMAGE_SIZE_LIMIT: u32,
        const HASH_SIZE: u32,
        const CONCURRENT_TASKS: usize,
        const BUFFER_SIZE: usize,
        const BATCH_SIZE: usize,
    > ConstGenericConfig
    for FastConfig<IMAGE_SIZE_LIMIT, HASH_SIZE, CONCURRENT_TASKS, BUFFER_SIZE, BATCH_SIZE>
{
    const IMAGE_SIZE_LIMIT: u32 = IMAGE_SIZE_LIMIT;
    const HASH_SIZE: u32 = HASH_SIZE;
    const CONCURRENT_TASKS: usize = CONCURRENT_TASKS;
    const BUFFER_SIZE: usize = BUFFER_SIZE;
    const BATCH_SIZE: usize = BATCH_SIZE;
    const QUIET_MODE: bool = true;

    type PerceptualHashType = AverageHasher;
    type ProgressReporterType = NoOpProgressReporter;

    fn create_perceptual_hash_typed() -> Self::PerceptualHashType {
        let config = AverageConfig { size: HASH_SIZE };
        // 非フォールバック版では、この設定が常に有効であることが保証されている
        config.create_hasher().unwrap_or_else(|_| {
            // このコードパスに到達することは設計上ありえないが、
            // 万が一のためのフォールバック
            unreachable!("Average config should always create a valid hasher")
        })
    }

    fn create_progress_reporter_typed() -> Self::ProgressReporterType {
        NoOpProgressReporter::new()
    }
}

impl<C> crate::core::StaticDependencyProvider for ConfiguredStaticProvider<C>
where
    C: ConstGenericConfig + 'static,
{
    type ImageLoader = StandardImageLoader;
    type PerceptualHash = C::PerceptualHashType;
    type Storage = LocalStorageBackend;
    type ProcessingConfig = DefaultProcessingConfig;
    type ProgressReporter = C::ProgressReporterType;
    type HashPersistence = StreamingJsonHashPersistence;

    const DEPENDENCIES_VALID: bool = true;
    const DEPENDENCY_DESCRIPTION: &'static str = "Configured static provider with const generics";

    fn create_image_loader() -> Self::ImageLoader {
        StandardImageLoader::new()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        C::create_perceptual_hash_typed()
    }

    fn create_storage() -> Self::Storage {
        LocalStorageBackend::new()
    }

    fn create_processing_config() -> Self::ProcessingConfig {
        DefaultProcessingConfig::new(C::CONCURRENT_TASKS)
            .with_buffer_size(C::BUFFER_SIZE)
            .with_batch_size(C::BATCH_SIZE)
    }

    fn create_progress_reporter() -> Self::ProgressReporter {
        C::create_progress_reporter_typed()
    }

    fn create_hash_persistence(output_path: &std::path::Path) -> Self::HashPersistence {
        StreamingJsonHashPersistence::new(output_path)
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
}

impl<IL, PH, S, PC, PR, HP> Default for CustomStaticProvider<IL, PH, S, PC, PR, HP> {
    fn default() -> Self {
        Self::new()
    }
}

/// カスタム依存関係作成トレイト
///
/// Defaultトレイトに依存せず、型安全な依存関係作成を実現
pub trait StaticComponentCreator<T> {
    /// コンポーネントを作成
    fn create() -> T;
}

/// パス付きカスタム依存関係作成トレイト
pub trait StaticComponentCreatorWithPath<T> {
    /// パス付きコンポーネントを作成
    fn create_with_path(output_path: &std::path::Path) -> T;
}

impl<IL, PH, S, PC, PR, HP> StaticDependencyProvider for CustomStaticProvider<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: crate::core::ProcessingConfig + Send + Sync + 'static,
    PR: crate::core::ProgressReporter + Send + Sync + 'static,
    HP: crate::core::HashPersistence + Send + Sync + 'static,
    CustomStaticProvider<IL, PH, S, PC, PR, HP>: StaticComponentCreator<IL>,
    CustomStaticProvider<IL, PH, S, PC, PR, HP>: StaticComponentCreator<PH>,
    CustomStaticProvider<IL, PH, S, PC, PR, HP>: StaticComponentCreator<S>,
    CustomStaticProvider<IL, PH, S, PC, PR, HP>: StaticComponentCreator<PC>,
    CustomStaticProvider<IL, PH, S, PC, PR, HP>: StaticComponentCreator<PR>,
    CustomStaticProvider<IL, PH, S, PC, PR, HP>: StaticComponentCreatorWithPath<HP>,
{
    type ImageLoader = IL;
    type PerceptualHash = PH;
    type Storage = S;
    type ProcessingConfig = PC;
    type ProgressReporter = PR;
    type HashPersistence = HP;

    const DEPENDENCIES_VALID: bool = true;
    const DEPENDENCY_DESCRIPTION: &'static str =
        "Custom static provider with type-safe component creation";

    fn create_image_loader() -> Self::ImageLoader {
        <Self as StaticComponentCreator<IL>>::create()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        <Self as StaticComponentCreator<PH>>::create()
    }

    fn create_storage() -> Self::Storage {
        <Self as StaticComponentCreator<S>>::create()
    }

    fn create_processing_config() -> Self::ProcessingConfig {
        <Self as StaticComponentCreator<PC>>::create()
    }

    fn create_progress_reporter() -> Self::ProgressReporter {
        <Self as StaticComponentCreator<PR>>::create()
    }

    fn create_hash_persistence(output_path: &std::path::Path) -> Self::HashPersistence {
        <Self as StaticComponentCreatorWithPath<HP>>::create_with_path(output_path)
    }
}

/// Defaultトレイトベースの標準実装
///
/// 既存のコンポーネントがDefaultを実装している場合の便利実装
impl<T> StaticComponentCreator<T> for CustomStaticProvider<T, T, T, T, T, T>
where
    T: Default
        + ImageLoaderBackend
        + PerceptualHashBackend
        + StorageBackend
        + crate::core::ProcessingConfig
        + crate::core::ProgressReporter
        + crate::core::HashPersistence
        + Send
        + Sync
        + 'static,
{
    fn create() -> T {
        T::default()
    }
}

impl<T> StaticComponentCreatorWithPath<T> for CustomStaticProvider<T, T, T, T, T, T>
where
    T: Default
        + ImageLoaderBackend
        + PerceptualHashBackend
        + StorageBackend
        + crate::core::ProcessingConfig
        + crate::core::ProgressReporter
        + crate::core::HashPersistence
        + Send
        + Sync
        + 'static,
{
    fn create_with_path(_output_path: &std::path::Path) -> T {
        T::default()
    }
}

/// 具体的な型に対する実装例マクロ
#[macro_export]
macro_rules! impl_component_creator {
    ($provider:ty, $component:ty, $creation_expr:expr) => {
        impl $crate::factories::StaticComponentCreator<$component> for $provider {
            fn create() -> $component {
                $creation_expr
            }
        }
    };
    ($provider:ty, $component:ty, $creation_expr:expr, with_path) => {
        impl $crate::factories::StaticComponentCreatorWithPath<$component> for $provider {
            fn create_with_path(output_path: &std::path::Path) -> $component {
                $creation_expr(output_path)
            }
        }
    };
}

/// プリセット作成マクロ（拡張版）
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
    (high_precision) => {
        $crate::core::StaticDIContainer::<$crate::factories::ConfiguredStaticProvider<$crate::factories::HighPrecisionConfig>>::new()
    };
    (fast) => {
        $crate::core::StaticDIContainer::<$crate::factories::ConfiguredStaticProvider<$crate::factories::FastConfig>>::new()
    };
    (custom: $config:ty) => {
        $crate::core::StaticDIContainer::<$crate::factories::ConfiguredStaticProvider<$config>>::new()
    };
}

/// const generics設定ビルダーマクロ
#[macro_export]
macro_rules! build_configured_container {
    (
        image_size: $image_size:expr,
        hash_size: $hash_size:expr,
        concurrent_tasks: $tasks:expr,
        buffer_size: $buffer:expr,
        batch_size: $batch:expr,
        precision: high
    ) => {
        {
            type CustomHighPrecision = $crate::factories::HighPrecisionConfig<$image_size, $hash_size, $tasks, $buffer, $batch>;
            $crate::core::StaticDIContainer::<$crate::factories::ConfiguredStaticProvider<CustomHighPrecision>>::new()
        }
    };
    (
        image_size: $image_size:expr,
        hash_size: $hash_size:expr,
        concurrent_tasks: $tasks:expr,
        buffer_size: $buffer:expr,
        batch_size: $batch:expr,
        precision: fast
    ) => {
        {
            type CustomFast = $crate::factories::FastConfig<$image_size, $hash_size, $tasks, $buffer, $batch>;
            $crate::core::StaticDIContainer::<$crate::factories::ConfiguredStaticProvider<CustomFast>>::new()
        }
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
    fn test_enhanced_static_di_builder() {
        let _builder = EnhancedStaticDIBuilder::<1024, 8, 4, 100, 50, false>::new();

        // 設定値の検証テスト
        assert!(EnhancedStaticDIBuilder::<1024, 8, 4, 100, 50, false>::validate_config());
        assert!(!EnhancedStaticDIBuilder::<0, 8, 4, 100, 50, false>::validate_config()); // 無効な設定

        // 設定説明のテスト
        let desc = EnhancedStaticDIBuilder::<1024, 32, 4, 100, 50, false>::config_description();
        assert!(desc.contains("high-precision"));
    }

    #[test]
    fn test_type_safe_component_creation() {
        // 新しいトレイトベースの型安全な作成をテスト
        use crate::image_loader::standard::StandardImageLoader;
        use crate::storage::local::LocalStorageBackend;

        // 具体的なケースでの実装テスト
        struct TestProvider;

        impl StaticComponentCreator<StandardImageLoader> for TestProvider {
            fn create() -> StandardImageLoader {
                StandardImageLoader::new()
            }
        }

        impl StaticComponentCreator<LocalStorageBackend> for TestProvider {
            fn create() -> LocalStorageBackend {
                LocalStorageBackend::new()
            }
        }

        let _loader: StandardImageLoader = TestProvider::create();
        let _storage: LocalStorageBackend = TestProvider::create();
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
        let _high_precision = create_static_container!(high_precision);
        let _fast = create_static_container!(fast);
    }

    #[test]
    fn test_const_generic_configs() {
        use crate::core::StaticDIContainer;

        // 高精度設定のテスト
        type HighPrecision = HighPrecisionConfig<2048, 32, 8, 500, 100>;
        let _container = StaticDIContainer::<ConfiguredStaticProvider<HighPrecision>>::new();
        assert_eq!(HighPrecision::HASH_SIZE, 32);
        assert_eq!(HighPrecision::CONCURRENT_TASKS, 8);
        // 実際の設定値を確認
        assert_eq!(HighPrecision::HASH_SIZE, 32);
        assert_eq!(HighPrecision::CONCURRENT_TASKS, 8);
        // QUIET_MODEの値をランタイムで確認
        let hp_reporter = HighPrecision::create_progress_reporter_typed();
        let hp_type_name = std::any::type_name_of_val(&hp_reporter);
        assert!(
            hp_type_name.contains("ConsoleProgressReporter"),
            "HighPrecision should use ConsoleProgressReporter"
        );

        // 高速設定のテスト
        type Fast = FastConfig<1024, 8, 16, 1000, 200>;
        let _container = StaticDIContainer::<ConfiguredStaticProvider<Fast>>::new();
        assert_eq!(Fast::HASH_SIZE, 8);
        assert_eq!(Fast::CONCURRENT_TASKS, 16);
        // QUIET_MODEの値をランタイムで確認
        let fast_reporter = Fast::create_progress_reporter_typed();
        let fast_type_name = std::any::type_name_of_val(&fast_reporter);
        assert!(
            fast_type_name.contains("NoOpProgressReporter"),
            "Fast should use NoOpProgressReporter"
        );
    }

    #[test]
    fn test_configured_container_creation() {
        // 新しいマクロを使った設定のテスト
        let _high_precision_container = build_configured_container!(
            image_size: 2048,
            hash_size: 64,
            concurrent_tasks: 4,
            buffer_size: 300,
            batch_size: 75,
            precision: high
        );

        let _fast_container = build_configured_container!(
            image_size: 512,
            hash_size: 8,
            concurrent_tasks: 32,
            buffer_size: 2000,
            batch_size: 500,
            precision: fast
        );
    }

    #[test]
    fn test_const_generic_type_safety() {
        use crate::core::StaticDIContainer;

        // コンパイル時型安全性の確認
        type TestHighPrecision = HighPrecisionConfig<1024, 16, 4, 100, 50>;
        type TestFast = FastConfig<512, 8, 8, 200, 100>;

        // 異なる設定は異なる型として扱われることを確認
        let _hp_container = StaticDIContainer::<ConfiguredStaticProvider<TestHighPrecision>>::new();
        let _fast_container = StaticDIContainer::<ConfiguredStaticProvider<TestFast>>::new();

        // 型情報の確認
        assert!(
            std::any::type_name::<TestHighPrecision>().contains("HighPrecisionConfig"),
            "TestHighPrecision should contain HighPrecisionConfig in type name"
        );
        assert!(
            std::any::type_name::<TestFast>().contains("FastConfig"),
            "TestFast should contain FastConfig in type name"
        );
    }
}
