use anyhow::Result;
use image_dedup::storage::{StorageBackend, StorageFactory, StorageType};
use image_dedup::{ImageLoader, PerceptualHasher};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ストレージ抽象化のデモ ===\n");

    // ローカルストレージを使用
    let storage_type = StorageType::Local;
    let storage = StorageFactory::create(&storage_type).await?;

    println!("使用するストレージ: {:?}\n", storage_type);

    // テストディレクトリをスキャン
    let test_dir = "./test_images";
    println!("スキャン対象: {}", test_dir);

    let items = storage.list_items(test_dir).await?;
    println!("見つかった画像: {} 個\n", items.len());

    // 各画像の情報を表示
    for (i, item) in items.iter().enumerate() {
        println!("{}. {}", i + 1, item.name);
        println!("   ID: {}", item.id);
        println!("   サイズ: {} bytes", item.size);
        println!("   拡張子: {:?}", item.extension);
    }

    // 最初の画像に対してハッシュを計算（デモ）
    if let Some(first_item) = items.first() {
        println!("\n最初の画像のハッシュを計算中...");

        // ストレージからデータを読み込み
        let image_data = storage.read_item(&first_item.id).await?;

        // 画像をロード
        let image = image::load_from_memory(&image_data)?;

        // ハッシュを計算
        let hasher = PerceptualHasher::new();
        let hash = hasher.generate_hash(&image)?;

        println!("ハッシュ値: {}", hash.to_base64());
    }

    // 将来的なS3対応の例（コメントアウト）
    demonstrate_future_s3_usage().await;

    Ok(())
}

async fn demonstrate_future_s3_usage() {
    println!("\n=== 将来的なS3対応の例 ===");
    println!("```rust");
    println!("// S3ストレージを使用する場合（将来実装予定）");
    println!("let s3_storage_type = StorageType::S3 {{");
    println!("    bucket: \"my-image-bucket\".to_string(),");
    println!("    region: \"ap-northeast-1\".to_string(),");
    println!("}};");
    println!("let s3_storage = StorageFactory::create(&s3_storage_type).await?;");
    println!("");
    println!("// 使い方は同じ - インターフェースが統一されている");
    println!("let items = s3_storage.list_items(\"images/\").await?;");
    println!("let data = s3_storage.read_item(\"images/photo.jpg\").await?;");
    println!("```");

    println!("\n現在のアーキテクチャの利点:");
    println!("1. ストレージバックエンドの切り替えが簡単");
    println!("2. テスト時にモックを使用可能");
    println!("3. 新しいストレージタイプの追加が容易");
    println!("4. ビジネスロジックとストレージ層が分離");
}

// 将来的なS3実装の骨組み（参考）
#[allow(dead_code)]
mod future_s3_implementation {
    use anyhow::Result;
    use async_trait::async_trait;
    use image_dedup::storage::{StorageBackend, StorageItem};

    pub struct S3StorageBackend {
        bucket: String,
        region: String,
        // client: aws_sdk_s3::Client,
    }

    #[async_trait]
    impl StorageBackend for S3StorageBackend {
        async fn list_items(&self, prefix: &str) -> Result<Vec<StorageItem>> {
            // S3 ListObjectsV2 APIを使用
            todo!("S3実装")
        }

        async fn read_item(&self, id: &str) -> Result<Vec<u8>> {
            // S3 GetObject APIを使用
            todo!("S3実装")
        }

        async fn exists(&self, id: &str) -> Result<bool> {
            // S3 HeadObject APIを使用
            todo!("S3実装")
        }

        async fn delete_item(&self, id: &str) -> Result<()> {
            // S3 DeleteObject APIを使用
            todo!("S3実装")
        }
    }
}
