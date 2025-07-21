use anyhow::Result;
use async_trait::async_trait;
use image::DynamicImage;
use std::path::Path;

pub mod standard;
// 将来的に: pub mod gpu_accelerated;
// 将来的に: pub mod memory_optimized;

/// 画像の読み込み方法を表す列挙型
#[derive(Debug, Clone)]
pub enum ImageLoadStrategy {
    /// 標準的な画像読み込み（image crateを直接使用）
    Standard,
    /// メモリ効率を重視した読み込み（大きな画像対応）
    MemoryOptimized { max_dimension: u32 },
    /// GPU加速読み込み（将来実装予定）
    GpuAccelerated,
}

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

/// 画像ローダーファクトリ
pub struct ImageLoaderFactory;

impl ImageLoaderFactory {
    /// 指定された戦略で画像ローダーを作成
    pub async fn create(strategy: &ImageLoadStrategy) -> Result<Box<dyn ImageLoaderBackend>> {
        match strategy {
            ImageLoadStrategy::Standard => Ok(Box::new(standard::StandardImageLoader::new())),
            ImageLoadStrategy::MemoryOptimized { max_dimension } => Ok(Box::new(
                standard::StandardImageLoader::with_max_dimension(*max_dimension),
            )),
            ImageLoadStrategy::GpuAccelerated => {
                anyhow::bail!("GPU accelerated image loading is not implemented yet")
            }
        }
    }

    /// システムのメモリ状況に応じて最適な戦略を選択
    pub fn recommend_strategy(available_memory_gb: f64) -> ImageLoadStrategy {
        if available_memory_gb < 2.0 {
            ImageLoadStrategy::MemoryOptimized {
                max_dimension: 2048,
            }
        } else if available_memory_gb < 8.0 {
            ImageLoadStrategy::MemoryOptimized {
                max_dimension: 4096,
            }
        } else {
            ImageLoadStrategy::Standard
        }
    }
}

/// 旧来のAPIとの互換性のためのラッパー（非推奨）
#[deprecated(note = "Use ImageLoaderFactory and ImageLoaderBackend instead")]
pub struct ImageLoader;

#[allow(deprecated)]
impl ImageLoader {
    pub fn load_image(path: &Path) -> Result<DynamicImage> {
        let img = image::open(path)?;
        Ok(img)
    }

    pub fn get_image_dimensions(image: &DynamicImage) -> (u32, u32) {
        use image::GenericImageView;
        (image.width(), image.height())
    }
}
