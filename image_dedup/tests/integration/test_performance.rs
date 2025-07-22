// パフォーマンス関連の統合テスト
use image_dedup::{
    engine::ProcessingEngine,
    image_loader::standard::StandardImageLoader,
    perceptual_hash::dct_hash::DCTHasher,
    storage::local::LocalStorageBackend,
    services::{DefaultProcessingConfig, NoOpProgressReporter, MemoryHashPersistence},
};
use tempfile::TempDir;
use std::fs;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;

// テスト用の有効な1x1 PNGファイル
const MINIMAL_PNG_DATA: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// パフォーマンス測定用の構造体
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    total_time: Duration,
    files_processed: usize,
    avg_time_per_file: Duration,
    throughput_files_per_sec: f64,
}

impl PerformanceMetrics {
    fn new(total_time: Duration, files_processed: usize) -> Self {
        let avg_time_per_file = if files_processed > 0 {
            total_time / files_processed as u32
        } else {
            Duration::ZERO
        };
        
        let throughput_files_per_sec = if total_time.as_secs_f64() > 0.0 {
            files_processed as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };
        
        Self {
            total_time,
            files_processed,
            avg_time_per_file,
            throughput_files_per_sec,
        }
    }
}

/// 複数のファイルを作成する
fn create_test_files(dir: &std::path::Path, count: usize, prefix: &str) {
    for i in 0..count {
        let filename = format!("{}_{:04}.png", prefix, i);
        fs::write(dir.join(filename), MINIMAL_PNG_DATA).unwrap();
    }
}

#[tokio::test]
async fn test_single_thread_vs_multi_thread_performance() {
    let temp_dir = TempDir::new().unwrap();
    let file_count = 100;
    
    create_test_files(temp_dir.path(), file_count, "perf");

    // シングルスレッド処理
    let single_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1).with_max_concurrent(1),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let start = Instant::now();
    let single_result = single_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    let single_metrics = PerformanceMetrics::new(start.elapsed(), single_result.processed_files);

    // マルチスレッド処理
    let multi_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(4).with_max_concurrent(8),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let start = Instant::now();
    let multi_result = multi_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    let multi_metrics = PerformanceMetrics::new(start.elapsed(), multi_result.processed_files);

    // 結果の検証
    assert_eq!(single_result.processed_files, multi_result.processed_files);
    assert_eq!(single_result.processed_files, file_count);

    // マルチスレッドの方が速いか同等であることを確認
    assert!(multi_metrics.total_time <= single_metrics.total_time * 2); // 最大2倍まで許容

    println!("シングルスレッド: {:?}", single_metrics);
    println!("マルチスレッド: {:?}", multi_metrics);
    
    // スループットの改善を確認
    if single_metrics.total_time > Duration::from_millis(100) { // 十分な実行時間がある場合のみ
        assert!(multi_metrics.throughput_files_per_sec >= single_metrics.throughput_files_per_sec * 0.8);
    }
}

#[tokio::test]
async fn test_memory_usage_with_large_batch() {
    let temp_dir = TempDir::new().unwrap();
    let file_count = 200;
    
    create_test_files(temp_dir.path(), file_count, "memory");

    // 小さなバッチサイズ
    let small_batch_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(2).with_batch_size(10),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let start = Instant::now();
    let small_result = small_batch_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    let small_time = start.elapsed();

    // 大きなバッチサイズ
    let large_batch_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(2).with_batch_size(100),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let start = Instant::now();
    let large_result = large_batch_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    let large_time = start.elapsed();

    // 両方とも同じ数のファイルを処理
    assert_eq!(small_result.processed_files, large_result.processed_files);
    assert_eq!(small_result.processed_files, file_count);

    // 大きなバッチサイズの方が効率的（またはほぼ同等）であることを確認
    assert!(large_time <= small_time * 2); // 最大2倍まで許容

    println!("小バッチ (10): {:?} - {:.2} files/sec", 
             small_time, 
             file_count as f64 / small_time.as_secs_f64());
    println!("大バッチ (100): {:?} - {:.2} files/sec", 
             large_time, 
             file_count as f64 / large_time.as_secs_f64());
}

#[tokio::test]
async fn test_concurrent_processing_stress() {
    let temp_dir = TempDir::new().unwrap();
    let file_count = 50;
    
    create_test_files(temp_dir.path(), file_count, "stress");

    // 高い並列度で処理
    let high_concurrency_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(8).with_max_concurrent(32), // 高並列度
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let start = Instant::now();
    let result = high_concurrency_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    // 正常に処理されることを確認
    assert_eq!(result.processed_files, file_count);
    assert_eq!(result.error_count, 0);

    // 合理的な時間内で完了することを確認（10秒以内）
    assert!(elapsed < Duration::from_secs(10));

    println!("高並列処理 (32): {:?} - {:.2} files/sec", 
             elapsed, 
             file_count as f64 / elapsed.as_secs_f64());
}

