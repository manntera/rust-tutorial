use anyhow::Result;
use image::GenericImageView;
use image_dedup::{
    HashAlgorithm, ImageLoadStrategy, ImageLoaderFactory, PerceptualHashFactory, StorageFactory,
    StorageType,
};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== 画像重複検知プログラム - 機能テスト ===\n");

    println!("1. ストレージスキャンのテスト");
    test_storage_scan().await?;

    println!("\n2. 画像ローダーのテスト");
    test_image_loader().await?;

    println!("\n3. 知覚ハッシュ生成のテスト");
    test_perceptual_hash().await?;

    Ok(())
}

async fn test_storage_scan() -> Result<()> {
    let storage = StorageFactory::create(&StorageType::Local).await?;
    let test_dir = "./test_images";

    if !Path::new(test_dir).exists() {
        println!("  テストディレクトリが存在しません: {test_dir}");
        println!("  現在のディレクトリから画像ファイルを検索します...");

        let items = storage.list_items(".").await?;
        println!("  見つかった画像ファイル数: {}", items.len());
        for (i, item) in items.iter().take(5).enumerate() {
            println!("    {}. {} ({} bytes)", i + 1, item.name, item.size);
        }
        if items.len() > 5 {
            println!("    ... 他 {} ファイル", items.len() - 5);
        }
    } else {
        let items = storage.list_items(test_dir).await?;
        println!("  見つかった画像ファイル数: {}", items.len());
        for (i, item) in items.iter().enumerate() {
            println!("    {}. {} ({} bytes)", i + 1, item.name, item.size);
        }
    }

    Ok(())
}

async fn test_image_loader() -> Result<()> {
    println!("  新しいImageLoader APIのテスト...");

    use image::{ImageBuffer, RgbImage};
    use std::fs;

    fs::create_dir_all("./temp_test")?;

    // テスト用の画像を作成
    let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
        if (x + y) % 2 == 0 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    });

    let test_path = Path::new("./temp_test/test_pattern.png");
    img.save(test_path)?;

    // 標準的なローダーをテスト
    println!("  標準的なローダーでテスト:");
    let loader = ImageLoaderFactory::create(&ImageLoadStrategy::Standard).await?;
    let result = loader.load_from_path(test_path).await?;

    println!(
        "    読み込み成功: {}x{}",
        result.image.width(),
        result.image.height()
    );
    println!("    元のサイズ: {:?}", result.original_dimensions);
    println!("    リサイズ済み: {}", result.was_resized);
    println!("    読み込み時間: {}ms", result.load_time_ms);
    println!("    戦略: {}", loader.strategy_name());

    // メモリ効率重視のローダーをテスト
    println!("\n  メモリ効率重視のローダーでテスト（50px制限）:");
    let memory_loader =
        ImageLoaderFactory::create(&ImageLoadStrategy::MemoryOptimized { max_dimension: 50 })
            .await?;
    let memory_result = memory_loader.load_from_path(test_path).await?;

    println!(
        "    読み込み成功: {}x{}",
        memory_result.image.width(),
        memory_result.image.height()
    );
    println!("    元のサイズ: {:?}", memory_result.original_dimensions);
    println!("    リサイズ済み: {}", memory_result.was_resized);
    println!("    読み込み時間: {}ms", memory_result.load_time_ms);
    println!("    戦略: {}", memory_loader.strategy_name());

    // 推奨戦略のテスト
    println!("\n  システム推奨戦略:");
    let recommended = ImageLoaderFactory::recommend_strategy(4.0); // 4GB想定
    println!("    推奨戦略: {recommended:?}");

    fs::remove_dir_all("./temp_test")?;

    Ok(())
}

async fn test_perceptual_hash() -> Result<()> {
    use image::{ImageBuffer, RgbImage};

    println!("  新しいPerceptualHash APIのテスト...");

    // テスト画像を作成
    let img1: RgbImage = ImageBuffer::from_fn(64, 64, |x, y| {
        let val = ((x + y) * 255 / 128) as u8;
        image::Rgb([val, val, val])
    });

    let img2: RgbImage = ImageBuffer::from_fn(64, 64, |x, y| {
        let val = ((x + y) * 255 / 128) as u8;
        image::Rgb([val, val, val])
    });

    let img3: RgbImage = ImageBuffer::from_fn(64, 64, |x, _y| {
        let val = if x > 32 { 255 } else { 0 };
        image::Rgb([val, val, val])
    });

    let image1 = image::DynamicImage::ImageRgb8(img1);
    let image2 = image::DynamicImage::ImageRgb8(img2);
    let image3 = image::DynamicImage::ImageRgb8(img3);

    // DCTハッシュをテスト
    println!("\n  DCTハッシュでテスト:");
    let dct_hasher = PerceptualHashFactory::create(&HashAlgorithm::DCT { size: 8 }).await?;
    let dct_hash1 = dct_hasher.generate_hash(&image1).await?;
    let dct_hash2 = dct_hasher.generate_hash(&image2).await?;
    let dct_hash3 = dct_hasher.generate_hash(&image3).await?;

    let dct_dist_same = dct_hasher.calculate_distance(&dct_hash1, &dct_hash2)?;
    let dct_dist_diff = dct_hasher.calculate_distance(&dct_hash1, &dct_hash3)?;

    println!(
        "    同じパターンの距離: {} ({}ms)",
        dct_dist_same, dct_hash1.computation_time_ms
    );
    println!(
        "    異なるパターンの距離: {} ({}ms)",
        dct_dist_diff, dct_hash3.computation_time_ms
    );
    println!("    推奨閾値: {}", dct_hasher.recommended_threshold());

    // 平均ハッシュをテスト
    println!("\n  平均ハッシュでテスト:");
    let avg_hasher = PerceptualHashFactory::create(&HashAlgorithm::Average { size: 8 }).await?;
    let avg_hash1 = avg_hasher.generate_hash(&image1).await?;
    let avg_hash3 = avg_hasher.generate_hash(&image3).await?;

    let avg_dist = avg_hasher.calculate_distance(&avg_hash1, &avg_hash3)?;
    println!(
        "    異なるパターンの距離: {} ({}ms)",
        avg_dist, avg_hash1.computation_time_ms
    );
    println!("    アルゴリズム: {}", avg_hasher.algorithm_name());
    println!(
        "    計算複雑度: {}/10",
        avg_hasher.computational_complexity()
    );

    // 差分ハッシュをテスト
    println!("\n  差分ハッシュでテスト:");
    let diff_hasher = PerceptualHashFactory::create(&HashAlgorithm::Difference { size: 8 }).await?;
    let diff_hash1 = diff_hasher.generate_hash(&image1).await?;
    let diff_hash3 = diff_hasher.generate_hash(&image3).await?;

    let diff_dist = diff_hasher.calculate_distance(&diff_hash1, &diff_hash3)?;
    println!(
        "    異なるパターンの距離: {} ({}ms)",
        diff_dist, diff_hash1.computation_time_ms
    );
    println!("    アルゴリズム: {}", diff_hasher.algorithm_name());

    // ハッシュの様々な表現形式をテスト
    println!("\n  ハッシュの表現形式:");
    println!("    Base64: {}", dct_hash1.to_base64());
    println!("    Hex: {}", dct_hash1.to_hex());
    println!("    Bits: {}...", &dct_hash1.to_bits()[..16]); // 最初の16ビットのみ表示

    Ok(())
}
