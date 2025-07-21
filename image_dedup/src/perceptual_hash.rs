use anyhow::Result;
use image::DynamicImage;
use img_hash::{HasherConfig, ImageHash};

pub struct PerceptualHasher {
    config: HasherConfig,
}

impl PerceptualHasher {
    pub fn new() -> Self {
        Self {
            config: HasherConfig::new()
                .hash_size(8, 8)
                .hash_alg(img_hash::HashAlg::Mean)
                .preproc_dct(),
        }
    }

    pub fn new_with_size(width: u32, height: u32) -> Self {
        Self {
            config: HasherConfig::new()
                .hash_size(width, height)
                .hash_alg(img_hash::HashAlg::Mean)
                .preproc_dct(),
        }
    }

    pub fn generate_hash(&self, image: &DynamicImage) -> Result<ImageHash> {
        let hash = self.config.to_hasher().hash_image(image);
        Ok(hash)
    }

    pub fn calculate_distance(hash1: &ImageHash, hash2: &ImageHash) -> u32 {
        hash1.dist(hash2)
    }

    pub fn are_similar(hash1: &ImageHash, hash2: &ImageHash, threshold: u32) -> bool {
        Self::calculate_distance(hash1, hash2) <= threshold
    }
}

impl Default for PerceptualHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    #[test]
    fn test_generate_hash() {
        let hasher = PerceptualHasher::new();
        let image = DynamicImage::ImageRgb8(RgbImage::new(100, 100));

        let hash = hasher.generate_hash(&image).unwrap();
        assert!(!hash.to_base64().is_empty());
    }

    #[test]
    fn test_similar_images() {
        let hasher = PerceptualHasher::new();

        let image1 = DynamicImage::ImageRgb8(RgbImage::new(100, 100));
        let mut image2 = RgbImage::new(100, 100);

        for pixel in image2.pixels_mut() {
            *pixel = image::Rgb([10, 10, 10]);
        }
        let image2 = DynamicImage::ImageRgb8(image2);

        let hash1 = hasher.generate_hash(&image1).unwrap();
        let hash2 = hasher.generate_hash(&image2).unwrap();

        let distance = PerceptualHasher::calculate_distance(&hash1, &hash2);
        assert!(distance > 0);

        assert!(PerceptualHasher::are_similar(&hash1, &hash1, 0));
    }
}
