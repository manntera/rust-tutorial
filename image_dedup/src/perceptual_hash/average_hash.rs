use super::{HashAlgorithm, HashResult, PerceptualHashBackend};
use anyhow::Result;
use async_trait::async_trait;
use image::DynamicImage;
use std::time::Instant;

/// 平均値ベースの知覚ハッシュ実装
#[derive(Clone)]
pub struct AverageHasher {
    algorithm: HashAlgorithm,
    hash_size: u32,
}

impl AverageHasher {
    pub fn new(size: u32) -> Self {
        Self {
            algorithm: HashAlgorithm::Average { size },
            hash_size: size,
        }
    }

    pub fn get_size(&self) -> u32 {
        self.hash_size
    }


    fn compute_average_hash_from_gray(gray_image: image::ImageBuffer<image::Luma<u8>, Vec<u8>>) -> Vec<u8> {
        let size = gray_image.width();

        // 平均輝度を計算
        let total: u32 = gray_image.pixels().map(|p| p[0] as u32).sum();
        let average = total / (size * size);

        // 各ピクセルが平均より明るいかどうかでビットを設定
        let mut hash_bits = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for pixel in gray_image.pixels() {
            let bit = if pixel[0] as u32 > average { 1 } else { 0 };
            current_byte = (current_byte << 1) | bit;
            bit_count += 1;

            if bit_count == 8 {
                hash_bits.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }

        // 残りのビットがある場合
        if bit_count > 0 {
            current_byte <<= 8 - bit_count;
            hash_bits.push(current_byte);
        }

        hash_bits
    }
}

#[async_trait]
impl PerceptualHashBackend for AverageHasher {
    async fn generate_hash(&self, image: &DynamicImage) -> Result<HashResult> {
        let start_time = Instant::now();

        let hash_data = tokio::task::spawn_blocking({
            let size = self.hash_size;
            let gray_image = image
                .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                .to_luma8();
            move || Self::compute_average_hash_from_gray(gray_image)
        })
        .await?;

        let computation_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(HashResult {
            hash_data,
            hash_size_bits: self.hash_size * self.hash_size,
            algorithm: self.algorithm.clone(),
            computation_time_ms,
            source_dimensions: (image.width(), image.height()),
        })
    }

    fn calculate_distance(&self, hash1: &HashResult, hash2: &HashResult) -> Result<u32> {
        if hash1.algorithm != hash2.algorithm {
            anyhow::bail!("Cannot compare hashes from different algorithms");
        }

        if hash1.hash_data.len() != hash2.hash_data.len() {
            anyhow::bail!("Cannot compare hashes of different sizes");
        }

        let distance = hash1
            .hash_data
            .iter()
            .zip(hash2.hash_data.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();

        Ok(distance)
    }

    fn algorithm(&self) -> &HashAlgorithm {
        &self.algorithm
    }

    fn algorithm_name(&self) -> &'static str {
        "Average Hash"
    }
}

/// 差分ベースの知覚ハッシュ実装
#[derive(Clone)]
pub struct DifferenceHasher {
    algorithm: HashAlgorithm,
    hash_size: u32,
}

impl DifferenceHasher {
    pub fn new(size: u32) -> Self {
        Self {
            algorithm: HashAlgorithm::Difference { size },
            hash_size: size,
        }
    }

    pub fn get_size(&self) -> u32 {
        self.hash_size
    }


