//! 静的ディスパッチ設定間のパフォーマンス比較ベンチマーク
//!
//! 異なる静的設定間でのパフォーマンス差を測定

use anyhow::Result;
use criterion::{criterion_group, criterion_main, Criterion};
use image_dedup::core::{
    traits::ProcessingConfig, DefaultConfig, HighPerformanceConfig, StaticDIContainer,
    TestingConfig,
};
use std::time::Duration;
use tempfile::TempDir;

/// DIコンテナ作成のベンチマーク
fn benchmark_di_container_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("DI Container Creation");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("DefaultConfig", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<DefaultConfig>::new();
            std::hint::black_box(container)
        })
    });

    group.bench_function("HighPerformanceConfig", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<HighPerformanceConfig>::new();
            std::hint::black_box(container)
        })
    });

    group.bench_function("TestingConfig", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<TestingConfig>::new();
            std::hint::black_box(container)
        })
    });

    group.finish();
}

/// 設定アクセスのベンチマーク
fn benchmark_config_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("Config Access");
    group.measurement_time(Duration::from_secs(10));

    let default_container = StaticDIContainer::<DefaultConfig>::new();
    let default_config = default_container.create_processing_config();

    let hp_container = StaticDIContainer::<HighPerformanceConfig>::new();
    let hp_config = hp_container.create_processing_config();

    let test_container = StaticDIContainer::<TestingConfig>::new();
    let test_config = test_container.create_processing_config();

    group.bench_function("DefaultConfig", |b| {
        b.iter(|| {
            std::hint::black_box(default_config.max_concurrent_tasks());
            std::hint::black_box(default_config.batch_size());
            std::hint::black_box(default_config.channel_buffer_size());
        })
    });

    group.bench_function("HighPerformanceConfig", |b| {
        b.iter(|| {
            std::hint::black_box(hp_config.max_concurrent_tasks());
            std::hint::black_box(hp_config.batch_size());
            std::hint::black_box(hp_config.channel_buffer_size());
        })
    });

    group.bench_function("TestingConfig", |b| {
        b.iter(|| {
            std::hint::black_box(test_config.max_concurrent_tasks());
            std::hint::black_box(test_config.batch_size());
            std::hint::black_box(test_config.channel_buffer_size());
        })
    });

    group.finish();
}

/// ProcessingEngine作成のベンチマーク
fn benchmark_processing_engine_creation(c: &mut Criterion) -> Result<()> {
    let mut group = c.benchmark_group("ProcessingEngine Creation");
    group.measurement_time(Duration::from_secs(10));

    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("test.json");

    group.bench_function("DefaultConfig", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<DefaultConfig>::new();
            let engine = container.create_processing_engine(&output_path);
            std::hint::black_box(engine)
        })
    });

    group.bench_function("HighPerformanceConfig", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<HighPerformanceConfig>::new();
            let engine = container.create_processing_engine(&output_path);
            std::hint::black_box(engine)
        })
    });

    group.bench_function("TestingConfig", |b| {
        b.iter(|| {
            let container = StaticDIContainer::<TestingConfig>::new();
            let engine = container.create_processing_engine(&output_path);
            std::hint::black_box(engine)
        })
    });

    group.finish();
    Ok(())
}

/// メモリサイズ測定
fn benchmark_memory_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("Memory Sizes");

    group.bench_function("Container Sizes", |b| {
        b.iter(|| {
            let default_size = std::mem::size_of::<StaticDIContainer<DefaultConfig>>();
            let hp_size = std::mem::size_of::<StaticDIContainer<HighPerformanceConfig>>();
            let test_size = std::mem::size_of::<StaticDIContainer<TestingConfig>>();

            std::hint::black_box((default_size, hp_size, test_size))
        })
    });

    group.finish();
}

// Wrapper function to handle Result return type for criterion
fn benchmark_processing_engine_creation_wrapper(c: &mut Criterion) {
    if let Err(e) = benchmark_processing_engine_creation(c) {
        panic!("Benchmark failed: {e}");
    }
}

criterion_group!(
    benches,
    benchmark_di_container_creation,
    benchmark_config_access,
    benchmark_processing_engine_creation_wrapper,
    benchmark_memory_sizes
);
criterion_main!(benches);
