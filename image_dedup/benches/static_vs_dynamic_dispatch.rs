//! 静的ディスパッチ vs 動的ディスパッチのパフォーマンス比較ベンチマーク
//! 
//! このベンチマークは以下を検証します：
//! - 依存関係注入のオーバーヘッド
//! - 関数呼び出しの最適化レベル
//! - メモリ使用量の差

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use image_dedup::{
    ProcessingConfig,
    core::{
        DependencyContainer, DefaultConfig as StaticDefaultConfig,
        StaticDIContainer,
    },
    cli::commands::{ScanConfig, StaticScanConfig},
    perceptual_hash::{
        PerceptualHashBackend,
        average_hash::AverageHasher,
        average_config::AverageConfig,
        config::AlgorithmConfig,
    },
    image_loader::standard::StandardImageLoader,
    storage::local::LocalStorageBackend,
    services::{DefaultProcessingConfig, ConsoleProgressReporter, MemoryHashPersistence},
};
use std::time::Duration;
use tempfile::TempDir;

/// 動的コンポーネント群の型エイリアス
type DynamicComponents = (
    Box<dyn image_dedup::image_loader::ImageLoaderBackend>,
    Box<dyn image_dedup::perceptual_hash::PerceptualHashBackend>,
    Box<dyn image_dedup::storage::StorageBackend>,
    Box<dyn image_dedup::core::ProcessingConfig>,
    Box<dyn image_dedup::core::ProgressReporter>,
    Box<dyn image_dedup::core::HashPersistence>,  
);

/// 動的ディスパッチベンチマーク用のヘルパー
fn create_dynamic_components() -> DynamicComponents {
    let config = AverageConfig { size: 8 };
    (
        Box::new(StandardImageLoader::new()),
        Box::new(config.create_hasher().unwrap()),
        Box::new(LocalStorageBackend::new()),
        Box::new(DefaultProcessingConfig::new(4)),
        Box::new(ConsoleProgressReporter::new()),
        Box::new(MemoryHashPersistence::new()),
    )
}

/// 静的ディスパッチベンチマーク用のヘルパー
fn create_static_components() -> (
    StandardImageLoader,
    AverageHasher,
    LocalStorageBackend,
    DefaultProcessingConfig,
    ConsoleProgressReporter,
    MemoryHashPersistence,
) {
    let config = AverageConfig { size: 8 };
    (
        StandardImageLoader::new(),
        config.create_hasher().unwrap(),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(4),
        ConsoleProgressReporter::new(),
        MemoryHashPersistence::new(),
    )
}

/// 依存関係注入コンテナ作成のベンチマーク
fn bench_di_container_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("DI Container Creation");
    
    // 動的DIコンテナ作成
    group.bench_function("Dynamic DI Container", |b| {
        b.iter(|| {
            let container = DependencyContainer::default();
            black_box(container);
        })
    });
    
    // 静的DIコンテナ作成
    group.bench_function("Static DI Container", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<StaticDefaultConfig>::new();
            black_box(container);
        })
    });
    
    group.finish();
}

/// コンポーネント作成のベンチマーク
fn bench_component_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Component Creation");
    
    // 動的コンポーネント作成
    group.bench_function("Dynamic Components", |b| {
        b.iter(|| {
            let components = create_dynamic_components();
            black_box(components);
        })
    });
    
    // 静的コンポーネント作成
    group.bench_function("Static Components", |b| {
        b.iter(|| {
            let components = create_static_components();
            black_box(components);
        })
    });
    
    group.finish();
}

/// 関数呼び出しオーバーヘッドのベンチマーク
fn bench_function_call_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("Function Call Overhead");
    
    let (_, _, _, dyn_config, _, _) = create_dynamic_components();
    let (_, _, _, static_config, _, _) = create_static_components();
    
    // 動的ディスパッチでの関数呼び出し
    group.bench_function("Dynamic Function Calls", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(dyn_config.max_concurrent_tasks());
                black_box(dyn_config.batch_size());
                black_box(dyn_config.channel_buffer_size());
            }
        })
    });
    
    // 静的ディスパッチでの関数呼び出し
    group.bench_function("Static Function Calls", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(static_config.max_concurrent_tasks());
                black_box(static_config.batch_size());
                black_box(static_config.channel_buffer_size());
            }
        })
    });
    
    group.finish();
}

