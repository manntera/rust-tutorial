use anyhow::Result;
use async_trait::async_trait;
use image::DynamicImage;
use mockall::automock;
use std::path::Path;

pub mod standard;

/// 画像読み込みの結果情報
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// 読み込まれた画像
    pub image: DynamicImage,
    /// 元の画像サイズ
    pub original_dimensions: (u32, u32),
    /// 読み込み時にリサイズされたかどうか
    pub was_resized: bool,
    /// 読み込みにかかった時間（ミリ秒）
    pub load_time_ms: u64,
}

/// 画像読み込みバックエンドのトレイト
#[automock]
#[async_trait]
pub trait ImageLoaderBackend: Send + Sync {
    /// バイト配列から画像を読み込む
    async fn load_from_bytes(&self, data: &[u8]) -> Result<LoadResult>;

    /// ファイルパスから画像を読み込む
    async fn load_from_path(&self, path: &Path) -> Result<LoadResult>;

    /// 画像フォーマットを指定して読み込む
    async fn load_with_format(&self, data: &[u8], format: image::ImageFormat)
    -> Result<LoadResult>;

    /// 読み込み戦略の名前を取得
    fn strategy_name(&self) -> &'static str;

    /// サポートする最大画像サイズを取得（ピクセル数）
    fn max_supported_pixels(&self) -> Option<u64> {
        None // デフォルトは制限なし
    }

    /// メモリ使用量を推定（バイト）
    fn estimate_memory_usage(&self, width: u32, height: u32) -> u64 {
        // RGBA8の場合の基本計算
        width as u64 * height as u64 * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_result_creation() {
        use image::{DynamicImage, RgbImage};

        let img = RgbImage::new(100, 100);
        let dynamic_img = DynamicImage::ImageRgb8(img);

        let result = LoadResult {
            image: dynamic_img.clone(),
            original_dimensions: (200, 150),
            was_resized: true,
            load_time_ms: 50,
        };

        assert_eq!(result.original_dimensions, (200, 150));
        assert!(result.was_resized);
        assert_eq!(result.load_time_ms, 50);
        assert_eq!(result.image.width(), 100);
        assert_eq!(result.image.height(), 100);
    }

    #[test]
    fn test_load_result_debug() {
        use image::{DynamicImage, RgbImage};

        let img = RgbImage::new(50, 50);
        let dynamic_img = DynamicImage::ImageRgb8(img);

        let result = LoadResult {
            image: dynamic_img,
            original_dimensions: (100, 100),
            was_resized: false,
            load_time_ms: 25,
        };

        let debug_str = format!("{result:?}");
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("25"));
    }

    #[test]
    fn test_load_result_clone() {
        use image::{DynamicImage, RgbImage};

        let img = RgbImage::new(25, 25);
        let dynamic_img = DynamicImage::ImageRgb8(img);

        let original = LoadResult {
            image: dynamic_img,
            original_dimensions: (50, 50),
            was_resized: true,
            load_time_ms: 10,
        };

        let cloned = original.clone();
        assert_eq!(cloned.original_dimensions, original.original_dimensions);
        assert_eq!(cloned.was_resized, original.was_resized);
        assert_eq!(cloned.load_time_ms, original.load_time_ms);
        assert_eq!(cloned.image.width(), original.image.width());
        assert_eq!(cloned.image.height(), original.image.height());
    }

    #[tokio::test]
    async fn test_mock_image_loader_backend() {
        use image::{DynamicImage, RgbImage};

        let mut mock_loader = MockImageLoaderBackend::new();

        let img = RgbImage::new(32, 32);
        let dynamic_img = DynamicImage::ImageRgb8(img);

        let expected_result = LoadResult {
            image: dynamic_img,
            original_dimensions: (64, 64),
            was_resized: true,
            load_time_ms: 15,
        };

        mock_loader
            .expect_load_from_bytes()
            .with(mockall::predicate::eq(&b"test_data"[..]))
            .times(1)
            .returning(move |_| Ok(expected_result.clone()));

        mock_loader
            .expect_strategy_name()
            .return_const("mock_strategy");

        mock_loader
            .expect_max_supported_pixels()
            .return_const(Some(1000000u64));

        // Test the mock
        let result = mock_loader.load_from_bytes(b"test_data").await.unwrap();
        assert_eq!(result.original_dimensions, (64, 64));
        assert!(result.was_resized);
        assert_eq!(result.load_time_ms, 15);

        assert_eq!(mock_loader.strategy_name(), "mock_strategy");
        assert_eq!(mock_loader.max_supported_pixels(), Some(1000000u64));
    }

    #[test]
    fn test_default_memory_estimation() {
        struct TestLoader;

        #[async_trait]
        impl ImageLoaderBackend for TestLoader {
            async fn load_from_bytes(&self, _data: &[u8]) -> Result<LoadResult> {
                unimplemented!()
            }

            async fn load_from_path(&self, _path: &std::path::Path) -> Result<LoadResult> {
                unimplemented!()
            }

            async fn load_with_format(
                &self,
                _data: &[u8],
                _format: image::ImageFormat,
            ) -> Result<LoadResult> {
                unimplemented!()
            }

            fn strategy_name(&self) -> &'static str {
                "test"
            }
        }

        let loader = TestLoader;

        // Test default memory estimation (RGBA8: width * height * 4)
        assert_eq!(loader.estimate_memory_usage(100, 100), 40000); // 100 * 100 * 4
        assert_eq!(loader.estimate_memory_usage(1920, 1080), 8294400); // 1920 * 1080 * 4

        // Test default max_supported_pixels (None)
        assert_eq!(loader.max_supported_pixels(), None);
    }

    #[test]
    fn test_large_image_memory_calculation() {
        struct TestLoader;

        #[async_trait]
        impl ImageLoaderBackend for TestLoader {
            async fn load_from_bytes(&self, _data: &[u8]) -> Result<LoadResult> {
                unimplemented!()
            }

            async fn load_from_path(&self, _path: &std::path::Path) -> Result<LoadResult> {
                unimplemented!()
            }

            async fn load_with_format(
                &self,
                _data: &[u8],
                _format: image::ImageFormat,
            ) -> Result<LoadResult> {
                unimplemented!()
            }

            fn strategy_name(&self) -> &'static str {
                "test"
            }

            fn estimate_memory_usage(&self, width: u32, height: u32) -> u64 {
                // Custom implementation for testing
                width as u64 * height as u64 * 8 // Assume 8 bytes per pixel
            }
        }

        let loader = TestLoader;
        assert_eq!(loader.estimate_memory_usage(1000, 1000), 8000000); // 1000 * 1000 * 8
    }
}
