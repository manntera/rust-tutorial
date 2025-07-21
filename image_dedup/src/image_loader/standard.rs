use super::{ImageLoaderBackend, LoadResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::path::Path;
use std::time::Instant;

/// 標準的な画像ローダー実装
pub struct StandardImageLoader {
    max_dimension: Option<u32>,
}

impl Default for StandardImageLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardImageLoader {
    /// 新しい標準画像ローダーを作成
    pub fn new() -> Self {
        Self {
            max_dimension: None,
        }
    }

    /// 最大サイズ制限付きの画像ローダーを作成
    pub fn with_max_dimension(max_dimension: u32) -> Self {
        Self {
            max_dimension: Some(max_dimension),
        }
    }

    /// 必要に応じて画像をリサイズ
    fn resize_if_needed(&self, mut image: DynamicImage) -> (DynamicImage, bool) {
        if let Some(max_dim) = self.max_dimension {
            let (width, height) = (image.width(), image.height());

            if width > max_dim || height > max_dim {
                // アスペクト比を保ってリサイズ
                let ratio = (max_dim as f32) / (width.max(height) as f32);
                let new_width = (width as f32 * ratio) as u32;
                let new_height = (height as f32 * ratio) as u32;

                image = image.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

                return (image, true);
            }
        }

        (image, false)
    }
}

#[async_trait]
impl ImageLoaderBackend for StandardImageLoader {
    async fn load_from_bytes(&self, data: &[u8]) -> Result<LoadResult> {
        let start_time = Instant::now();

        let image = tokio::task::spawn_blocking({
            let data = data.to_vec();
            move || image::load_from_memory(&data)
        })
        .await
        .context("Failed to spawn blocking task for image loading")?
        .context("Failed to load image from memory")?;

        let original_dimensions = (image.width(), image.height());
        let (final_image, was_resized) = self.resize_if_needed(image);
        let load_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(LoadResult {
            image: final_image,
            original_dimensions,
            was_resized,
            load_time_ms,
        })
    }

    async fn load_from_path(&self, path: &Path) -> Result<LoadResult> {
        let start_time = Instant::now();

        let image = tokio::task::spawn_blocking({
            let path = path.to_path_buf();
            move || image::open(&path)
        })
        .await
        .context("Failed to spawn blocking task for image loading")?
        .with_context(|| format!("Failed to load image from path: {}", path.display()))?;

        let original_dimensions = (image.width(), image.height());
        let (final_image, was_resized) = self.resize_if_needed(image);
        let load_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(LoadResult {
            image: final_image,
            original_dimensions,
            was_resized,
            load_time_ms,
        })
    }

    async fn load_with_format(&self, data: &[u8], format: ImageFormat) -> Result<LoadResult> {
        let start_time = Instant::now();

        let image = tokio::task::spawn_blocking({
            let data = data.to_vec();
            move || {
                use std::io::Cursor;
                image::load(Cursor::new(data), format)
            }
        })
        .await
        .context("Failed to spawn blocking task for image loading")?
        .with_context(|| format!("Failed to load image with format: {format:?}"))?;

        let original_dimensions = (image.width(), image.height());
        let (final_image, was_resized) = self.resize_if_needed(image);
        let load_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(LoadResult {
            image: final_image,
            original_dimensions,
            was_resized,
            load_time_ms,
        })
    }

    fn strategy_name(&self) -> &'static str {
        if self.max_dimension.is_some() {
            "Standard with size limit"
        } else {
            "Standard"
        }
    }

    fn max_supported_pixels(&self) -> Option<u64> {
        self.max_dimension.map(|dim| (dim as u64) * (dim as u64))
    }

    fn estimate_memory_usage(&self, width: u32, height: u32) -> u64 {
        let actual_width = self
            .max_dimension
            .map_or(width, |max_dim| width.min(max_dim));
        let actual_height = self
            .max_dimension
            .map_or(height, |max_dim| height.min(max_dim));

        // RGBA8 + 処理用のオーバーヘッド
        (actual_width as u64) * (actual_height as u64) * 4 * 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_from_path() {
        let temp_dir = tempdir().unwrap();
        let image_path = temp_dir.path().join("test.png");

        // テスト画像を作成
        let img = image::RgbImage::new(100, 100);
        img.save(&image_path).unwrap();

        let loader = StandardImageLoader::new();
        let result = loader.load_from_path(&image_path).await.unwrap();

        assert_eq!(result.original_dimensions, (100, 100));
        assert!(!result.was_resized);
        // 読み込み時間は0msの場合もあるので削除
        assert_eq!(loader.strategy_name(), "Standard");
    }

    #[tokio::test]
    async fn test_load_with_resize() {
        let temp_dir = tempdir().unwrap();
        let image_path = temp_dir.path().join("large_test.png");

        // 大きなテスト画像を作成
        let img = image::RgbImage::new(300, 200);
        img.save(&image_path).unwrap();

        let loader = StandardImageLoader::with_max_dimension(150);
        let result = loader.load_from_path(&image_path).await.unwrap();

        assert_eq!(result.original_dimensions, (300, 200));
        assert!(result.was_resized);
        assert!(result.image.width() <= 150);
        assert!(result.image.height() <= 150);
        assert_eq!(loader.strategy_name(), "Standard with size limit");
    }

    #[tokio::test]
    async fn test_load_from_bytes() {
        let temp_dir = tempdir().unwrap();
        let image_path = temp_dir.path().join("test_bytes.png");

        // テスト画像を作成してバイト配列として読み込む
        let img = image::RgbImage::new(10, 10);
        img.save(&image_path).unwrap();

        let image_bytes = std::fs::read(&image_path).unwrap();

        let loader = StandardImageLoader::new();
        let result = loader.load_from_bytes(&image_bytes).await.unwrap();

        assert_eq!(result.original_dimensions, (10, 10));
        assert!(!result.was_resized);
        // u64は常に0以上なので比較を削除
    }
}