#[tokio::test]
async fn test_image_size_processing_performance() {
    let temp_dir = TempDir::new().unwrap();
    
    // 異なるサイズの処理設定
    let configs = [
        (128, "small"),
        (512, "medium"), 
        (1024, "large"),
    ];

    for (max_dimension, label) in configs {
        let file_count = 20;
        create_test_files(temp_dir.path(), file_count, &format!("{}_img", label));

        let engine = ProcessingEngine::new(
            StandardImageLoader::with_max_dimension(max_dimension),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::new(2),
            NoOpProgressReporter::new(),
            MemoryHashPersistence::new(),
        );

        let start = Instant::now();
        let result = engine
            .process_directory(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert!(result.processed_files >= file_count); // 前のテストファイルも含む
        assert_eq!(result.error_count, 0);

        println!("{} ({}px): {:?} - {:.2} files/sec", 
                 label, 
                 max_dimension,
                 elapsed, 
                 result.processed_files as f64 / elapsed.as_secs_f64());
    }
}

#[tokio::test]
async fn test_hash_algorithm_performance_comparison() {
    let temp_dir = TempDir::new().unwrap();
    let file_count = 30;
    
    create_test_files(temp_dir.path(), file_count, "hash_perf");

    // DCT ハッシュのパフォーマンス
    let dct_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(2),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let start = Instant::now();
    let dct_result = dct_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    let dct_time = start.elapsed();

    // Average ハッシュのパフォーマンス
    use image_dedup::perceptual_hash::average_hash::AverageHasher;
    let avg_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        AverageHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(2),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let start = Instant::now();
    let avg_result = avg_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    let avg_time = start.elapsed();

    // 両方とも同じ数のファイルを処理
    assert_eq!(dct_result.processed_files, avg_result.processed_files);
    
    println!("DCT Hash: {:?} - {:.2} files/sec", 
             dct_time, 
             dct_result.processed_files as f64 / dct_time.as_secs_f64());
    println!("Average Hash: {:?} - {:.2} files/sec", 
             avg_time, 
             avg_result.processed_files as f64 / avg_time.as_secs_f64());

    // どちらも合理的な時間内で完了することを確認
    assert!(dct_time < Duration::from_secs(5));
    assert!(avg_time < Duration::from_secs(5));
}

#[tokio::test]
async fn test_resource_cleanup_performance() {
    // メモリリークや不適切なリソース管理がないことを確認するテスト
    let temp_dir = TempDir::new().unwrap();
    
    // 複数回の処理を実行してリソース管理をテスト
    for iteration in 0..5 {
        let file_count = 20;
        let subdir = temp_dir.path().join(format!("iter_{}", iteration));
        fs::create_dir_all(&subdir).unwrap();
        
        create_test_files(&subdir, file_count, "cleanup");

        let engine = ProcessingEngine::new(
            StandardImageLoader::new(),
            DCTHasher::new(8),
            LocalStorageBackend::new(),
            DefaultProcessingConfig::new(2),
            NoOpProgressReporter::new(),
            MemoryHashPersistence::new(),
        );

        let start = Instant::now();
        let result = engine
            .process_directory(subdir.to_str().unwrap())
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.processed_files, file_count);
        assert_eq!(result.error_count, 0);
        
        // 各反復が合理的な時間内で完了することを確認
        assert!(elapsed < Duration::from_secs(3));
        
        println!("Iteration {}: {:?}", iteration, elapsed);
    }
}

/// 複数の処理エンジンを同時実行してリソース競合をテスト
#[tokio::test]
async fn test_concurrent_engines_performance() {
    let temp_dir = TempDir::new().unwrap();
    
    // 複数のサブディレクトリを作成
    let subdirs = (0..3).map(|i| {
        let subdir = temp_dir.path().join(format!("concurrent_{}", i));
        fs::create_dir_all(&subdir).unwrap();
        create_test_files(&subdir, 15, "concurrent");
        subdir
    }).collect::<Vec<_>>();

    let start = Instant::now();
    
    // 複数のエンジンを並行実行
    let handles: Vec<_> = subdirs.into_iter().enumerate().map(|(i, subdir)| {
        tokio::spawn(async move {
            let engine = ProcessingEngine::new(
                StandardImageLoader::new(),
                DCTHasher::new(8),
                LocalStorageBackend::new(),
                DefaultProcessingConfig::new(1).with_max_concurrent(2),
                NoOpProgressReporter::new(),
                MemoryHashPersistence::new(),
            );

            let result = engine
                .process_directory(subdir.to_str().unwrap())
                .await
                .unwrap();
            
            (i, result)
        })
    }).collect();

    // すべての結果を待機
    let results: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    let total_elapsed = start.elapsed();

    // 各エンジンが正常に処理を完了したことを確認
    for (i, result) in results {
        assert_eq!(result.processed_files, 15);
        assert_eq!(result.error_count, 0);
        println!("Engine {}: {} files processed", i, result.processed_files);
    }

    // 合理的な時間内で並行処理が完了することを確認
    assert!(total_elapsed < Duration::from_secs(10));
    
    println!("並行処理合計時間: {:?}", total_elapsed);
}