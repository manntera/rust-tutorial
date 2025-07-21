# アーキテクチャ設計

## 概要

このプロジェクトは、様々なストレージバックエンド（ローカルファイルシステム、S3、将来的には他のクラウドストレージ）に対応できるよう、抽象化レイヤーを使用した設計になっています。

## 設計の原則

1. **抽象化による柔軟性**: `StorageBackend`トレイトを使用して、ストレージの詳細を隠蔽
2. **依存性の逆転**: ビジネスロジックは具体的なストレージ実装に依存せず、抽象インターフェースに依存
3. **テスタビリティ**: モックストレージを簡単に作成できる
4. **拡張性**: 新しいストレージタイプの追加が容易

## コンポーネント構成

```
image_dedup/
├── src/
│   ├── storage/              # ストレージ抽象化層
│   │   ├── mod.rs           # トレイト定義とファクトリ
│   │   ├── local.rs         # ローカルファイルシステム実装
│   │   └── s3.rs            # S3実装（将来）
│   ├── file_scanner.rs      # レガシーAPI（互換性のため）
│   ├── image_loader.rs      # 画像読み込み
│   └── perceptual_hash.rs   # ハッシュ計算
```

## StorageBackendトレイト

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn list_items(&self, prefix: &str) -> Result<Vec<StorageItem>>;
    async fn read_item(&self, id: &str) -> Result<Vec<u8>>;
    async fn exists(&self, id: &str) -> Result<bool>;
    async fn delete_item(&self, id: &str) -> Result<()>;
    fn is_image_file(&self, item: &StorageItem) -> bool;
}
```

## 新しいストレージバックエンドの追加方法

### 1. 新しい実装を作成

```rust
// src/storage/gcs.rs
use super::{StorageBackend, StorageItem};
use async_trait::async_trait;

pub struct GcsStorageBackend {
    bucket: String,
    // client: google_cloud_storage::Client,
}

#[async_trait]
impl StorageBackend for GcsStorageBackend {
    // 実装...
}
```

### 2. StorageTypeに追加

```rust
pub enum StorageType {
    Local,
    S3 { bucket: String, region: String },
    Gcs { bucket: String },  // 新規追加
}
```

### 3. ファクトリを更新

```rust
impl StorageFactory {
    pub async fn create(storage_type: &StorageType) -> Result<Box<dyn StorageBackend>> {
        match storage_type {
            StorageType::Local => { /* ... */ },
            StorageType::S3 { /* ... */ } => { /* ... */ },
            StorageType::Gcs { bucket } => {
                Ok(Box::new(GcsStorageBackend::new(bucket)))
            }
        }
    }
}
```

## 使用例

### ローカルファイルシステム

```rust
let storage = StorageFactory::create(&StorageType::Local).await?;
let items = storage.list_items("./images").await?;
```

### S3（将来）

```rust
let storage = StorageFactory::create(&StorageType::S3 {
    bucket: "my-bucket".to_string(),
    region: "us-east-1".to_string(),
}).await?;
let items = storage.list_items("photos/").await?;
```

## 設定ベースの切り替え

環境変数や設定ファイルでストレージタイプを切り替える例：

```rust
use std::env;

fn get_storage_type() -> StorageType {
    match env::var("STORAGE_TYPE").as_deref() {
        Ok("s3") => StorageType::S3 {
            bucket: env::var("S3_BUCKET").unwrap(),
            region: env::var("S3_REGION").unwrap(),
        },
        _ => StorageType::Local,
    }
}
```

## テスト戦略

### モックストレージの作成

```rust
struct MockStorageBackend {
    items: Vec<StorageItem>,
}

#[async_trait]
impl StorageBackend for MockStorageBackend {
    async fn list_items(&self, _prefix: &str) -> Result<Vec<StorageItem>> {
        Ok(self.items.clone())
    }
    // 他のメソッドも同様に実装
}
```

## 今後の拡張予定

1. **S3対応**: AWS SDK for Rustを使用した実装
2. **Google Cloud Storage対応**: google-cloud-storageクレートを使用
3. **Azure Blob Storage対応**: azure_storageクレートを使用
4. **並列処理**: 大量のファイルを効率的に処理
5. **ストリーミング**: 大きなファイルのメモリ効率的な処理