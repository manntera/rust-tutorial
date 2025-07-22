use image_dedup::storage::{StorageBackend, local::LocalStorageBackend};
use std::sync::Arc;

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
        println!("  別のタスクで実行: {data:?}");
        data.value * 2
    });

    let result = task.await.unwrap();
    println!("  タスクの結果: {result}");
}

// Sync の例 - 参照を複数のタスクで共有
async fn demonstrate_sync() {
    println!("2. Sync の例:");

    let data = Arc::new(ThreadSafeData { value: 100 });

    // 複数のタスクで同じデータの参照を共有
    let data1 = Arc::clone(&data);
    let data2 = Arc::clone(&data);

    let task1 = tokio::spawn(async move {
        println!("  タスク1: {data1:?}");
        data1.value + 10
    });

    let task2 = tokio::spawn(async move {
        println!("  タスク2: {data2:?}");
        data2.value + 20
    });

    let (result1, result2) = tokio::join!(task1, task2);
    let result1_value = result1.unwrap();
    println!("  タスク1の結果: {result1_value}");
    let result2_value = result2.unwrap();
    println!("  タスク2の結果: {result2_value}");
}

// StorageBackend での実際の使用例
async fn demonstrate_storage_usage() {
    println!("3. StorageBackend での使用例:");

    // StorageBackendはSend + Syncなので以下が可能
    let storage: Box<dyn StorageBackend> = Box::new(LocalStorageBackend::new());
    let shared_storage = Arc::new(storage);

    // 複数のタスクで同じストレージを使用
    let storage1 = Arc::clone(&shared_storage);
    let storage2 = Arc::clone(&shared_storage);

    let task1 = tokio::spawn(async move {
        // 別のタスクでストレージを使用（Send）
        let items = storage1.list_items(".").await.unwrap();
        items.len()
    });

    let task2 = tokio::spawn(async move {
        // 同時に別のタスクでも使用（Sync）
        let exists = storage2.exists("Cargo.toml").await.unwrap();
        if exists { 1 } else { 0 }
    });

    let (count, exists_flag) = tokio::join!(task1, task2);
    let count_value = count.unwrap();
    println!("  タスク1: {count_value} 個のファイルを発見");
    println!(
        "  タスク2: ファイル存在確認 = {}",
        exists_flag.unwrap() == 1
    );
}
