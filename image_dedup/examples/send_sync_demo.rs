use std::sync::Arc;
use tokio;

// Send + Syncを実装する型の例
#[derive(Debug, Clone)]
struct ThreadSafeData {
    value: i32,
}

// Send + Syncを実装しない型の例
use std::cell::RefCell;
use std::rc::Rc;

#[allow(dead_code)]
#[derive(Debug)]
struct NotThreadSafeData {
    // Rcは参照カウントがスレッドセーフでないためSendでない
    _rc_data: Rc<i32>,
    // RefCellは内部可変性がスレッドセーフでないためSyncでない
    _ref_cell: RefCell<i32>,
}

#[tokio::main]
async fn main() {
    println!("=== Send + Sync の説明 ===\n");

    // 1. Send の例 - 値を別のタスクに移動
    demonstrate_send().await;

    println!();

    // 2. Sync の例 - 参照を複数のタスクで共有
    demonstrate_sync().await;

    println!();

    // 3. StorageBackend での実際の使用例
    demonstrate_storage_usage().await;
}

// Send の例 - 値を別のタスクに移動
async fn demonstrate_send() {
    println!("1. Send の例:");

    let data = ThreadSafeData { value: 42 };

    // データを別のタスクに移動（move）
    let task = tokio::spawn(async move {
        println!("  別のタスクで実行: {:?}", data);
        data.value * 2
    });

    let result = task.await.unwrap();
    println!("  タスクの結果: {}", result);
}

// Sync の例 - 参照を複数のタスクで共有
async fn demonstrate_sync() {
    println!("2. Sync の例:");

    let data = Arc::new(ThreadSafeData { value: 100 });

    // 複数のタスクで同じデータの参照を共有
    let data1 = Arc::clone(&data);
    let data2 = Arc::clone(&data);

    let task1 = tokio::spawn(async move {
        println!("  タスク1: {:?}", data1);
        data1.value + 10
    });

    let task2 = tokio::spawn(async move {
        println!("  タスク2: {:?}", data2);
        data2.value + 20
    });

    let (result1, result2) = tokio::join!(task1, task2);
    println!("  タスク1の結果: {}", result1.unwrap());
    println!("  タスク2の結果: {}", result2.unwrap());
}

// StorageBackend での実際の使用例
async fn demonstrate_storage_usage() {
    use image_dedup::storage::{StorageFactory, StorageType};

    println!("3. StorageBackend での使用例:");

    // StorageBackendはSend + Syncなので以下が可能
    let storage = StorageFactory::create(&StorageType::Local).await.unwrap();
    let shared_storage = Arc::new(storage);

    // 複数のタスクで同じストレージを使用
    let storage1 = Arc::clone(&shared_storage);
    let storage2 = Arc::clone(&shared_storage);

    let task1 = tokio::spawn(async move {
        // 別のタスクでストレージを使用（Send）
        let items = storage1.list_items("./test_images").await.unwrap();
        items.len()
    });

    let task2 = tokio::spawn(async move {
        // 同時に別のタスクでも使用（Sync）
        let exists = storage2.exists("./test_images/sample1.jpg").await.unwrap();
        if exists { 1 } else { 0 }
    });

    let (count, exists_flag) = tokio::join!(task1, task2);
    println!("  タスク1: {} 個のファイルを発見", count.unwrap());
    println!(
        "  タスク2: ファイル存在確認 = {}",
        exists_flag.unwrap() == 1
    );
}

// コンパイルエラーの例（コメントアウト）
#[allow(dead_code)]
fn demonstrate_compile_errors() {
    // これらのコードはコンパイルエラーになる例

    /*
    // Send でない型を別スレッドに送ろうとする例
    let rc_data = std::rc::Rc::new(42);
    std::thread::spawn(move || {
        println!("{}", rc_data); // エラー: Rc<i32> is not Send
    });

    // Sync でない型の参照を別スレッドで共有しようとする例
    let cell_data = std::cell::RefCell::new(42);
    let cell_ref = &cell_data;
    std::thread::spawn(move || {
        println!("{:?}", cell_ref); // エラー: RefCell<i32> is not Sync
    });
    */

    println!("上記のコードはコンパイルエラーになる例です（コメントアウト済み）");
}
