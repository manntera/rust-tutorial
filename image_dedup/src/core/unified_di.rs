//! 統一DI API - 動的・静的ディスパッチの統一インターフェース
//!
//! Rustの哲学に基づく最適化されたDIシステム：
//! - Zero-cost abstraction: 使わない機能にはコストが発生しない
//! - Performance by default: デフォルトで最高性能を提供
//! - Type safety: コンパイル時の型安全性を保証
//! - Ergonomics: 使いやすい統一API

use super::{
    di_container::{BoxedProcessingEngine, DependencyContainer},
    static_config::{DefaultConfig, HighPerformanceConfig, TestingConfig},
    static_di::{StaticDIContainer, StaticProcessingEngine},
    ProcessingError, ProcessingResult,
};
use std::path::Path;

/// DIモードの選択
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DIMode {
    /// 静的ディスパッチ（最高性能、コンパイル時型確定）
    Static,
    /// 動的ディスパッチ（柔軟性、実行時設定変更）
    Dynamic,
}

/// 統一DI インターフェース
///
/// 静的・動的ディスパッチを統一的に扱う高レベルAPI
/// 使用者は実装の詳細を意識せずに最適なDIを選択可能
pub struct UnifiedDI {
    mode: DIMode,
}

impl UnifiedDI {
    /// 新しい統一DIを作成
    pub const fn new(mode: DIMode) -> Self {
        Self { mode }
    }

    /// デフォルト静的DI（最高性能）
    pub const fn static_default() -> Self {
        Self::new(DIMode::Static)
    }

    /// 動的DI（柔軟性重視）
    pub const fn dynamic() -> Self {
        Self::new(DIMode::Dynamic)
    }

    /// 現在のDIモードを取得
    pub const fn mode(&self) -> DIMode {
        self.mode
    }

    /// プリセット設定で処理エンジンを作成
    pub fn create_engine_with_preset(
        &self,
        preset: &str,
        output_path: &Path,
    ) -> ProcessingResult<ProcessingEngineVariant> {
        match self.mode {
            DIMode::Static => self.create_static_engine_with_preset(preset, output_path),
            DIMode::Dynamic => self.create_dynamic_engine_with_preset(preset, output_path),
        }
    }

    /// 静的エンジンをプリセットで作成
    fn create_static_engine_with_preset(
        &self,
        preset: &str,
        output_path: &Path,
    ) -> ProcessingResult<ProcessingEngineVariant> {
        match preset {
            "default" => {
                let container = StaticDIContainer::<DefaultConfig>::new();
                let engine = container.create_processing_engine(output_path);
                Ok(ProcessingEngineVariant::DefaultStatic(engine))
            }
            "high_performance" => {
                let container = StaticDIContainer::<HighPerformanceConfig>::new();
                let engine = container.create_processing_engine(output_path);
                Ok(ProcessingEngineVariant::HighPerformanceStatic(engine))
            }
            "testing" => {
                let container = StaticDIContainer::<TestingConfig>::new();
                let engine = container.create_processing_engine(output_path);
                Ok(ProcessingEngineVariant::TestingStatic(engine))
            }
            _ => Err(ProcessingError::configuration(format!(
                "無効な静的プリセット: {preset}. 利用可能: default, high_performance, testing"
            ))),
        }
    }

    /// 動的エンジンをプリセットで作成
    fn create_dynamic_engine_with_preset(
        &self,
        preset: &str,
        output_path: &Path,
    ) -> ProcessingResult<ProcessingEngineVariant> {
        let container = DependencyContainer::with_preset(preset)?;
        let bundle = container.resolve_all_dependencies(output_path)?;
        let engine = bundle.create_processing_engine();

        match preset {
            "default" => Ok(ProcessingEngineVariant::DefaultDynamic(engine)),
            "high_performance" => Ok(ProcessingEngineVariant::HighPerformanceDynamic(engine)),
            "testing" => Ok(ProcessingEngineVariant::TestingDynamic(engine)),
            _ => Err(ProcessingError::configuration(format!(
                "無効な動的プリセット: {preset}"
            ))),
        }
    }
}

impl Default for UnifiedDI {
    /// デフォルトは静的DI（最高性能）
    fn default() -> Self {
        Self::static_default()
    }
}

