// エンドツーエンド統合テスト
use image_dedup::{
    engine::ProcessingEngine,
    image_loader::standard::StandardImageLoader,
    perceptual_hash::{dct_hash::DCTHasher, average_hash::AverageHasher},
    storage::local::LocalStorageBackend,
    services::{DefaultProcessingConfig, ConsoleProgressReporter, StreamingJsonHashPersistence},
    cli::commands::scan::execute_scan,
};
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;
use serde_json::Value;

// テスト用の有効な1x1 PNGファイル
const MINIMAL_PNG_DATA: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

// 異なる色の2x2 PNGファイル（テスト用）
const DIFFERENT_PNG_DATA: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x72, 0xB6, 0x0D, 0x24, 0x00, 0x00, 0x00,
    0x12, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xFF, 0xFF, 0xFF, 0x7F,
    0x00, 0x00, 0x00, 0x10, 0x00, 0x01, 0x43, 0x88, 0x4A, 0x75, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// テスト環境をセットアップ：複数の画像ファイルを含むディレクトリを作成
fn setup_test_images(base_dir: &std::path::Path) {
    // サブディレクトリを作成
    let subdir1 = base_dir.join("subdir1");
    let subdir2 = base_dir.join("subdir2");
    fs::create_dir_all(&subdir1).unwrap();
    fs::create_dir_all(&subdir2).unwrap();

    // 様々な画像ファイルを作成
    fs::write(base_dir.join("image1.png"), MINIMAL_PNG_DATA).unwrap();
    fs::write(base_dir.join("image2.png"), MINIMAL_PNG_DATA).unwrap(); // 重複
    fs::write(base_dir.join("image3.png"), DIFFERENT_PNG_DATA).unwrap();
    fs::write(subdir1.join("nested1.png"), MINIMAL_PNG_DATA).unwrap(); // 重複
    fs::write(subdir2.join("nested2.png"), DIFFERENT_PNG_DATA).unwrap();
    fs::write(subdir2.join("nested3.png"), MINIMAL_PNG_DATA).unwrap(); // 重複

    // 非画像ファイルも含める
    fs::write(base_dir.join("readme.txt"), "This is a text file").unwrap();
    fs::write(subdir1.join("config.json"), r#"{"test": true}"#).unwrap();

    // 破損した画像ファイル
    fs::write(base_dir.join("corrupted.png"), "NOT_A_PNG").unwrap();
}

#[tokio::test]
async fn test_full_directory_scan_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("results.json");
    
    // テスト画像をセットアップ
    setup_test_images(temp_dir.path());

    // scanコマンドを実行
    let result = execute_scan(
        temp_dir.path().to_path_buf(),
        output_file.clone(),
        Some(2), // 2スレッド使用
        false,
    ).await;

    assert!(result.is_ok());

    // 出力ファイルが作成されていることを確認
    assert!(output_file.exists());

    // 出力ファイルの内容を検証
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    let results = json.as_array().unwrap();

    // 有効な画像ファイルが5つ処理されていることを確認
    // (image1.png, image2.png, image3.png, nested1.png, nested2.png, nested3.png)
    // ただし、corrupted.pngは処理されない
    assert_eq!(results.len(), 5);

    // 各結果がfile_path、hash、metadataを含むことを確認
    for result in results {
        assert!(result.get("file_path").is_some());
        assert!(result.get("hash").is_some());
        assert!(result.get("metadata").is_some());
        
        let metadata = result.get("metadata").unwrap();
        assert!(metadata.get("file_size").is_some());
        assert!(metadata.get("processing_time_ms").is_some());
        assert!(metadata.get("image_dimensions").is_some());
        assert!(metadata.get("was_resized").is_some());
    }
}

#[tokio::test]
async fn test_duplicate_detection_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("duplicates.json");
    
    setup_test_images(temp_dir.path());

    // 処理エンジンを手動で作成してより詳細な制御
    let engine = ProcessingEngine::new(
        StandardImageLoader::with_max_dimension(256),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1)
            .with_max_concurrent(2)
            .with_batch_size(10),
        ConsoleProgressReporter::new(),
        StreamingJsonHashPersistence::new(&output_file),
    );

    let result = engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();

    // 統計を確認
    assert_eq!(result.total_files, 8); // 全ファイル数（画像・非画像・破損ファイル含む）
    assert_eq!(result.processed_files, 5); // 正常処理された画像数
    assert_eq!(result.error_count, 3); // エラーファイル数（非画像2 + 破損1）

    // 結果ファイルを解析して重複検出
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    let results = json.as_array().unwrap();

    // ハッシュ値でグループ化して重複を検出
    let mut hash_groups = std::collections::HashMap::new();
    for result in results {
        let hash = result.get("hash").unwrap().as_str().unwrap();
        let file_path = result.get("file_path").unwrap().as_str().unwrap();
        hash_groups.entry(hash.to_string())
            .or_insert_with(Vec::new)
            .push(file_path.to_string());
    }

    // 重複があることを確認（同じMINIMAL_PNG_DATAを使用した4つのファイル）
    let duplicates: Vec<_> = hash_groups
        .values()
        .filter(|files| files.len() > 1)
        .collect();
    
    assert!(!duplicates.is_empty(), "重複ファイルが検出されるはずです");
    
    // 最大の重複グループが4つのファイルを含むことを確認
    let max_duplicate_count = duplicates.iter().map(|group| group.len()).max().unwrap();
    assert_eq!(max_duplicate_count, 4);
}