/// ProcessingEngine作成のベンチマーク
fn bench_processing_engine_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ProcessingEngine Creation");
    group.measurement_time(Duration::from_secs(10));
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("benchmark.json");
    
    // 動的ディスパッチProcessingEngine
    group.bench_function("Dynamic ProcessingEngine", |b| {
        b.iter(|| {
            let container = DependencyContainer::default();
            let dependencies = container.resolve_all_dependencies(&output_path).unwrap();
            let engine = dependencies.create_processing_engine();
            black_box(engine);
        })
    });
    
    // 静的ディスパッチProcessingEngine
    group.bench_function("Static ProcessingEngine", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<StaticDefaultConfig>::new();
            let engine = container.create_processing_engine(&output_path);
            black_box(engine);
        })
    });
    
    group.finish();
}

/// ハッシュ計算のベンチマーク（実際の処理での性能差）
fn bench_hash_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Hash Computation");
    group.measurement_time(Duration::from_secs(5));
    
    // ダミー画像データ（8x8 グレースケール）
    let dummy_image = image::DynamicImage::new_rgb8(8, 8);
    
    let (_, dyn_hasher, _, _, _, _) = create_dynamic_components();
    let (_, static_hasher, _, _, _, _) = create_static_components();
    
    // 動的ディスパッチでのハッシュ計算
    group.bench_function("Dynamic Hash Computation", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    let result = dyn_hasher.generate_hash(&dummy_image).await;
                    let _ = black_box(result);
                }
            })
        })
    });
    
    // 静的ディスパッチでのハッシュ計算
    group.bench_function("Static Hash Computation", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    let result = static_hasher.generate_hash(&dummy_image).await;
                    let _ = black_box(result);
                }
            })
        })
    });
    
    group.finish();
}

/// メモリ使用量の比較ベンチマーク
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("Memory Usage");
    
    // 静的DIコンテナのメモリ使用量測定
    group.bench_function("Static DI Memory", |b| {
        b.iter(|| {
            let containers: Vec<StaticDIContainer<StaticDefaultConfig>> = 
                (0..1000).map(|_| StaticDIContainer::new()).collect();
            black_box(containers);
        })
    });
    
    // 動的DIコンテナのメモリ使用量測定
    group.bench_function("Dynamic DI Memory", |b| {
        b.iter(|| {
            let containers: Vec<DependencyContainer> = 
                (0..1000).map(|_| DependencyContainer::default()).collect();
            black_box(containers);
        })
    });
    
    group.finish();
}

/// 全体的なワークフローのベンチマーク
fn bench_full_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("Full Workflow");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);
    
    // テスト用の空ディレクトリ作成
    let temp_dir = TempDir::new().unwrap();
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();
    
    // 動的ディスパッチでの完全ワークフロー
    group.bench_function("Dynamic Full Workflow", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let output = temp_dir.path().join("dynamic_output.json");
                let config = ScanConfig {
                    target_directory: target_dir.clone(),
                    output,
                    threads: Some(1),
                    force: true,
                };
                let container = DependencyContainer::default();
                let result = image_dedup::cli::commands::execute_scan_with_container(config, container).await;
                let _ = black_box(result);
            })
        })
    });
    
    // 静的ディスパッチでの完全ワークフロー
    group.bench_function("Static Full Workflow", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let output = temp_dir.path().join("static_output.json");
                let config = StaticScanConfig {
                    target_directory: target_dir.clone(),
                    output,
                    threads: Some(1),
                    force: true,
                };
                let result = image_dedup::cli::commands::execute_default_scan(config).await;
                let _ = black_box(result);
            })
        })
    });
    
    group.finish();
}

/// パフォーマンス回帰テスト
fn bench_performance_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("Performance Regression");
    
    // 静的ディスパッチが動的ディスパッチより速いことを確認
    let (_, dyn_hasher, _, _, _, _) = create_dynamic_components();
    let (_, static_hasher, _, _, _, _) = create_static_components();
    
    group.bench_function("Regression Test - Dynamic", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let dummy_img = image::DynamicImage::new_rgb8(8, 8);
                let result = dyn_hasher.generate_hash(&dummy_img).await;
                let _ = black_box(result);
            })
        })
    });
    
    group.bench_function("Regression Test - Static", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let dummy_img = image::DynamicImage::new_rgb8(8, 8);
                let result = static_hasher.generate_hash(&dummy_img).await;
                let _ = black_box(result);
            })
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_di_container_creation,
    bench_component_creation,
    bench_function_call_overhead,
    bench_processing_engine_creation,
    bench_hash_computation,
    bench_memory_usage,
    bench_full_workflow,
    bench_performance_regression,
);

criterion_main!(benches);