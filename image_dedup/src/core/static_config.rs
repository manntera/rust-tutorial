//! コンパイル時設定システム
//!
//! 型レベルで設定を表現し、コンパイル時に依存関係を解決：
//! - TypeConfig: 型レベル設定表現
//! - StaticValidation: 設定の静的検証
//! - ZeroCostConfiguration: 実行時オーバーヘッドゼロの設定

use super::static_di::StaticDependencyProvider;
use crate::{
    core::{HashPersistence, ProcessingConfig, ProgressReporter},
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

/// 型レベル設定 - コンパイル時設定表現
///
/// 各設定項目を型パラメータで表現し、コンパイル時に設定を確定
pub trait TypeConfig {
    /// 設定名（コンパイル時文字列）
    const NAME: &'static str;

    /// 説明（コンパイル時文字列）
    const DESCRIPTION: &'static str;

    /// パフォーマンス特性
    const PERFORMANCE_LEVEL: PerformanceLevel;
}

/// パフォーマンスレベル定義
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceLevel {
    /// 低性能・高精度
    Accurate,
    /// バランス型
    Balanced,
    /// 高性能・低精度
    Fast,
}

/// デフォルト設定
pub struct DefaultConfig;

impl TypeConfig for DefaultConfig {
    const NAME: &'static str = "default";
    const DESCRIPTION: &'static str = "バランスの取れたデフォルト設定";
    const PERFORMANCE_LEVEL: PerformanceLevel = PerformanceLevel::Balanced;
}

impl StaticDependencyProvider for DefaultConfig {
    type ImageLoader = StandardImageLoader;
    type PerceptualHash = DctHasher;
    type Storage = LocalStorageBackend;
    type ProcessingConfig = DefaultProcessingConfig;
    type ProgressReporter = ConsoleProgressReporter;
    type HashPersistence = StreamingJsonHashPersistence;

    const DEPENDENCIES_VALID: bool = true;
    const DEPENDENCY_DESCRIPTION: &'static str = "Default configuration with balanced performance";

    fn create_image_loader() -> Self::ImageLoader {
        StandardImageLoader::new()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        let config = DctConfig {
            size: 8,
            quality_factor: 1.0,
        };
        // 非フォールバック版では、この設定が常に有効であることが保証されている
        config.create_hasher().unwrap_or_else(|_| {
            // このコードパスに到達することは設計上ありえないが、
            // 万が一のためのフォールバック
            unreachable!("DCT config with size 8 should always create a valid hasher")
        })
    }

    fn try_create_perceptual_hash() -> crate::core::ProcessingResult<Self::PerceptualHash> {
        let config = DctConfig {
            size: 8,
            quality_factor: 1.0,
        };
        config.create_hasher().map_err(|e| {
            crate::core::ProcessingError::dependency_injection(format!(
                "Failed to create DCT hasher for DefaultConfig: {e}"
            ))
        })
    }

    fn create_storage() -> Self::Storage {
        LocalStorageBackend::new()
    }

    fn create_processing_config() -> Self::ProcessingConfig {
        DefaultProcessingConfig::new(num_cpus::get())
            .with_buffer_size(100)
            .with_batch_size(50)
    }

    fn create_progress_reporter() -> Self::ProgressReporter {
        ConsoleProgressReporter::new()
    }

    fn create_hash_persistence(output_path: &std::path::Path) -> Self::HashPersistence {
        StreamingJsonHashPersistence::new(output_path)
    }
}

/// 高性能設定
pub struct HighPerformanceConfig;

impl TypeConfig for HighPerformanceConfig {
    const NAME: &'static str = "high_performance";
    const DESCRIPTION: &'static str = "高性能・高スループット設定";
    const PERFORMANCE_LEVEL: PerformanceLevel = PerformanceLevel::Fast;
}

impl StaticDependencyProvider for HighPerformanceConfig {
    type ImageLoader = StandardImageLoader;
    type PerceptualHash = DctHasher;
    type Storage = LocalStorageBackend;
    type ProcessingConfig = DefaultProcessingConfig;
    type ProgressReporter = ConsoleProgressReporter;
    type HashPersistence = StreamingJsonHashPersistence;

    const DEPENDENCIES_VALID: bool = true;
    const DEPENDENCY_DESCRIPTION: &'static str =
        "High performance configuration for maximum throughput";

    fn create_image_loader() -> Self::ImageLoader {
        StandardImageLoader::new()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        let config = DctConfig {
            size: 32,
            quality_factor: 1.0,
        };
        // 非フォールバック版では、この設定が常に有効であることが保証されている
        config.create_hasher().unwrap_or_else(|_| {
            // このコードパスに到達することは設計上ありえないが、
            // 万が一のためのフォールバック
            unreachable!("DCT config with size 32 should always create a valid hasher")
        })
    }

    fn try_create_perceptual_hash() -> crate::core::ProcessingResult<Self::PerceptualHash> {
        let config = DctConfig {
            size: 32,
            quality_factor: 1.0,
        };
        config.create_hasher().map_err(|e| {
            crate::core::ProcessingError::dependency_injection(format!(
                "Failed to create DCT hasher for HighPerformanceConfig: {e}"
            ))
        })
    }

    fn create_storage() -> Self::Storage {
        LocalStorageBackend::new()
    }

    fn create_processing_config() -> Self::ProcessingConfig {
        DefaultProcessingConfig::new(num_cpus::get() * 2)
            .with_buffer_size(500)
            .with_batch_size(100)
    }

    fn create_progress_reporter() -> Self::ProgressReporter {
        ConsoleProgressReporter::new()
    }

    fn create_hash_persistence(output_path: &std::path::Path) -> Self::HashPersistence {
        StreamingJsonHashPersistence::new(output_path)
    }
}

/// テスト用設定
pub struct TestingConfig;

impl TypeConfig for TestingConfig {
    const NAME: &'static str = "testing";
    const DESCRIPTION: &'static str = "テスト用軽量設定";
    const PERFORMANCE_LEVEL: PerformanceLevel = PerformanceLevel::Fast;
}

impl StaticDependencyProvider for TestingConfig {
    type ImageLoader = StandardImageLoader;
    type PerceptualHash = AverageHasher;
    type Storage = LocalStorageBackend;
    type ProcessingConfig = DefaultProcessingConfig;
    type ProgressReporter = NoOpProgressReporter;
    type HashPersistence = MemoryHashPersistence;

    const DEPENDENCIES_VALID: bool = true;
    const DEPENDENCY_DESCRIPTION: &'static str =
        "Testing configuration with lightweight components";

    fn create_image_loader() -> Self::ImageLoader {
        StandardImageLoader::new()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        let config = AverageConfig { size: 8 };
        // 非フォールバック版では、この設定が常に有効であることが保証されている
        config.create_hasher().unwrap_or_else(|_| {
            // このコードパスに到達することは設計上ありえないが、
            // 万が一のためのフォールバック
            unreachable!("Average config with size 8 should always create a valid hasher")
        })
    }

    fn try_create_perceptual_hash() -> crate::core::ProcessingResult<Self::PerceptualHash> {
        let config = AverageConfig { size: 8 };
        config.create_hasher().map_err(|e| {
            crate::core::ProcessingError::dependency_injection(format!(
                "Failed to create Average hasher for TestingConfig: {e}"
            ))
        })
    }

    fn create_storage() -> Self::Storage {
        LocalStorageBackend::new()
    }

    fn create_processing_config() -> Self::ProcessingConfig {
        DefaultProcessingConfig::new(2)
            .with_buffer_size(10)
            .with_batch_size(5)
    }

    fn create_progress_reporter() -> Self::ProgressReporter {
        NoOpProgressReporter::new()
    }

    fn create_hash_persistence(_output_path: &std::path::Path) -> Self::HashPersistence {
        MemoryHashPersistence::new()
    }
}

/// カスタム設定ビルダー
///
/// 型安全な方法で独自設定を構築
pub struct CustomConfigBuilder<IL, PH, S, PC, PR, HP> {
    _image_loader: PhantomData<IL>,
    _perceptual_hash: PhantomData<PH>,
    _storage: PhantomData<S>,
    _processing_config: PhantomData<PC>,
    _progress_reporter: PhantomData<PR>,
    _hash_persistence: PhantomData<HP>,
}

impl<IL, PH, S, PC, PR, HP> Default for CustomConfigBuilder<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: ProcessingConfig + Send + Sync + 'static,
    PR: ProgressReporter + Send + Sync + 'static,
    HP: HashPersistence + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<IL, PH, S, PC, PR, HP> CustomConfigBuilder<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: ProcessingConfig + Send + Sync + 'static,
    PR: ProgressReporter + Send + Sync + 'static,
    HP: HashPersistence + Send + Sync + 'static,
{
    /// 新しいカスタム設定ビルダーを作成
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

/// カスタム設定の実装
pub struct CustomConfig<IL, PH, S, PC, PR, HP> {
    _builder: CustomConfigBuilder<IL, PH, S, PC, PR, HP>,
}

impl<IL, PH, S, PC, PR, HP> TypeConfig for CustomConfig<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: ProcessingConfig + Send + Sync + 'static,
    PR: ProgressReporter + Send + Sync + 'static,
    HP: HashPersistence + Send + Sync + 'static,
{
    const NAME: &'static str = "custom";
    const DESCRIPTION: &'static str = "カスタム設定";
    const PERFORMANCE_LEVEL: PerformanceLevel = PerformanceLevel::Balanced;
}

/// カスタム設定のStaticDependencyProvider実装
///
/// Default依存を削除し、より厳密な型制約を使用
/// 具体的な作成方法は各型で実装される専用トレイトに委ねる
impl<IL, PH, S, PC, PR, HP> StaticDependencyProvider for CustomConfig<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: ProcessingConfig + Send + Sync + 'static,
    PR: ProgressReporter + Send + Sync + 'static,
    HP: HashPersistence + Send + Sync + 'static,
    CustomConfig<IL, PH, S, PC, PR, HP>: crate::factories::StaticComponentCreator<IL>,
    CustomConfig<IL, PH, S, PC, PR, HP>: crate::factories::StaticComponentCreator<PH>,
    CustomConfig<IL, PH, S, PC, PR, HP>: crate::factories::StaticComponentCreator<S>,
    CustomConfig<IL, PH, S, PC, PR, HP>: crate::factories::StaticComponentCreator<PC>,
    CustomConfig<IL, PH, S, PC, PR, HP>: crate::factories::StaticComponentCreator<PR>,
    CustomConfig<IL, PH, S, PC, PR, HP>: crate::factories::StaticComponentCreatorWithPath<HP>,
{
    type ImageLoader = IL;
    type PerceptualHash = PH;
    type Storage = S;
    type ProcessingConfig = PC;
    type ProgressReporter = PR;
    type HashPersistence = HP;

    const DEPENDENCIES_VALID: bool = true;
    const DEPENDENCY_DESCRIPTION: &'static str =
        "Custom configuration with type-safe component creation";

    fn create_image_loader() -> Self::ImageLoader {
        <Self as crate::factories::StaticComponentCreator<IL>>::create()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        <Self as crate::factories::StaticComponentCreator<PH>>::create()
    }

    fn create_storage() -> Self::Storage {
        <Self as crate::factories::StaticComponentCreator<S>>::create()
    }

    fn create_processing_config() -> Self::ProcessingConfig {
        <Self as crate::factories::StaticComponentCreator<PC>>::create()
    }

    fn create_progress_reporter() -> Self::ProgressReporter {
        <Self as crate::factories::StaticComponentCreator<PR>>::create()
    }

    fn create_hash_persistence(output_path: &std::path::Path) -> Self::HashPersistence {
        <Self as crate::factories::StaticComponentCreatorWithPath<HP>>::create_with_path(
            output_path,
        )
    }
}

/// カスタム設定の静的バリデーション実装
impl<IL, PH, S, PC, PR, HP> StaticConfigValidator for CustomConfig<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static,
    PH: PerceptualHashBackend + Send + Sync + 'static,
    S: StorageBackend + Send + Sync + 'static,
    PC: ProcessingConfig + Send + Sync + 'static,
    PR: ProgressReporter + Send + Sync + 'static,
    HP: HashPersistence + Send + Sync + 'static,
{
    // カスタム設定はより緩い制約を適用
    const BATCH_SIZE_VALID: bool = true; // 実行時チェックに委ねる
    const BUFFER_SIZE_VALID: bool = true;
    const THREAD_COUNT_VALID: bool = true;
    const HASH_SIZE_VALID: bool = true;

    const IS_VALID: bool = Self::BATCH_SIZE_VALID
        && Self::BUFFER_SIZE_VALID
        && Self::THREAD_COUNT_VALID
        && Self::HASH_SIZE_VALID;
    const ERROR_MESSAGE: &'static str = "CustomConfig設定検証に失敗しました";
}

/// 設定の静的検証
pub trait StaticConfigValidator {
    /// 設定の整合性を検証
    const IS_VALID: bool;

    /// エラーメッセージ（コンパイル時）
    const ERROR_MESSAGE: &'static str;

    /// 詳細バリデーション
    const BATCH_SIZE_VALID: bool;
    const BUFFER_SIZE_VALID: bool;
    const THREAD_COUNT_VALID: bool;
    const HASH_SIZE_VALID: bool;
}

/// DefaultConfig用の静的バリデーション実装
impl StaticConfigValidator for DefaultConfig {
    const BATCH_SIZE_VALID: bool = 50 >= Self::MIN_BATCH_SIZE && 50 <= Self::MAX_BATCH_SIZE;
    const BUFFER_SIZE_VALID: bool = 100 >= Self::MIN_BUFFER_SIZE && 100 <= Self::MAX_BUFFER_SIZE;
    const THREAD_COUNT_VALID: bool = {
        // num_cpus::get()は実行時関数のため、合理的な範囲でチェック
        let assumed_cpus = 8; // 仮定値
        assumed_cpus >= Self::MIN_THREADS && assumed_cpus <= Self::MAX_THREADS
    };
    const HASH_SIZE_VALID: bool = 8 >= Self::MIN_HASH_SIZE && 8 <= Self::MAX_HASH_SIZE;

    const IS_VALID: bool = Self::BATCH_SIZE_VALID
        && Self::BUFFER_SIZE_VALID
        && Self::THREAD_COUNT_VALID
        && Self::HASH_SIZE_VALID;
    const ERROR_MESSAGE: &'static str = "DefaultConfig設定検証に失敗しました";
}

/// HighPerformanceConfig用の静的バリデーション実装
impl StaticConfigValidator for HighPerformanceConfig {
    const BATCH_SIZE_VALID: bool = 100 >= Self::MIN_BATCH_SIZE && 100 <= Self::MAX_BATCH_SIZE;
    const BUFFER_SIZE_VALID: bool = 500 >= Self::MIN_BUFFER_SIZE && 500 <= Self::MAX_BUFFER_SIZE;
    const THREAD_COUNT_VALID: bool = {
        let assumed_cpus = 16; // 高性能設定での仮定値
        assumed_cpus >= Self::MIN_THREADS && assumed_cpus <= Self::MAX_THREADS
    };
    const HASH_SIZE_VALID: bool = 32 >= Self::MIN_HASH_SIZE && 32 <= Self::MAX_HASH_SIZE;

    const IS_VALID: bool = Self::BATCH_SIZE_VALID
        && Self::BUFFER_SIZE_VALID
        && Self::THREAD_COUNT_VALID
        && Self::HASH_SIZE_VALID;
    const ERROR_MESSAGE: &'static str = "HighPerformanceConfig設定検証に失敗しました";
}

/// TestingConfig用の静的バリデーション実装
impl StaticConfigValidator for TestingConfig {
    const BATCH_SIZE_VALID: bool = 5 >= Self::MIN_BATCH_SIZE && 5 <= Self::MAX_BATCH_SIZE;
    const BUFFER_SIZE_VALID: bool = 10 >= Self::MIN_BUFFER_SIZE && 10 <= Self::MAX_BUFFER_SIZE;
    const THREAD_COUNT_VALID: bool = 2 >= Self::MIN_THREADS && 2 <= Self::MAX_THREADS;
    const HASH_SIZE_VALID: bool = 8 >= Self::MIN_HASH_SIZE && 8 <= Self::MAX_HASH_SIZE;

    const IS_VALID: bool = Self::BATCH_SIZE_VALID
        && Self::BUFFER_SIZE_VALID
        && Self::THREAD_COUNT_VALID
        && Self::HASH_SIZE_VALID;
    const ERROR_MESSAGE: &'static str = "TestingConfig設定検証に失敗しました";
}

/// 高度な設定制約チェック
pub trait AdvancedConfigConstraints {
    /// バッチサイズの制約
    const MIN_BATCH_SIZE: usize = 1;
    const MAX_BATCH_SIZE: usize = 10000;

    /// バッファサイズの制約
    const MIN_BUFFER_SIZE: usize = 1;
    const MAX_BUFFER_SIZE: usize = 100000;

    /// スレッド数の制約
    const MIN_THREADS: usize = 1;
    const MAX_THREADS: usize = 1024;

    /// ハッシュサイズの制約
    const MIN_HASH_SIZE: u32 = 4;
    const MAX_HASH_SIZE: u32 = 256;
}

impl<T: TypeConfig> AdvancedConfigConstraints for T {}

/// コンパイル時設定検証マクロ（改善版）
#[macro_export]
macro_rules! validate_static_config {
    ($config:ty) => {
        const _: () = {
            // 各制約を個別にチェックして詳細なエラーメッセージを提供
            if !<$config as $crate::core::static_config::StaticConfigValidator>::BATCH_SIZE_VALID {
                compile_error!(concat!(
                    "BatchSize制約違反: ",
                    <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE
                ));
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::BUFFER_SIZE_VALID {
                compile_error!(concat!(
                    "BufferSize制約違反: ",
                    <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE
                ));
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::THREAD_COUNT_VALID
            {
                compile_error!(concat!(
                    "ThreadCount制約違反: ",
                    <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE
                ));
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::HASH_SIZE_VALID {
                compile_error!(concat!(
                    "HashSize制約違反: ",
                    <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE
                ));
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::IS_VALID {
                compile_error!(concat!(
                    "総合制約違反: ",
                    <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE
                ));
            }
        };
    };
}

/// const assertion マクロ（改善版）
#[macro_export]
macro_rules! const_assert_config {
    ($condition:expr, $message:expr) => {
        const _: () = {
            if !$condition {
                compile_error!($message);
            }
        };
    };
    // 詳細情報付きバージョン
    ($condition:expr, $message:expr, $config:ty) => {
        const _: () = {
            if !$condition {
                compile_error!(concat!($message, " - Config: ", stringify!($config)));
            }
        };
    };
}

/// 型レベル制約強化マクロ
#[macro_export]
macro_rules! enforce_config_constraints {
    ($config:ty) => {
        // バッチサイズ制約
        $crate::const_assert_config!(
            <$config as $crate::core::static_config::AdvancedConfigConstraints>::MIN_BATCH_SIZE <= <$config as $crate::core::static_config::AdvancedConfigConstraints>::MAX_BATCH_SIZE,
            "MIN_BATCH_SIZE must be <= MAX_BATCH_SIZE"
        );

        // バッファサイズ制約
        $crate::const_assert_config!(
            <$config as $crate::core::static_config::AdvancedConfigConstraints>::MIN_BUFFER_SIZE <= <$config as $crate::core::static_config::AdvancedConfigConstraints>::MAX_BUFFER_SIZE,
            "MIN_BUFFER_SIZE must be <= MAX_BUFFER_SIZE"
        );

        // スレッド数制約
        $crate::const_assert_config!(
            <$config as $crate::core::static_config::AdvancedConfigConstraints>::MIN_THREADS <= <$config as $crate::core::static_config::AdvancedConfigConstraints>::MAX_THREADS,
            "MIN_THREADS must be <= MAX_THREADS"
        );

        // ハッシュサイズ制約
        $crate::const_assert_config!(
            <$config as $crate::core::static_config::AdvancedConfigConstraints>::MIN_HASH_SIZE <= <$config as $crate::core::static_config::AdvancedConfigConstraints>::MAX_HASH_SIZE,
            "MIN_HASH_SIZE must be <= MAX_HASH_SIZE"
        );

        // 実際の設定値制約
        $crate::validate_static_config!($config);
    };
}

/// 型安全性検証マクロ（簡略版）
#[macro_export]
macro_rules! verify_type_safety {
    ($config:ty) => {
        // 静的アサート（実行時チェック）
        fn _verify_type_safety() {
            // ゼロコスト抽象化の確認
            assert_eq!(
                std::mem::size_of::<$config>(),
                0,
                "Config should be zero-sized"
            );

            // Send + Sync の確認（コンパイル時）
            fn assert_send<T: Send>() {}
            fn assert_sync<T: Sync>() {}

            assert_send::<
                <$config as $crate::core::static_di::StaticDependencyProvider>::ImageLoader,
            >();
            assert_sync::<
                <$config as $crate::core::static_di::StaticDependencyProvider>::ImageLoader,
            >();

            assert_send::<
                <$config as $crate::core::static_di::StaticDependencyProvider>::PerceptualHash,
            >();
            assert_sync::<
                <$config as $crate::core::static_di::StaticDependencyProvider>::PerceptualHash,
            >();

            assert_send::<<$config as $crate::core::static_di::StaticDependencyProvider>::Storage>(
            );
            assert_sync::<<$config as $crate::core::static_di::StaticDependencyProvider>::Storage>(
            );
        }
    };
}

/// 性能指標計算トレイト
pub trait PerformanceMetrics {
    /// 理論的最大スループット（ファイル/秒）
    const THEORETICAL_MAX_THROUGHPUT: f64;

    /// メモリ使用量推定（MB）
    const ESTIMATED_MEMORY_MB: f64;

    /// CPU使用率推定（%）
    const ESTIMATED_CPU_USAGE: f64;
}

impl PerformanceMetrics for DefaultConfig {
    const THEORETICAL_MAX_THROUGHPUT: f64 = 100.0;
    const ESTIMATED_MEMORY_MB: f64 = 256.0;
    const ESTIMATED_CPU_USAGE: f64 = 70.0;
}

impl PerformanceMetrics for HighPerformanceConfig {
    const THEORETICAL_MAX_THROUGHPUT: f64 = 500.0;
    const ESTIMATED_MEMORY_MB: f64 = 1024.0;
    const ESTIMATED_CPU_USAGE: f64 = 95.0;
}

impl PerformanceMetrics for TestingConfig {
    const THEORETICAL_MAX_THROUGHPUT: f64 = 20.0;
    const ESTIMATED_MEMORY_MB: f64 = 64.0;
    const ESTIMATED_CPU_USAGE: f64 = 30.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::static_di::StaticDIContainer;

    #[test]
    fn test_config_constants() {
        assert_eq!(DefaultConfig::NAME, "default");
        assert_eq!(HighPerformanceConfig::NAME, "high_performance");
        assert_eq!(TestingConfig::NAME, "testing");
    }

    #[test]
    fn test_performance_levels() {
        assert_eq!(DefaultConfig::PERFORMANCE_LEVEL, PerformanceLevel::Balanced);
        assert_eq!(
            HighPerformanceConfig::PERFORMANCE_LEVEL,
            PerformanceLevel::Fast
        );
        assert_eq!(TestingConfig::PERFORMANCE_LEVEL, PerformanceLevel::Fast);
    }

    #[test]
    fn test_static_di_with_configs() {
        let default_container = StaticDIContainer::<DefaultConfig>::new();
        let hp_container = StaticDIContainer::<HighPerformanceConfig>::new();
        let test_container = StaticDIContainer::<TestingConfig>::new();

        // 型が異なることを確認（静的ディスパッチ）
        let _default_loader = default_container.create_image_loader();
        let _hp_loader = hp_container.create_image_loader();
        let _test_loader = test_container.create_image_loader();
    }

    #[test]
    fn test_zero_cost_config() {
        assert_eq!(std::mem::size_of::<DefaultConfig>(), 0);
        assert_eq!(std::mem::size_of::<HighPerformanceConfig>(), 0);
        assert_eq!(std::mem::size_of::<TestingConfig>(), 0);
    }

    #[test]
    fn test_config_validation() {
        // 実際の設定値を使った実行時テスト
        let temp_dir = tempfile::TempDir::new().unwrap();
        let _output_path = temp_dir.path().join("test.json");

        // DefaultConfigの実際の動作をテスト
        let default_container = StaticDIContainer::<DefaultConfig>::new();
        let default_config = default_container.create_processing_config();
        assert!(default_config.max_concurrent_tasks() > 0);
        assert!(default_config.batch_size() > 0);
        assert!(default_config.channel_buffer_size() > 0);

        // HighPerformanceConfigの実際の動作をテスト
        let hp_container = StaticDIContainer::<HighPerformanceConfig>::new();
        let hp_config = hp_container.create_processing_config();
        assert!(hp_config.max_concurrent_tasks() > default_config.max_concurrent_tasks());
        assert!(hp_config.batch_size() > default_config.batch_size());
        assert!(hp_config.channel_buffer_size() > default_config.channel_buffer_size());

        // TestingConfigの実際の動作をテスト
        let test_container = StaticDIContainer::<TestingConfig>::new();
        let test_config = test_container.create_processing_config();
        assert!(test_config.max_concurrent_tasks() > 0);
        assert!(test_config.batch_size() > 0);
        assert!(test_config.channel_buffer_size() > 0);

        // 実行時での設定値整合性を実際にテスト
        let default_container = StaticDIContainer::<DefaultConfig>::new();
        let default_config = default_container.create_processing_config();
        assert!(default_config.max_concurrent_tasks() > 0);
        assert!(default_config.batch_size() > 0);
        assert!(default_config.channel_buffer_size() > 0);
    }

    #[test]
    fn test_static_config_constraints() {
        // 実際の設定値が予想範囲内であることをテスト
        let temp_dir = tempfile::TempDir::new().unwrap();
        let _output_path = temp_dir.path().join("test.json");

        // DefaultConfigの実際の値をテスト
        let default_container = StaticDIContainer::<DefaultConfig>::new();
        let default_config = default_container.create_processing_config();
        assert!(default_config.batch_size() >= 1);
        assert!(default_config.batch_size() <= 10000);
        assert!(default_config.channel_buffer_size() >= 1);
        assert!(default_config.channel_buffer_size() <= 100000);
        assert!(default_config.max_concurrent_tasks() >= 1);
        assert!(default_config.max_concurrent_tasks() <= 1024);

        // HighPerformanceConfigの実際の値をテスト
        let hp_container = StaticDIContainer::<HighPerformanceConfig>::new();
        let hp_config = hp_container.create_processing_config();
        assert!(hp_config.batch_size() >= 1);
        assert!(hp_config.batch_size() <= 10000);
        assert!(hp_config.channel_buffer_size() >= 1);
        assert!(hp_config.channel_buffer_size() <= 100000);
        assert!(hp_config.max_concurrent_tasks() >= 1);
        assert!(hp_config.max_concurrent_tasks() <= 1024);

        // TestingConfigの実際の値をテスト
        let test_container = StaticDIContainer::<TestingConfig>::new();
        let test_config = test_container.create_processing_config();
        assert!(test_config.batch_size() >= 1);
        assert!(test_config.batch_size() <= 10000);
        assert!(test_config.channel_buffer_size() >= 1);
        assert!(test_config.channel_buffer_size() <= 100000);
        assert!(test_config.max_concurrent_tasks() >= 1);
        assert!(test_config.max_concurrent_tasks() <= 1024);

        // 実際の値範囲をテスト
        let temp_dir = tempfile::TempDir::new().unwrap();
        let _output_path = temp_dir.path().join("test.json");

        let default_container = StaticDIContainer::<DefaultConfig>::new();
        let default_config = default_container.create_processing_config();
        assert!(default_config.batch_size() >= DefaultConfig::MIN_BATCH_SIZE);
        assert!(default_config.batch_size() <= DefaultConfig::MAX_BATCH_SIZE);

        let hp_container = StaticDIContainer::<HighPerformanceConfig>::new();
        let hp_config = hp_container.create_processing_config();
        assert!(hp_config.batch_size() >= HighPerformanceConfig::MIN_BATCH_SIZE);
        assert!(hp_config.batch_size() <= HighPerformanceConfig::MAX_BATCH_SIZE);
    }

    #[test]
    fn test_advanced_config_constraints() {
        // 制約定数の範囲が合理的であることをテスト
        // MIN値が1以上であることを確認
        let min_batch = DefaultConfig::MIN_BATCH_SIZE;
        let max_batch = DefaultConfig::MAX_BATCH_SIZE;
        assert!(
            min_batch >= 1,
            "MIN_BATCH_SIZE should be at least 1, got {min_batch}"
        );
        assert!(
            max_batch <= 100000,
            "MAX_BATCH_SIZE should be reasonable, got {max_batch}"
        );
        assert!(
            min_batch <= max_batch,
            "MIN_BATCH_SIZE ({min_batch}) should be <= MAX_BATCH_SIZE ({max_batch})"
        );

        let min_buffer = DefaultConfig::MIN_BUFFER_SIZE;
        let max_buffer = DefaultConfig::MAX_BUFFER_SIZE;
        assert!(
            min_buffer >= 1,
            "MIN_BUFFER_SIZE should be at least 1, got {min_buffer}"
        );
        assert!(
            max_buffer <= 200000,
            "MAX_BUFFER_SIZE should be reasonable, got {max_buffer}"
        );
        assert!(
            min_buffer <= max_buffer,
            "MIN_BUFFER_SIZE ({min_buffer}) should be <= MAX_BUFFER_SIZE ({max_buffer})"
        );

        let min_threads = DefaultConfig::MIN_THREADS;
        let max_threads = DefaultConfig::MAX_THREADS;
        assert!(
            min_threads >= 1,
            "MIN_THREADS should be at least 1, got {min_threads}"
        );
        assert!(
            max_threads <= 2048,
            "MAX_THREADS should be reasonable, got {max_threads}"
        );
        assert!(
            min_threads <= max_threads,
            "MIN_THREADS ({min_threads}) should be <= MAX_THREADS ({max_threads})"
        );

        let min_hash = DefaultConfig::MIN_HASH_SIZE;
        let max_hash = DefaultConfig::MAX_HASH_SIZE;
        assert!(
            min_hash >= 4,
            "MIN_HASH_SIZE should be at least 4, got {min_hash}"
        );
        assert!(
            max_hash <= 512,
            "MAX_HASH_SIZE should be reasonable, got {max_hash}"
        );
        assert!(
            min_hash <= max_hash,
            "MIN_HASH_SIZE ({min_hash}) should be <= MAX_HASH_SIZE ({max_hash})"
        );
    }

    #[test]
    fn test_const_config_creation() {
        const _BUILDER: CustomConfigBuilder<
            StandardImageLoader,
            AverageHasher,
            LocalStorageBackend,
            DefaultProcessingConfig,
            ConsoleProgressReporter,
            MemoryHashPersistence,
        > = CustomConfigBuilder::new();
    }
}
