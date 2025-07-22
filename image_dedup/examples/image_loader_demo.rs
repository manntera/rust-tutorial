use anyhow::Result;
use image_dedup::image_loader::{ImageLoaderBackend, LoadResult, standard::StandardImageLoader};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ImageLoader 抽象化のデモ ===\n");

    // テスト画像の準備
    create_test_images().await?;

    // 1. 標準的なローダー
    println!("1. 標準的なImageLoader:");
    let standard_loader = StandardImageLoader::new();
    test_loader(&standard_loader).await?;

    println!();

    // 2. メモリ効率重視のローダー
    println!("2. メモリ効率重視のImageLoader（1024px制限）:");
    let memory_loader = StandardImageLoader::with_max_dimension(1024);
    test_loader(&memory_loader).await?;

    println!();

    // 3. 小さいサイズ制限のローダー
    println!("3. 小さいサイズ制限のImageLoader（256px制限）:");
    let small_loader = StandardImageLoader::with_max_dimension(256);
    test_loader(&small_loader).await?;

    println!();

    // 4. パフォーマンス比較
    performance_comparison().await?;

    // テストファイルのクリーンアップ
    std::fs::remove_dir_all("./test_images_demo").ok();

    Ok(())
}

async fn test_loader(loader: &impl ImageLoaderBackend) -> Result<()> {
    let strategy_name = loader.strategy_name();
    println!("  戦略: {strategy_name}");
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

async fn performance_comparison() -> Result<()> {
    println!("4. パフォーマンス比較:");

    let loaders: Vec<Box<dyn ImageLoaderBackend>> = vec![
        Box::new(StandardImageLoader::new()),
        Box::new(StandardImageLoader::with_max_dimension(1024)),
        Box::new(StandardImageLoader::with_max_dimension(512)),
    ];

    let test_file = Path::new("./test_images_demo/large_2000x1500.png");

    if test_file.exists() {
        for loader in loaders {
            // 3回測定して平均を取る
            let mut total_time = 0;
            let mut results: Vec<LoadResult> = Vec::new();

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
