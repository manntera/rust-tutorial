use anyhow::Result;
use async_trait::async_trait;
use image::DynamicImage;
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
