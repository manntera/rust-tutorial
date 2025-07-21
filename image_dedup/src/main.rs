use anyhow::Result;
use image_dedup::{ImageLoader, PerceptualHasher, StorageFactory, StorageType};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== 画像重複検知プログラム - 機能テスト ===\n");

    println!("1. ストレージスキャンのテスト");
    test_storage_scan().await?;

    println!("\n2. 画像ローダーのテスト");
    test_image_loader()?;

    println!("\n3. 知覚ハッシュ生成のテスト");
    test_perceptual_hash()?;

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

fn test_image_loader() -> Result<()> {
    println!("  サンプル画像を作成して読み込みテスト...");

    use image::{ImageBuffer, RgbImage};
    use std::fs;

    fs::create_dir_all("./temp_test")?;

    let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
        if (x + y) % 2 == 0 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    });

    let test_path = Path::new("./temp_test/test_pattern.png");
    img.save(test_path)?;

    let loaded_image = ImageLoader::load_image(test_path)?;
    let (width, height) = ImageLoader::get_image_dimensions(&loaded_image);

    println!("  画像の読み込みに成功: {width}x{height}");

    fs::remove_dir_all("./temp_test")?;

    Ok(())
}

fn test_perceptual_hash() -> Result<()> {
    use image::{ImageBuffer, RgbImage};

    println!("  2つの画像を生成してハッシュを比較...");

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

    let hasher = PerceptualHasher::new();

    let hash1 = hasher.generate_hash(&image::DynamicImage::ImageRgb8(img1))?;
    let hash2 = hasher.generate_hash(&image::DynamicImage::ImageRgb8(img2))?;
    let hash3 = hasher.generate_hash(&image::DynamicImage::ImageRgb8(img3))?;

    let dist_same = PerceptualHasher::calculate_distance(&hash1, &hash2);
    let dist_diff = PerceptualHasher::calculate_distance(&hash1, &hash3);

    println!("  同じパターンの画像間の距離: {dist_same}");
    println!("  異なるパターンの画像間の距離: {dist_diff}");

    println!("  類似判定 (閾値: 10):");
    println!(
        "    同じパターン: {}",
        PerceptualHasher::are_similar(&hash1, &hash2, 10)
    );
    println!(
        "    異なるパターン: {}",
        PerceptualHasher::are_similar(&hash1, &hash3, 10)
    );

    Ok(())
}
