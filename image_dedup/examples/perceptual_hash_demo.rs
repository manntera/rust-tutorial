use anyhow::Result;
use image::{ImageBuffer, RgbImage};
use image_dedup::{HashAlgorithm, HashStrategy, PerceptualHashFactory};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== PerceptualHash 抽象化のデモ ===\n");

    // テスト画像を作成
    let test_images = create_test_images();

    // 1. 異なるアルゴリズムの比較
    compare_algorithms(&test_images).await?;

    println!();

    // 2. ハッシュサイズの比較
    compare_hash_sizes(&test_images[0]).await?;

    println!();

    // 3. パフォーマンス比較
    performance_comparison(&test_images).await?;

    println!();

    // 4. 推奨設定のデモ
    demonstrate_recommendations().await;

    Ok(())
}

fn create_test_images() -> Vec<image::DynamicImage> {
    let mut images = Vec::new();

    // 1. グラデーション画像
    let gradient: RgbImage = ImageBuffer::from_fn(128, 128, |x, y| {
        let r = (x * 255 / 128) as u8;
        let g = (y * 255 / 128) as u8;
        let b = ((x + y) * 255 / 256) as u8;
        image::Rgb([r, g, b])
    });
    images.push(image::DynamicImage::ImageRgb8(gradient));

    // 2. チェッカーボードパターン
    let checkerboard: RgbImage = ImageBuffer::from_fn(128, 128, |x, y| {
        let cell_size = 16;
        let checker = ((x / cell_size) + (y / cell_size)) % 2;
        let color = if checker == 0 { 255 } else { 0 };
        image::Rgb([color, color, color])
    });
    images.push(image::DynamicImage::ImageRgb8(checkerboard));

    // 3. 円パターン
    let circle: RgbImage = ImageBuffer::from_fn(128, 128, |x, y| {
        let center_x = 64.0;
        let center_y = 64.0;
        let radius = 40.0;
        let distance = ((x as f64 - center_x).powi(2) + (y as f64 - center_y).powi(2)).sqrt();
        let intensity = if distance < radius { 255 } else { 0 };
        image::Rgb([intensity, intensity, intensity])
    });
    images.push(image::DynamicImage::ImageRgb8(circle));

    // 4. ノイズパターン
    let noise: RgbImage = ImageBuffer::from_fn(128, 128, |x, y| {
        let pseudo_random = ((x * 127 + y * 31) % 256) as u8;
        image::Rgb([pseudo_random, pseudo_random, pseudo_random])
    });
    images.push(image::DynamicImage::ImageRgb8(noise));

    images
}

async fn compare_algorithms(test_images: &[image::DynamicImage]) -> Result<()> {
    println!("1. 異なるアルゴリズムの比較:");

    let algorithms = [
        HashAlgorithm::DCT { size: 8 },
        HashAlgorithm::Average { size: 8 },
        HashAlgorithm::Difference { size: 8 },
    ];

    for algorithm in &algorithms {
        println!("\n  {}:", get_algorithm_description(algorithm));

        let hasher = PerceptualHashFactory::create(algorithm).await?;
        println!("    計算複雑度: {}/10", hasher.computational_complexity());
        println!("    推奨閾値: {}", hasher.recommended_threshold());

        // 最初の2つの画像で類似性をテスト
        let hash1 = hasher.generate_hash(&test_images[0]).await?;
        let hash2 = hasher.generate_hash(&test_images[1]).await?;

        let distance = hasher.calculate_distance(&hash1, &hash2)?;
        let is_similar = hasher.are_similar(&hash1, &hash2, hasher.recommended_threshold())?;

        println!("    グラデーション vs チェッカーボード:");
        println!("      距離: {}", distance);
        println!(
            "      類似判定: {}",
            if is_similar { "類似" } else { "非類似" }
        );
        println!(
            "      計算時間: {}ms / {}ms",
            hash1.computation_time_ms, hash2.computation_time_ms
        );
    }

    Ok(())
}

