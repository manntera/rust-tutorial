// ファイルサイズに基づく優先順位付けの統合テスト
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Debug, Deserialize, Serialize)]
struct DuplicatesReport {
    total_groups: usize,
    total_duplicates: usize,
    threshold: u32,
    groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DuplicateGroup {
    group_id: usize,
    #[serde(default)]
    original_index: usize,
    files: Vec<DuplicateFile>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DuplicateFile {
    path: String,
    hash: String,
    distance_from_first: u32,
    #[serde(default)]
    is_original: bool,
}

// 異なるサイズのPNG画像データ
const SMALL_PNG_DATA: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

// 大きいサイズのPNG画像データ（パディングを追加）
fn create_large_png_data() -> Vec<u8> {
    let mut data = SMALL_PNG_DATA.to_vec();
    // コメントチャンクを追加して画像サイズを大きくする
    let comment = b"This is a larger image with padding data to increase file size";
    data.extend_from_slice(comment);
    data
}

#[tokio::test]
async fn test_file_size_priority_end_to_end() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let target_dir = temp_dir.path().join("images");
    fs::create_dir_all(&target_dir)?;

    // 異なるサイズの類似画像を作成
    let small_file = target_dir.join("small.png");
    let large_file = target_dir.join("large.png");
    let medium_file = target_dir.join("medium.png");

    fs::write(&small_file, SMALL_PNG_DATA)?;
    fs::write(&large_file, create_large_png_data())?;
    
    let mut medium_data = SMALL_PNG_DATA.to_vec();
    medium_data.extend_from_slice(b"medium");
    fs::write(&medium_file, medium_data)?;

    // ファイルサイズを確認
    let small_size = fs::metadata(&small_file)?.len();
    let large_size = fs::metadata(&large_file)?.len();
    let medium_size = fs::metadata(&medium_file)?.len();
    
    assert!(large_size > medium_size);
    assert!(medium_size > small_size);

    // 1. Scanコマンドを実行
    let hash_db = temp_dir.path().join("hashes.json");
    image_dedup::cli::commands::execute_scan(
        target_dir.clone(),
        hash_db.clone(),
        None,
        false,
        "dct".to_string(),
        8,
        None,
    )
    .await?;

    // 2. Find-dupsコマンドを実行
    let duplicates_file = temp_dir.path().join("duplicates.json");
    image_dedup::cli::commands::execute_find_dups(
        hash_db,
        duplicates_file.clone(),
        64, // 高い閾値で全ファイルを同じグループにする
    )
    .await?;

    // 重複レポートを読み込んで検証
    let report_content = fs::read_to_string(&duplicates_file)?;
    let report: DuplicatesReport = serde_json::from_str(&report_content)?;

    assert_eq!(report.total_groups, 1, "All files should be in one group");
    assert_eq!(report.groups[0].files.len(), 3, "Group should have 3 files");

    // 最大ファイルがオリジナルとしてマークされているか確認
    let group = &report.groups[0];
    let original_file = &group.files[group.original_index];
    
    assert!(
        original_file.path.contains("large"),
        "The largest file should be marked as original"
    );
    assert!(
        original_file.is_original,
        "Original file should have is_original flag set"
    );

    // 3. Processコマンドを実行
    let moved_dir = temp_dir.path().join("duplicates");
    image_dedup::cli::commands::execute_process(
        duplicates_file,
        image_dedup::cli::ProcessAction::Move,
        moved_dir.clone(),
        true, // no_confirm
    )
    .await?;

    // 検証: large.pngが元の場所に残っているか
    assert!(
        large_file.exists(),
        "Large file (original) should still exist"
    );
    
    // 検証: small.pngとmedium.pngが移動されているか
    assert!(
        !small_file.exists(),
        "Small file should have been moved"
    );
    assert!(
        !medium_file.exists(),
        "Medium file should have been moved"
    );

    // 移動先を確認
    assert!(
        moved_dir.join("group_0").join("small.png").exists(),
        "Small file should be in moved directory"
    );
    assert!(
        moved_dir.join("group_0").join("medium.png").exists(),
        "Medium file should be in moved directory"
    );

    Ok(())
}

#[tokio::test]
async fn test_backward_compatibility_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // 旧フォーマットのハッシュデータベースを作成
    let hash_db = temp_dir.path().join("old_format.json");
    let old_format_json = r#"[
        {
            "file_path": "/tmp/image1.png",
            "hash": "abc123",
            "hash_bits": 0
        },
        {
            "file_path": "/tmp/image2.png",
            "hash": "abc124",
            "hash_bits": 1
        }
    ]"#;
    fs::write(&hash_db, old_format_json)?;

    // Find-dupsコマンドを実行
    let duplicates_file = temp_dir.path().join("duplicates.json");
    image_dedup::cli::commands::execute_find_dups(
        hash_db,
        duplicates_file.clone(),
        5,
    )
    .await?;

    // 重複レポートを確認
    let report_content = fs::read_to_string(&duplicates_file)?;
    let report: DuplicatesReport = serde_json::from_str(&report_content)?;

    // 旧フォーマットでも動作することを確認
    assert_eq!(report.total_groups, 1);
    assert_eq!(report.groups[0].files.len(), 2);
    
    // デフォルトでは最初のファイルがオリジナル
    assert_eq!(report.groups[0].original_index, 0);

    Ok(())
}