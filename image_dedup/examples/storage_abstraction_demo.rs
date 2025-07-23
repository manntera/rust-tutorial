use anyhow::Result;
use image_dedup::perceptual_hash::{PerceptualHashBackend, dct_hash::DctHasher};
use image_dedup::storage::{StorageBackend, local::LocalStorageBackend};
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let test_dir = "./test_images_storage_demo";
    // デモ用の画像とディレクトリを準備
    setup_test_environment(test_dir)?;

    println!("=== ストレージ抽象化のデモ ===\n");

    // ローカルストレージを直接インスタンス化
    let storage = LocalStorageBackend::new();

    println!("使用するストレージ: LocalStorage\n");

    // テストディレクトリをスキャン
    println!("スキャン対象: {test_dir}");

    let items: Vec<_> = storage
        .list_items(test_dir)
        .await?
        .into_iter()
        .filter(|item| storage.is_image_file(item))
        .collect();
    let items_count = items.len();
    println!("見つかった画像: {items_count} 個\n");

    // 各画像の情報を表示
    for (i, item) in items.iter().enumerate() {
        let item_num = i + 1;
        let item_name = &item.name;
        println!("{item_num}. {item_name}");
        let item_id = &item.id;
        println!("   ID: {item_id}");
        let item_size = item.size;
        println!("   サイズ: {item_size} bytes");
        println!("   拡張子: {:?}", item.extension);
    }

    // 最初の画像に対してハッシュを計算（デモ）
    if let Some(first_item) = items.first() {
        println!("\n最初の画像のハッシュを計算中...");

        // ストレージからデータを読み込み
        let image_data: Vec<u8> = storage.read_item(&first_item.id).await?;

        // 画像をロード
        let image = image::load_from_memory(&image_data)?;

        // ハッシュを計算
        let hasher = DctHasher::new(8);
        let hash = hasher.generate_hash(&image).await?;

        let hash_value = hash.to_base64();
        println!("ハッシュ値: {hash_value}");
    }

    // クリーンアップ
    fs::remove_dir_all(test_dir)?;
    println!("\nテストディレクトリをクリーンアップしました。");

    Ok(())
}

fn setup_test_environment(dir: &str) -> Result<()> {
    // ディレクトリが存在する場合は一度削除して作り直す
    if Path::new(dir).exists() {
        fs::remove_dir_all(dir)?;
    }
    fs::create_dir_all(dir)?;

    // 簡単なPNG画像を生成
    use image::{ImageBuffer, RgbImage};
    let img: RgbImage = ImageBuffer::from_fn(10, 10, |x, y| {
        if (x + y) % 2 == 0 {
            image::Rgb([0, 0, 0])
        } else {
            image::Rgb([255, 255, 255])
        }
    });

    img.save(Path::new(dir).join("sample_image.png"))?;

    // 画像ではないファイルも作成
    fs::write(Path::new(dir).join("notes.txt"), "This is not an image.")?;

    println!("テスト用のディレクトリと画像を作成しました: {dir}");
    Ok(())
}