    fn compute_difference_hash_from_gray(
        gray_image: image::ImageBuffer<image::Luma<u8>, Vec<u8>>, 
        size: u32
    ) -> Vec<u8> {

        // 隣接ピクセル間の差分でビットを設定
        let mut hash_bits = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for y in 0..size {
            for x in 0..size {
                let left_pixel = gray_image.get_pixel(x, y)[0] as u32;
                let right_pixel = gray_image.get_pixel(x + 1, y)[0] as u32;

                let bit = if left_pixel > right_pixel { 1 } else { 0 };
                current_byte = (current_byte << 1) | bit;
                bit_count += 1;

                if bit_count == 8 {
                    hash_bits.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }
        }

        // 残りのビットがある場合
        if bit_count > 0 {
            current_byte <<= 8 - bit_count;
            hash_bits.push(current_byte);
        }

        hash_bits
    }
}

#[async_trait]
impl PerceptualHashBackend for DifferenceHasher {
    async fn generate_hash(&self, image: &DynamicImage) -> Result<HashResult> {
        let start_time = Instant::now();

        let hash_data = tokio::task::spawn_blocking({
            let size = self.hash_size;
            let gray_image = image
                .resize_exact(size + 1, size, image::imageops::FilterType::Lanczos3)
                .to_luma8();
            move || Self::compute_difference_hash_from_gray(gray_image, size)
        })
        .await?;

        let computation_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(HashResult {
            hash_data,
            hash_size_bits: self.hash_size * self.hash_size,
            algorithm: self.algorithm.clone(),
            computation_time_ms,
            source_dimensions: (image.width(), image.height()),
        })
    }

    fn calculate_distance(&self, hash1: &HashResult, hash2: &HashResult) -> Result<u32> {
        if hash1.algorithm != hash2.algorithm {
            anyhow::bail!("Cannot compare hashes from different algorithms");
        }

        if hash1.hash_data.len() != hash2.hash_data.len() {
            anyhow::bail!("Cannot compare hashes of different sizes");
        }

        let distance = hash1
            .hash_data
            .iter()
            .zip(hash2.hash_data.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();

        Ok(distance)
    }

    fn algorithm(&self) -> &HashAlgorithm {
        &self.algorithm
    }

    fn algorithm_name(&self) -> &'static str {
        "Difference Hash"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, RgbImage};

    #[tokio::test]
    async fn test_average_hash() {
        let hasher = AverageHasher::new(8);

        // チェッカーボードパターンの画像を作成
        let img: RgbImage = ImageBuffer::from_fn(64, 64, |x, y| {
            if (x + y) % 2 == 0 {
                image::Rgb([255, 255, 255])
            } else {
                image::Rgb([0, 0, 0])
            }
        });
        let image = DynamicImage::ImageRgb8(img);

        let result = hasher.generate_hash(&image).await.unwrap();

        assert_eq!(result.hash_size_bits, 64);
        assert_eq!(result.algorithm, HashAlgorithm::Average { size: 8 });
        assert!(!result.hash_data.is_empty());
        assert_eq!(hasher.algorithm_name(), "Average Hash");
    }

    #[tokio::test]
    async fn test_difference_hash() {
        let hasher = DifferenceHasher::new(8);

        // グラデーション画像を作成
        let img: RgbImage = ImageBuffer::from_fn(64, 64, |x, _y| {
            let intensity = (x * 255 / 64) as u8;
            image::Rgb([intensity, intensity, intensity])
        });
        let image = DynamicImage::ImageRgb8(img);

        let result = hasher.generate_hash(&image).await.unwrap();

        assert_eq!(result.hash_size_bits, 64);
        assert_eq!(result.algorithm, HashAlgorithm::Difference { size: 8 });
        assert!(!result.hash_data.is_empty());
        assert_eq!(hasher.algorithm_name(), "Difference Hash");
    }

    #[tokio::test]
    async fn test_hash_comparison() {
        let avg_hasher = AverageHasher::new(8);
        let diff_hasher = DifferenceHasher::new(8);

        let image = DynamicImage::ImageRgb8(RgbImage::new(50, 50));

        let avg_hash = avg_hasher.generate_hash(&image).await.unwrap();
        let diff_hash = diff_hasher.generate_hash(&image).await.unwrap();

        // 同じアルゴリズム同士は比較可能
        let avg_distance = avg_hasher.calculate_distance(&avg_hash, &avg_hash).unwrap();
        assert_eq!(avg_distance, 0);

        // 異なるアルゴリズム同士は比較不可
        let result = avg_hasher.calculate_distance(&avg_hash, &diff_hash);
        assert!(result.is_err());
    }
}