/// 処理エンジンの型安全バリアント
///
/// 静的・動的ディスパッチの両方をサポートしつつ、
/// 型安全性を保持する
pub enum ProcessingEngineVariant {
    // 静的ディスパッチ版
    DefaultStatic(StaticProcessingEngine<DefaultConfig>),
    HighPerformanceStatic(StaticProcessingEngine<HighPerformanceConfig>),
    TestingStatic(StaticProcessingEngine<TestingConfig>),

    // 動的ディスパッチ版
    DefaultDynamic(BoxedProcessingEngine),
    HighPerformanceDynamic(BoxedProcessingEngine),
    TestingDynamic(BoxedProcessingEngine),
}

impl ProcessingEngineVariant {
    /// ディレクトリを処理（統一API）
    pub async fn process_directory(
        &self,
        path: &str,
    ) -> ProcessingResult<crate::ProcessingSummary> {
        match self {
            // 静的ディスパッチ版（最高性能）
            ProcessingEngineVariant::DefaultStatic(engine) => engine.process_directory(path).await,
            ProcessingEngineVariant::HighPerformanceStatic(engine) => {
                engine.process_directory(path).await
            }
            ProcessingEngineVariant::TestingStatic(engine) => engine.process_directory(path).await,

            // 動的ディスパッチ版（柔軟性）
            ProcessingEngineVariant::DefaultDynamic(engine) => engine.process_directory(path).await,
            ProcessingEngineVariant::HighPerformanceDynamic(engine) => {
                engine.process_directory(path).await
            }
            ProcessingEngineVariant::TestingDynamic(engine) => engine.process_directory(path).await,
        }
    }

    /// ファイルリストを処理（統一API）
    pub async fn process_files(
        &self,
        files: Vec<String>,
    ) -> ProcessingResult<crate::ProcessingSummary> {
        match self {
            // 静的ディスパッチ版
            ProcessingEngineVariant::DefaultStatic(engine) => engine.process_files(files).await,
            ProcessingEngineVariant::HighPerformanceStatic(engine) => {
                engine.process_files(files).await
            }
            ProcessingEngineVariant::TestingStatic(engine) => engine.process_files(files).await,

            // 動的ディスパッチ版
            ProcessingEngineVariant::DefaultDynamic(engine) => engine.process_files(files).await,
            ProcessingEngineVariant::HighPerformanceDynamic(engine) => {
                engine.process_files(files).await
            }
            ProcessingEngineVariant::TestingDynamic(engine) => engine.process_files(files).await,
        }
    }

    /// エンジンの種類を取得
    pub fn engine_type(&self) -> &'static str {
        match self {
            ProcessingEngineVariant::DefaultStatic(_) => "StaticDefault",
            ProcessingEngineVariant::HighPerformanceStatic(_) => "StaticHighPerformance",
            ProcessingEngineVariant::TestingStatic(_) => "StaticTesting",
            ProcessingEngineVariant::DefaultDynamic(_) => "DynamicDefault",
            ProcessingEngineVariant::HighPerformanceDynamic(_) => "DynamicHighPerformance",
            ProcessingEngineVariant::TestingDynamic(_) => "DynamicTesting",
        }
    }

    /// パフォーマンス特性を取得
    pub fn performance_characteristics(&self) -> PerformanceCharacteristics {
        match self {
            ProcessingEngineVariant::DefaultStatic(_) => PerformanceCharacteristics::StaticBalanced,
            ProcessingEngineVariant::HighPerformanceStatic(_) => {
                PerformanceCharacteristics::StaticFast
            }
            ProcessingEngineVariant::TestingStatic(_) => PerformanceCharacteristics::StaticTesting,
            ProcessingEngineVariant::DefaultDynamic(_) => {
                PerformanceCharacteristics::DynamicBalanced
            }
            ProcessingEngineVariant::HighPerformanceDynamic(_) => {
                PerformanceCharacteristics::DynamicFast
            }
            ProcessingEngineVariant::TestingDynamic(_) => {
                PerformanceCharacteristics::DynamicTesting
            }
        }
    }
}

/// パフォーマンス特性の分類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceCharacteristics {
    /// 静的ディスパッチ・バランス型
    StaticBalanced,
    /// 静的ディスパッチ・高速型
    StaticFast,
    /// 静的ディスパッチ・テスト用
    StaticTesting,
    /// 動的ディスパッチ・バランス型
    DynamicBalanced,
    /// 動的ディスパッチ・高速型
    DynamicFast,
    /// 動的ディスパッチ・テスト用
    DynamicTesting,
}