async fn compare_hash_sizes(test_image: &image::DynamicImage) -> Result<()> {
    println!("2. ハッシュサイズの比較 (DCTアルゴリズム):");

    let sizes = [8, 16, 32];

    for size in sizes {
        let algorithm = HashAlgorithm::DCT { size };
        let hasher = PerceptualHashFactory::create(&algorithm).await?;

        let hash = hasher.generate_hash(test_image).await?;

        println!("  {}x{} ハッシュ:", size, size);
        println!("    ビット数: {}", hash.hash_size_bits);
        println!("    計算時間: {}ms", hash.computation_time_ms);
        println!("    推奨閾値: {}", hasher.recommended_threshold());
        println!("    ハッシュ値: {}...", &hash.to_hex()[..16]);
    }

    Ok(())
}

async fn performance_comparison(test_images: &[image::DynamicImage]) -> Result<()> {
    println!("3. パフォーマンス比較 ({} 画像):", test_images.len());

    let algorithms = [
        HashAlgorithm::Average { size: 8 },
        HashAlgorithm::Difference { size: 8 },
        HashAlgorithm::DCT { size: 8 },
        HashAlgorithm::DCT { size: 16 },
    ];

    for algorithm in &algorithms {
        let hasher = PerceptualHashFactory::create(algorithm).await?;
        let mut total_time = 0u64;
        let mut hashes = Vec::new();

        // 全画像でハッシュを計算
        for image in test_images {
            let hash = hasher.generate_hash(image).await?;
            total_time += hash.computation_time_ms;
            hashes.push(hash);
        }

        // 類似性マトリックスを計算
        let mut comparisons = 0;
        let mut similar_pairs = 0;

        for i in 0..hashes.len() {
            for j in i + 1..hashes.len() {
                let distance = hasher.calculate_distance(&hashes[i], &hashes[j])?;
                let is_similar = distance <= hasher.recommended_threshold();

                comparisons += 1;
                if is_similar {
                    similar_pairs += 1;
                }
            }
        }

        println!("  {}:", get_algorithm_description(algorithm));
        println!("    総計算時間: {}ms", total_time);
        println!(
            "    平均計算時間: {}ms",
            total_time / test_images.len() as u64
        );
        println!("    類似ペア: {}/{} 組", similar_pairs, comparisons);
    }

    Ok(())
}

async fn demonstrate_recommendations() {
    println!("4. 推奨設定のデモ:");

    println!("\n  用途別推奨アルゴリズム:");

    let speed_strategy = HashStrategy {
        algorithm: HashAlgorithm::Average { size: 8 },
        priority_speed: true,
        priority_accuracy: false,
    };
    let speed_rec = PerceptualHashFactory::recommend_algorithm(&speed_strategy);
    println!("    高速処理重視: {:?}", speed_rec);

    let accuracy_strategy = HashStrategy {
        algorithm: HashAlgorithm::DCT { size: 16 },
        priority_speed: false,
        priority_accuracy: true,
    };
    let accuracy_rec = PerceptualHashFactory::recommend_algorithm(&accuracy_strategy);
    println!("    高精度重視: {:?}", accuracy_rec);

    let balanced_strategy = HashStrategy {
        algorithm: HashAlgorithm::DCT { size: 8 },
        priority_speed: true,
        priority_accuracy: true,
    };
    let balanced_rec = PerceptualHashFactory::recommend_algorithm(&balanced_strategy);
    println!("    バランス重視: {:?}", balanced_rec);

    println!("\n  画像サイズ別推奨ハッシュサイズ:");
    let image_sizes = [(128, 128), (512, 512), (2048, 2048), (8192, 8192)];

    for (width, height) in image_sizes {
        let recommended_size = PerceptualHashFactory::recommend_hash_size(width, height);
        println!(
            "    {}x{} 画像 → {}x{} ハッシュ",
            width, height, recommended_size, recommended_size
        );
    }

    println!("\n  将来的な拡張予定:");
    println!("    - Wavelet Hash: 周波数解析ベース");
    println!("    - Block Hash: ブロック分割ベース");
    println!("    - Neural Hash: 機械学習ベース");
    println!("    - GPU加速: 並列計算による高速化");
}

fn get_algorithm_description(algorithm: &HashAlgorithm) -> &'static str {
    match algorithm {
        HashAlgorithm::DCT { .. } => "DCT Hash",
        HashAlgorithm::Average { .. } => "Average Hash",
        HashAlgorithm::Difference { .. } => "Difference Hash",
        HashAlgorithm::Wavelet { .. } => "Wavelet Hash",
        HashAlgorithm::Block { .. } => "Block Hash",
    }
}
