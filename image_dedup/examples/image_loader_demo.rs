use anyhow::Result;
use image::GenericImageView;
use image_dedup::{ImageLoadStrategy, ImageLoaderFactory};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ImageLoader 抽象化のデモ ===\n");

    // テスト画像の準備
    create_test_images().await?;

    // 1. 標準的なローダー
    println!("1. 標準的なImageLoader:");
    test_loader_strategy(&ImageLoadStrategy::Standard).await?;

    println!();

    // 2. メモリ効率重視のローダー
    println!("2. メモリ効率重視のImageLoader（1024px制限）:");
    test_loader_strategy(&ImageLoadStrategy::MemoryOptimized {
        max_dimension: 1024,
    })
    .await?;

    println!();

    // 3. 小さいサイズ制限のローダー
    println!("3. 小さいサイズ制限のImageLoader（256px制限）:");
    test_loader_strategy(&ImageLoadStrategy::MemoryOptimized { max_dimension: 256 }).await?;

    println!();

    // 4. 推奨戦略の例
    demonstrate_recommended_strategies().await;

    println!();

    // 5. パフォーマンス比較
    performance_comparison().await?;

    // テストファイルのクリーンアップ
    std::fs::remove_dir_all("./test_images_demo").ok();

    Ok(())
}

async fn test_loader_strategy(strategy: &ImageLoadStrategy) -> Result<()> {
    let loader = ImageLoaderFactory::create(strategy).await?;

    println!("  戦略: {}", loader.strategy_name());
    if let Some(max_pixels) = loader.max_supported_pixels() {
        println!("  最大ピクセル数: {max_pixels}");
    }

    // 複数のテスト画像で試行
    let test_files = [
        ("small_100x100.png", "小さな画像"),
        ("medium_800x600.png", "中サイズ画像"),
        ("large_2000x1500.png", "大きな画像"),
    ];

    for (filename, description) in test_files {
        let path = Path::new("./test_images_demo").join(filename);
        if path.exists() {
            match loader.load_from_path(&path).await {
                Ok(result) => {
                    let memory_usage =
                        loader.estimate_memory_usage(result.image.width(), result.image.height());

                    println!(
                        "  {}: {}x{} -> {}x{} ({}ms, {:.1}MB)",
                        description,
                        result.original_dimensions.0,
                        result.original_dimensions.1,
                        result.image.width(),
                        result.image.height(),
                        result.load_time_ms,
                        memory_usage as f64 / 1024.0 / 1024.0
                    );

                    if result.was_resized {
                        println!("    ↳ リサイズが実行されました");
                    }
                }
                Err(e) => {
                    println!("  {description}: 読み込みエラー - {e}");
                }
            }
        }
    }

    Ok(())
}

async fn demonstrate_recommended_strategies() {
    println!("4. システム推奨戦略:");

    let memory_scenarios = [
        (1.0, "低メモリ環境（1GB）"),
        (4.0, "標準環境（4GB）"),
        (16.0, "高メモリ環境（16GB）"),
        (64.0, "サーバー環境（64GB）"),
    ];

    for (memory_gb, description) in memory_scenarios {
        let strategy = ImageLoaderFactory::recommend_strategy(memory_gb);
        println!("  {description}: {strategy:?}");
    }
}

async fn performance_comparison() -> Result<()> {
    println!("5. パフォーマンス比較:");

    let strategies = [
        ImageLoadStrategy::Standard,
        ImageLoadStrategy::MemoryOptimized {
            max_dimension: 1024,
        },
        ImageLoadStrategy::MemoryOptimized { max_dimension: 512 },
    ];

    let test_file = Path::new("./test_images_demo/large_2000x1500.png");

    if test_file.exists() {
        for strategy in strategies {
            let loader = ImageLoaderFactory::create(&strategy).await?;

            // 3回測定して平均を取る
            let mut total_time = 0;
            let mut results = Vec::new();

            for _ in 0..3 {
                let result = loader.load_from_path(test_file).await?;
                total_time += result.load_time_ms;
                results.push(result);
            }

            let avg_time = total_time / 3;
            let final_result = &results[0];

            println!(
                "  {}: 平均 {}ms ({}x{} -> {}x{})",
                loader.strategy_name(),
                avg_time,
                final_result.original_dimensions.0,
                final_result.original_dimensions.1,
                final_result.image.width(),
                final_result.image.height()
            );
        }
    }

    Ok(())
}

async fn create_test_images() -> Result<()> {
    use image::{ImageBuffer, RgbImage};
    use std::fs;

    fs::create_dir_all("./test_images_demo")?;

    // 小さな画像 (100x100)
    let small_img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
        let r = (x * 255 / 100) as u8;
        let g = (y * 255 / 100) as u8;
        let b = ((x + y) * 255 / 200) as u8;
        image::Rgb([r, g, b])
    });
    small_img.save("./test_images_demo/small_100x100.png")?;

    // 中サイズ画像 (800x600)
    let medium_img: RgbImage = ImageBuffer::from_fn(800, 600, |x, y| {
        let r = ((x * y) % 256) as u8;
        let g = (x % 256) as u8;
        let b = (y % 256) as u8;
        image::Rgb([r, g, b])
    });
    medium_img.save("./test_images_demo/medium_800x600.png")?;

    // 大きな画像 (2000x1500)
    let large_img: RgbImage = ImageBuffer::from_fn(2000, 1500, |x, y| {
        let pattern = (x / 50) + (y / 50);
        let intensity = ((pattern * 30) % 256) as u8;
        image::Rgb([intensity, intensity / 2, intensity / 3])
    });
    large_img.save("./test_images_demo/large_2000x1500.png")?;

    println!("テスト画像を作成しました:");
    println!("  - 小: 100x100 (約 40KB)");
    println!("  - 中: 800x600 (約 1.5MB)");
    println!("  - 大: 2000x1500 (約 9MB)\n");

    Ok(())
}

// 将来的なGPU実装のデモンストレーション
#[allow(dead_code)]
async fn demonstrate_future_gpu_loading() {
    println!("=== 将来的なGPU対応の例 ===");
    println!("```rust");
    println!("// GPU加速ローダーを使用する場合（将来実装予定）");
    println!("let gpu_strategy = ImageLoadStrategy::GpuAccelerated;");
    println!("let gpu_loader = ImageLoaderFactory::create(&gpu_strategy).await?;");
    println!();
    println!("// 大量の画像を並列処理");
    println!("let results = futures::future::join_all(");
    println!("    image_paths.iter().map(|path| gpu_loader.load_from_path(path))");
    println!(").await;");
    println!("```");

    println!("\n将来的な拡張予定:");
    println!("1. GPU加速による高速な画像変換");
    println!("2. ストリーミング読み込み（大容量ファイル対応）");
    println!("3. キャッシュ機能付きローダー");
    println!("4. WebAssembly対応ローダー");
}