impl PerformanceCharacteristics {
    /// ディスパッチタイプを取得
    pub const fn dispatch_type(&self) -> &'static str {
        match self {
            Self::StaticBalanced | Self::StaticFast | Self::StaticTesting => "Static",
            Self::DynamicBalanced | Self::DynamicFast | Self::DynamicTesting => "Dynamic",
        }
    }

    /// パフォーマンスレベルを取得
    pub const fn performance_level(&self) -> &'static str {
        match self {
            Self::StaticBalanced | Self::DynamicBalanced => "Balanced",
            Self::StaticFast | Self::DynamicFast => "Fast",
            Self::StaticTesting | Self::DynamicTesting => "Testing",
        }
    }

    /// 推定オーバーヘッド（相対値）
    pub const fn estimated_overhead(&self) -> u8 {
        match self {
            Self::StaticBalanced => 0,
            Self::StaticFast => 0,
            Self::StaticTesting => 0,
            Self::DynamicBalanced => 1,
            Self::DynamicFast => 1,
            Self::DynamicTesting => 1,
        }
    }
}

impl std::fmt::Debug for ProcessingEngineVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingEngineVariant::DefaultStatic(_) => f
                .debug_tuple("DefaultStatic")
                .field(&"ProcessingEngine<DefaultConfig>")
                .finish(),
            ProcessingEngineVariant::HighPerformanceStatic(_) => f
                .debug_tuple("HighPerformanceStatic")
                .field(&"ProcessingEngine<HighPerformanceConfig>")
                .finish(),
            ProcessingEngineVariant::TestingStatic(_) => f
                .debug_tuple("TestingStatic")
                .field(&"ProcessingEngine<TestingConfig>")
                .finish(),
            ProcessingEngineVariant::DefaultDynamic(_) => f
                .debug_tuple("DefaultDynamic")
                .field(&"BoxedProcessingEngine")
                .finish(),
            ProcessingEngineVariant::HighPerformanceDynamic(_) => f
                .debug_tuple("HighPerformanceDynamic")
                .field(&"BoxedProcessingEngine")
                .finish(),
            ProcessingEngineVariant::TestingDynamic(_) => f
                .debug_tuple("TestingDynamic")
                .field(&"BoxedProcessingEngine")
                .finish(),
        }
    }
}

/// 処理エンジンファクトリー - より高レベルなAPI
pub struct ProcessingEngineFactory;

impl ProcessingEngineFactory {
    /// 最適な処理エンジンを自動選択
    ///
    /// パフォーマンス重視なら静的、柔軟性重視なら動的を選択
    pub fn create_optimal(
        preset: &str,
        output_path: &Path,
        prefer_performance: bool,
    ) -> ProcessingResult<ProcessingEngineVariant> {
        let di_mode = if prefer_performance {
            DIMode::Static
        } else {
            DIMode::Dynamic
        };

        let unified_di = UnifiedDI::new(di_mode);
        unified_di.create_engine_with_preset(preset, output_path)
    }

    /// デフォルト性能エンジンを作成
    pub fn create_default(output_path: &Path) -> ProcessingResult<ProcessingEngineVariant> {
        Self::create_optimal("default", output_path, true)
    }

    /// 高性能エンジンを作成
    pub fn create_high_performance(
        output_path: &Path,
    ) -> ProcessingResult<ProcessingEngineVariant> {
        Self::create_optimal("high_performance", output_path, true)
    }

    /// テスト用エンジンを作成
    pub fn create_testing(output_path: &Path) -> ProcessingResult<ProcessingEngineVariant> {
        Self::create_optimal("testing", output_path, true)
    }

    /// 柔軟性重視エンジンを作成
    pub fn create_flexible(
        preset: &str,
        output_path: &Path,
    ) -> ProcessingResult<ProcessingEngineVariant> {
        Self::create_optimal(preset, output_path, false)
    }
}

// 便利な型エイリアス
pub type DefaultStaticEngine = StaticProcessingEngine<DefaultConfig>;
pub type HighPerformanceStaticEngine = StaticProcessingEngine<HighPerformanceConfig>;
pub type TestingStaticEngine = StaticProcessingEngine<TestingConfig>;
pub type DefaultDynamicEngine = BoxedProcessingEngine;
pub type HighPerformanceDynamicEngine = BoxedProcessingEngine;
pub type TestingDynamicEngine = BoxedProcessingEngine;

