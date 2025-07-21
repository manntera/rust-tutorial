use anyhow::Result;
use image_dedup::App;

// 具体的な実装をインポート
use image_dedup::image_loader::standard::StandardImageLoader;
use image_dedup::perceptual_hash::dct_hash::DCTHasher;
use image_dedup::storage::local::LocalStorageBackend;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 依存関係をインスタンス化
    let image_loader = StandardImageLoader::new();
    let perceptual_hasher = DCTHasher::new(8); // 8x8のハッシュサイズ
    let storage = LocalStorageBackend::new();

    // 2. App構造体に依存性を注入（DI）
    let app = App::new(image_loader, perceptual_hasher, storage);

    // 3. アプリケーションを実行
    // カレントディレクトリを対象とする
    app.run(".").await?;

    Ok(())
}
