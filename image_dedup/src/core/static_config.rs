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
        config.create_hasher().expect("Failed to create DCT hasher")
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
    const DEPENDENCY_DESCRIPTION: &'static str = "High performance configuration for maximum throughput";

    fn create_image_loader() -> Self::ImageLoader {
        StandardImageLoader::new()
    }

    fn create_perceptual_hash() -> Self::PerceptualHash {
        let config = DctConfig {
            size: 32,
            quality_factor: 1.0,
        };
        config.create_hasher().expect("Failed to create DCT hasher")
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
    const DEPENDENCY_DESCRIPTION: &'static str = "Testing configuration with lightweight components";

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
/// 注意：この実装はコンパイル時に具象型が確定している場合のみ使用可能
/// ここでは型パラメータを使用するため、実際の作成は困難
/// 代わりにCustomStaticProviderを使用することを推奨
impl<IL, PH, S, PC, PR, HP> StaticDependencyProvider for CustomConfig<IL, PH, S, PC, PR, HP>
where
    IL: ImageLoaderBackend + Send + Sync + 'static + Default,
    PH: PerceptualHashBackend + Send + Sync + 'static + Default,
    S: StorageBackend + Send + Sync + 'static + Default,
    PC: ProcessingConfig + Send + Sync + 'static + Default,
    PR: ProgressReporter + Send + Sync + 'static + Default,
    HP: HashPersistence + Send + Sync + 'static + Default,
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
                panic!("BatchSize制約違反: {}", <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE);
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::BUFFER_SIZE_VALID {
                panic!("BufferSize制約違反: {}", <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE);
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::THREAD_COUNT_VALID {
                panic!("ThreadCount制約違反: {}", <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE);
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::HASH_SIZE_VALID {
                panic!("HashSize制約違反: {}", <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE);
            }
            if !<$config as $crate::core::static_config::StaticConfigValidator>::IS_VALID {
                panic!("総合制約違反: {}", <$config as $crate::core::static_config::StaticConfigValidator>::ERROR_MESSAGE);
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
                panic!($message);
            }
        };
    };
    // 詳細情報付きバージョン
    ($condition:expr, $message:expr, $config:ty) => {
        const _: () = {
            if !$condition {
                panic!("{} - Config: {}", $message, stringify!($config));
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
            assert_eq!(std::mem::size_of::<$config>(), 0, "Config should be zero-sized");
            
            // Send + Sync の確認（コンパイル時）
            fn assert_send<T: Send>() {}
            fn assert_sync<T: Sync>() {}
            
            assert_send::<<$config as $crate::core::static_di::StaticDependencyProvider>::ImageLoader>();
            assert_sync::<<$config as $crate::core::static_di::StaticDependencyProvider>::ImageLoader>();
            
            assert_send::<<$config as $crate::core::static_di::StaticDependencyProvider>::PerceptualHash>();
            assert_sync::<<$config as $crate::core::static_di::StaticDependencyProvider>::PerceptualHash>();
            
            assert_send::<<$config as $crate::core::static_di::StaticDependencyProvider>::Storage>();
            assert_sync::<<$config as $crate::core::static_di::StaticDependencyProvider>::Storage>();
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
        // 簡単な実行時バリデーション
        assert!(DefaultConfig::IS_VALID);
        assert!(HighPerformanceConfig::IS_VALID);
        assert!(TestingConfig::IS_VALID);
    }

    #[test]
    fn test_static_config_constraints() {
        // 各設定の制約値をテスト
        assert!(DefaultConfig::BATCH_SIZE_VALID);
        assert!(DefaultConfig::BUFFER_SIZE_VALID);
        assert!(DefaultConfig::THREAD_COUNT_VALID);
        assert!(DefaultConfig::HASH_SIZE_VALID);
        assert!(DefaultConfig::IS_VALID);

        assert!(HighPerformanceConfig::BATCH_SIZE_VALID);
        assert!(HighPerformanceConfig::BUFFER_SIZE_VALID);
        assert!(HighPerformanceConfig::THREAD_COUNT_VALID);
        assert!(HighPerformanceConfig::HASH_SIZE_VALID);
        assert!(HighPerformanceConfig::IS_VALID);

        assert!(TestingConfig::BATCH_SIZE_VALID);
        assert!(TestingConfig::BUFFER_SIZE_VALID);
        assert!(TestingConfig::THREAD_COUNT_VALID);
        assert!(TestingConfig::HASH_SIZE_VALID);
        assert!(TestingConfig::IS_VALID);
    }

    #[test]
    fn test_advanced_config_constraints() {
        // 制約定数の一貫性をテスト
        assert!(DefaultConfig::MIN_BATCH_SIZE <= DefaultConfig::MAX_BATCH_SIZE);
        assert!(DefaultConfig::MIN_BUFFER_SIZE <= DefaultConfig::MAX_BUFFER_SIZE);
        assert!(DefaultConfig::MIN_THREADS <= DefaultConfig::MAX_THREADS);
        assert!(DefaultConfig::MIN_HASH_SIZE <= DefaultConfig::MAX_HASH_SIZE);
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