#[tokio::test]
async fn test_different_hash_algorithms() {
    let temp_dir = TempDir::new().unwrap();
    let dct_output = temp_dir.path().join("dct_results.json");
    let avg_output = temp_dir.path().join("avg_results.json");
    
    setup_test_images(temp_dir.path());

    // DCTハッシュで処理
    let dct_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        ConsoleProgressReporter::new(),
        StreamingJsonHashPersistence::new(&dct_output),
    );

    let dct_result = dct_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();

    // 平均ハッシュで処理
    let avg_engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        AverageHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        ConsoleProgressReporter::new(),
        StreamingJsonHashPersistence::new(&avg_output),
    );

    let avg_result = avg_engine
        .process_directory(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();

    // 両方とも同じ数のファイルを処理
    assert_eq!(dct_result.total_files, avg_result.total_files);
    assert_eq!(dct_result.processed_files, avg_result.processed_files);
    assert_eq!(dct_result.error_count, avg_result.error_count);

    // 結果ファイルが両方作成されている
    assert!(dct_output.exists());
    assert!(avg_output.exists());

    // ハッシュ値は異なる可能性がある（アルゴリズムが違うため）
    let dct_content = fs::read_to_string(&dct_output).unwrap();
    let avg_content = fs::read_to_string(&avg_output).unwrap();
    
    let dct_json: Value = serde_json::from_str(&dct_content).unwrap();
    let avg_json: Value = serde_json::from_str(&avg_content).unwrap();
    
    // 同じ数の結果
    assert_eq!(dct_json.as_array().unwrap().len(), avg_json.as_array().unwrap().len());
}

#[tokio::test]
async fn test_force_overwrite_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("overwrite_test.json");
    
    setup_test_images(temp_dir.path());

    // 最初の実行
    let result1 = execute_scan(
        temp_dir.path().to_path_buf(),
        output_file.clone(),
        None,
        false,
    ).await;
    assert!(result1.is_ok());

    // 既存ファイルがある状態で--forceなしで実行（エラーになるべき）
    let result2 = execute_scan(
        temp_dir.path().to_path_buf(),
        output_file.clone(),
        None,
        false,
    ).await;
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("already exists"));

    // --forceありで実行（成功するべき）
    let result3 = execute_scan(
        temp_dir.path().to_path_buf(),
        output_file.clone(),
        None,
        true, // force = true
    ).await;
    assert!(result3.is_ok());
}

#[tokio::test]
async fn test_large_directory_performance() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("performance_test.json");
    
    // 多数の小さなファイルを作成
    for i in 0..50 {
        let filename = format!("test_{:03}.png", i);
        // 半分は重複、半分は異なる
        let data = if i % 2 == 0 { MINIMAL_PNG_DATA } else { DIFFERENT_PNG_DATA };
        fs::write(temp_dir.path().join(filename), data).unwrap();
    }

    let start_time = std::time::Instant::now();

    let result = execute_scan(
        temp_dir.path().to_path_buf(),
        output_file.clone(),
        Some(4), // 4スレッド使用
        false,
    ).await;

    let elapsed = start_time.elapsed();

    assert!(result.is_ok());
    
    // 結果ファイルの確認
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    let results = json.as_array().unwrap();
    assert_eq!(results.len(), 50);

    // パフォーマンス確認（50ファイルを10秒以内で処理）
    assert!(elapsed.as_secs() < 10, "処理時間が遅すぎます: {:?}", elapsed);
    
    println!("50ファイルの処理時間: {:?}", elapsed);
    println!("平均処理時間: {:.2}ms/ファイル", elapsed.as_millis() as f64 / 50.0);
}

#[tokio::test]
async fn test_nested_directory_structure() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("nested_test.json");
    
    // 深くネストした構造を作成
    let deep_path = temp_dir.path()
        .join("level1")
        .join("level2")
        .join("level3")
        .join("level4");
    fs::create_dir_all(&deep_path).unwrap();

    // 各レベルにファイルを配置
    fs::write(temp_dir.path().join("root.png"), MINIMAL_PNG_DATA).unwrap();
    fs::write(temp_dir.path().join("level1").join("l1.png"), MINIMAL_PNG_DATA).unwrap();
    fs::write(temp_dir.path().join("level1").join("level2").join("l2.png"), DIFFERENT_PNG_DATA).unwrap();
    fs::write(&deep_path.join("deep.png"), MINIMAL_PNG_DATA).unwrap();

    let result = execute_scan(
        temp_dir.path().to_path_buf(),
        output_file.clone(),
        None,
        false,
    ).await;

    assert!(result.is_ok());

    // 結果確認
    let content = fs::read_to_string(&output_file).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();
    let results = json.as_array().unwrap();
    assert_eq!(results.len(), 4);

    // 深いパスのファイルも含まれていることを確認
    let file_paths: Vec<String> = results
        .iter()
        .map(|r| r.get("file_path").unwrap().as_str().unwrap().to_string())
        .collect();
    
    assert!(file_paths.iter().any(|p| p.contains("level4")));
}