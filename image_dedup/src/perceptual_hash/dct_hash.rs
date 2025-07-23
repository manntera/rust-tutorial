use super::{HashAlgorithm, HashResult, PerceptualHashBackend};
use anyhow::Result;
use async_trait::async_trait;
use image::DynamicImage;
use img_hash::{HashAlg, HasherConfig};
use std::time::Instant;

/// DCTベースの知覚ハッシュ実装
#[derive(Clone)]
pub struct DctHasher {
    algorithm: HashAlgorithm,
    hash_size: u32,
    quality_factor: f32,
}

impl DctHasher {
    pub fn new(size: u32) -> Result<Self> {
        Ok(Self {
            algorithm: HashAlgorithm::DCT { size },
            hash_size: size,
            quality_factor: 1.0,
        })
    }
    
    pub fn with_quality_factor(size: u32, quality_factor: f32) -> Result<Self> {
        Ok(Self {
            algorithm: HashAlgorithm::DCT { size },
            hash_size: size,
            quality_factor,
        })
    }
    
    pub fn get_size(&self) -> u32 {
        self.hash_size
    }
    
    pub fn get_quality_factor(&self) -> f32 {
        self.quality_factor
    }
}

#[async_trait]
impl PerceptualHashBackend for DctHasher {
    async fn generate_hash(&self, image: &DynamicImage) -> Result<HashResult> {
        let start_time = Instant::now();

        let hash = tokio::task::spawn_blocking({
            let image = image.clone();
            let size = self.hash_size;
            move || {
                let hasher = HasherConfig::new()
                    .hash_size(size, size)
                    .hash_alg(HashAlg::Mean)
                    .preproc_dct()
                    .to_hasher();

                let rgb_image = image.to_rgb8();
                let img_hash_image = img_hash::image::ImageBuffer::from_raw(
                    rgb_image.width(),
                    rgb_image.height(),
                    rgb_image.into_raw(),
                )
                .unwrap();
                let dynamic_img_hash_image =
                    img_hash::image::DynamicImage::ImageRgb8(img_hash_image);
                hasher.hash_image(&dynamic_img_hash_image)
            }
        })
        .await?;

        let computation_time_ms = start_time.elapsed().as_millis() as u64;

        // ImageHashからバイト配列に変換
        let hash_bytes = hash.as_bytes().to_vec();

        Ok(HashResult {
            hash_data: hash_bytes,
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
        "DCT (Discrete Cosine Transform)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    #[tokio::test]
    async fn test_dct_hash_generation() {
        let hasher = DctHasher::new(8);
        let image = DynamicImage::ImageRgb8(RgbImage::new(100, 100));

        let result = hasher.generate_hash(&image).await.unwrap();

        assert_eq!(result.hash_size_bits, 64);
        assert_eq!(result.algorithm, HashAlgorithm::DCT { size: 8 });
        assert_eq!(result.source_dimensions, (100, 100));
        assert!(!result.hash_data.is_empty());
        assert_eq!(hasher.algorithm_name(), "DCT (Discrete Cosine Transform)");
    }

    #[tokio::test]
    async fn test_dct_hash_similarity() {
        let hasher = DctHasher::new(8);

        // 同じ画像
        let image1 = DynamicImage::ImageRgb8(RgbImage::new(50, 50));
        let image2 = DynamicImage::ImageRgb8(RgbImage::new(50, 50));

        let hash1 = hasher.generate_hash(&image1).await.unwrap();
        let hash2 = hasher.generate_hash(&image2).await.unwrap();

        let distance = hasher.calculate_distance(&hash1, &hash2).unwrap();
        assert_eq!(distance, 0);

        let similar = hasher.are_similar(&hash1, &hash2, 5).unwrap();
        assert!(similar);
    }

    #[tokio::test]
    async fn test_dct_hash_different_sizes() {
        let hasher8 = DctHasher::new(8);
        let hasher16 = DctHasher::new(16);

        let image = DynamicImage::ImageRgb8(RgbImage::new(100, 100));

        let hash8 = hasher8.generate_hash(&image).await.unwrap();
        let hash16 = hasher16.generate_hash(&image).await.unwrap();

        assert_eq!(hash8.hash_size_bits, 64);
        assert_eq!(hash16.hash_size_bits, 256);

        // 異なるサイズのハッシュは比較できない
        let result = hasher8.calculate_distance(&hash8, &hash16);
        assert!(result.is_err());
    }
}