/// 統一DI作成マクロ
#[macro_export]
macro_rules! create_unified_di {
    (static) => {
        $crate::core::UnifiedDI::static_default()
    };
    (dynamic) => {
        $crate::core::UnifiedDI::dynamic()
    };
    (static, $preset:expr, $path:expr) => {{
        let di = $crate::core::UnifiedDI::static_default();
        di.create_engine_with_preset($preset, $path)
    }};
    (dynamic, $preset:expr, $path:expr) => {{
        let di = $crate::core::UnifiedDI::dynamic();
        di.create_engine_with_preset($preset, $path)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_unified_di_modes() {
        let static_di = UnifiedDI::static_default();
        assert_eq!(static_di.mode(), DIMode::Static);

        let dynamic_di = UnifiedDI::dynamic();
        assert_eq!(dynamic_di.mode(), DIMode::Dynamic);

        let default_di = UnifiedDI::default();
        assert_eq!(default_di.mode(), DIMode::Static); // デフォルトは静的
    }

    #[tokio::test]
    async fn test_static_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

        let di = UnifiedDI::static_default();
        let engine = di
            .create_engine_with_preset("default", &output_path)
            .unwrap();

        match engine {
            ProcessingEngineVariant::DefaultStatic(_) => {
                assert_eq!(engine.engine_type(), "StaticDefault");
                assert_eq!(
                    engine.performance_characteristics(),
                    PerformanceCharacteristics::StaticBalanced
                );
            }
            _ => panic!("Expected DefaultStatic engine"),
        }
    }

    #[tokio::test]
    async fn test_dynamic_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

        let di = UnifiedDI::dynamic();
        let engine = di
            .create_engine_with_preset("high_performance", &output_path)
            .unwrap();

        match engine {
            ProcessingEngineVariant::HighPerformanceDynamic(_) => {
                assert_eq!(engine.engine_type(), "DynamicHighPerformance");
                assert_eq!(
                    engine.performance_characteristics(),
                    PerformanceCharacteristics::DynamicFast
                );
            }
            _ => panic!("Expected HighPerformanceDynamic engine"),
        }
    }

    #[tokio::test]
    async fn test_engine_factory() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

        // 性能重視（静的）
        let engine =
            ProcessingEngineFactory::create_optimal("default", &output_path, true).unwrap();
        assert!(matches!(engine, ProcessingEngineVariant::DefaultStatic(_)));

        // 柔軟性重視（動的）
        let engine = ProcessingEngineFactory::create_flexible("testing", &output_path).unwrap();
        assert!(matches!(engine, ProcessingEngineVariant::TestingDynamic(_)));
    }

    #[tokio::test]
    async fn test_process_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");
        let target_path = temp_dir.path().to_str().unwrap();

        let engine = ProcessingEngineFactory::create_default(&output_path).unwrap();
        let result = engine.process_directory(target_path).await.unwrap();

        assert_eq!(result.total_files, 0);
        assert_eq!(result.processed_files, 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_performance_characteristics() {
        let static_balanced = PerformanceCharacteristics::StaticBalanced;
        assert_eq!(static_balanced.dispatch_type(), "Static");
        assert_eq!(static_balanced.performance_level(), "Balanced");
        assert_eq!(static_balanced.estimated_overhead(), 0);

        let dynamic_fast = PerformanceCharacteristics::DynamicFast;
        assert_eq!(dynamic_fast.dispatch_type(), "Dynamic");
        assert_eq!(dynamic_fast.performance_level(), "Fast");
        assert_eq!(dynamic_fast.estimated_overhead(), 1);
    }

    #[test]
    fn test_macro_creation() {
        let _static_di = create_unified_di!(static);
        let _dynamic_di = create_unified_di!(dynamic);
    }

    #[test]
    fn test_invalid_preset() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

        let di = UnifiedDI::static_default();
        let result = di.create_engine_with_preset("invalid_preset", &output_path);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("無効な静的プリセット"));
    }

    #[test]
    fn test_zero_cost_unified_di() {
        // UnifiedDIがゼロコストであることを確認
        assert_eq!(
            std::mem::size_of::<UnifiedDI>(),
            std::mem::size_of::<DIMode>()
        );
        assert_eq!(std::mem::size_of::<DIMode>(), 1); // enumなので1バイト
    }
}
