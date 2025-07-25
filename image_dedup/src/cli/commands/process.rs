use crate::cli::ProcessAction;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
struct DuplicatesReport {
    total_groups: usize,
    total_duplicates: usize,
    threshold: u32,
    groups: Vec<DuplicateGroup>,
}

// Hash database structures for metadata lookup
#[derive(Debug, Deserialize)]
struct HashEntry {
    file_path: String,
    #[allow(dead_code)]
    hash: String,
    #[allow(dead_code)]
    hash_bits: u64,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ScanResult {
    images: Vec<HashEntry>,
    #[allow(dead_code)]
    scan_info: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HashDatabase {
    NewFormat(ScanResult),
    OldFormat(Vec<HashEntry>),
}

#[derive(Debug, Deserialize, Serialize)]
struct DuplicateGroup {
    group_id: usize,
    representative_file: String,
    files: Vec<DuplicateFile>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DuplicateFile {
    path: String,
    hash: String,
    distance_from_representative: u32,
}

/// Prompt user for confirmation
fn confirm_action(action: &ProcessAction, total_files: usize) -> Result<bool> {
    use std::io::{self, Write};

    print!(
        "⚠️  {} files will be {}. Continue? [y/N]: ",
        total_files,
        match action {
            ProcessAction::Move => "moved",
            ProcessAction::Delete => "PERMANENTLY DELETED",
        }
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y")
}

/// Load hash database for metadata lookup
fn load_hash_database(scan_database_path: &Path) -> Result<HashMap<String, u64>> {
    let json_content = fs::read_to_string(scan_database_path)?;
    let database: HashDatabase = serde_json::from_str(&json_content)?;
    
    let hash_entries = match database {
        HashDatabase::NewFormat(scan_result) => scan_result.images,
        HashDatabase::OldFormat(entries) => entries,
    };
    
    let mut file_sizes = HashMap::new();
    for entry in hash_entries {
        if let Some(metadata) = entry.metadata {
            if let Some(file_size) = metadata.get("file_size").and_then(|v| v.as_u64()) {
                file_sizes.insert(entry.file_path, file_size);
            }
        }
    }
    
    Ok(file_sizes)
}

/// Find the file with the largest size in a group
fn find_largest_file(group: &DuplicateGroup, file_sizes: &HashMap<String, u64>) -> String {
    group
        .files
        .iter()
        .max_by_key(|file| file_sizes.get(&file.path).unwrap_or(&0))
        .map(|file| file.path.clone())
        .unwrap_or_else(|| group.files[0].path.clone())
}

/// Process duplicate images (move or delete)
pub async fn execute_process(
    duplicate_list: PathBuf,
    action: ProcessAction,
    dest: PathBuf,
    no_confirm: bool,
) -> Result<()> {
    execute_process_with_scan_database(duplicate_list, action, dest, no_confirm, None).await
}

/// Process duplicate images with optional scan database for file size lookup
pub async fn execute_process_with_scan_database(
    duplicate_list: PathBuf,
    action: ProcessAction,
    dest: PathBuf,
    no_confirm: bool,
    scan_database: Option<PathBuf>,
) -> Result<()> {
    // Validate input file
    if !duplicate_list.exists() {
        anyhow::bail!(
            "Duplicate list file does not exist: {}",
            duplicate_list.display()
        );
    }

    println!("🔧 画像重複検出ツール - processコマンド");
    println!("📄 重複リストファイル: {}", duplicate_list.display());
    println!("🎯 アクション: {action:?}");
    if matches!(action, ProcessAction::Move) {
        println!("📁 移動先ディレクトリ: {}", dest.display());
    }

    // Read duplicates report
    let json_content = fs::read_to_string(&duplicate_list)?;
    let report: DuplicatesReport = serde_json::from_str(&json_content)?;

    if report.total_groups == 0 {
        println!("✅ 処理する重複ファイルがありません。");
        return Ok(());
    }

    // Load file sizes from scan database if available
    let file_sizes = if let Some(scan_db_path) = &scan_database {
        match load_hash_database(scan_db_path) {
            Ok(sizes) => {
                println!("📊 スキャンデータベースからファイルサイズ情報を読み込みました");
                sizes
            }
            Err(e) => {
                println!("⚠️  スキャンデータベースの読み込みに失敗: {e}");
                println!("   各グループの代表ファイル（最初に見つかったファイル）を保持します");
                HashMap::new()
            }
        }
    } else {
        println!("📊 ファイルサイズ情報なし - 各グループの代表ファイルを保持します");
        HashMap::new()
    };

    println!("\n📊 重複情報:");
    println!("   - グループ数: {}", report.total_groups);
    println!("   - 重複ファイル総数: {}", report.total_duplicates);

    // Determine which files to keep and which to process
    let files_to_process: Vec<(usize, &DuplicateFile, String)> = report
        .groups
        .iter()
        .flat_map(|group| {
            let file_to_keep = if file_sizes.is_empty() {
                // No file size info - use representative file or first in group if not set
                if !group.representative_file.is_empty() {
                    group.representative_file.clone()
                } else {
                    group.files[0].path.clone()
                }
            } else {
                // Use file size info to find largest file
                find_largest_file(group, &file_sizes)
            };

            let file_to_keep_clone = file_to_keep.clone();
            group
                .files
                .iter()
                .filter(move |file| file.path != file_to_keep)
                .map(move |file| (group.group_id, file, file_to_keep_clone.clone()))
        })
        .collect();

    println!(
        "   - 処理対象ファイル数: {} (各グループで最大サイズのファイルを保持)",
        files_to_process.len()
    );

    // Confirm action
    if !no_confirm && !confirm_action(&action, files_to_process.len())? {
        println!("❌ 処理をキャンセルしました。");
        return Ok(());
    }

    // Create destination directory if moving
    if matches!(action, ProcessAction::Move) {
        fs::create_dir_all(&dest)?;
    }

    // Process files
    let mut success_count = 0;
    let mut error_count = 0;

    for (group_id, file, _file_to_keep) in files_to_process {
        let source_path = Path::new(&file.path);

        match &action {
            ProcessAction::Move => {
                let filename = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
                let dest_subdir = dest.join(format!("group_{group_id}"));
                fs::create_dir_all(&dest_subdir)?;
                let dest_path = dest_subdir.join(filename);

                match fs::rename(source_path, &dest_path) {
                    Ok(_) => {
                        println!(
                            "✓ 移動: {} → {}",
                            source_path.display(),
                            dest_path.display()
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        eprintln!("✗ エラー: {} - {}", source_path.display(), e);
                        error_count += 1;
                    }
                }
            }
            ProcessAction::Delete => match fs::remove_file(source_path) {
                Ok(_) => {
                    println!("✓ 削除: {}", source_path.display());
                    success_count += 1;
                }
                Err(e) => {
                    eprintln!("✗ エラー: {} - {}", source_path.display(), e);
                    error_count += 1;
                }
            },
        }
    }

    println!("\n✅ 処理完了!");
    println!("📊 結果:");
    println!("   - 成功: {success_count} ファイル");
    if error_count > 0 {
        println!("   - エラー: {error_count} ファイル");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_duplicate_report(
        groups: Vec<DuplicateGroup>,
    ) -> Result<String, serde_json::Error> {
        let report = DuplicatesReport {
            total_groups: groups.len(),
            total_duplicates: groups.iter().map(|g| g.files.len().saturating_sub(1)).sum(),
            threshold: 5,
            groups,
        };
        serde_json::to_string_pretty(&report)
    }

    #[tokio::test]
    async fn test_process_nonexistent_duplicate_list() {
        let nonexistent = PathBuf::from("nonexistent.json");
        let dest = PathBuf::from("./duplicates");

        let result = execute_process(nonexistent, ProcessAction::Move, dest, true).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_process_empty_duplicate_list() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let dest = temp_dir.path().join("moved");

        // Create empty duplicates report
        let empty_report = create_test_duplicate_report(vec![]).unwrap();
        fs::write(&dup_list, empty_report).unwrap();

        let result = execute_process(dup_list, ProcessAction::Move, dest, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_move_action() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let dest = temp_dir.path().join("moved");

        // Create test files
        let file1 = temp_dir.path().join("image1.jpg");
        let file2 = temp_dir.path().join("image2.jpg");
        fs::write(&file1, "test content 1").unwrap();
        fs::write(&file2, "test content 2").unwrap();

        // Create duplicate report with these files
        let group = DuplicateGroup {
            group_id: 0,
            representative_file: file1.to_string_lossy().to_string(),
            files: vec![
                DuplicateFile {
                    path: file1.to_string_lossy().to_string(),
                    hash: "hash1".to_string(),
                    distance_from_representative: 0,
                },
                DuplicateFile {
                    path: file2.to_string_lossy().to_string(),
                    hash: "hash2".to_string(),
                    distance_from_representative: 3,
                },
            ],
        };

        let report_json = create_test_duplicate_report(vec![group]).unwrap();
        fs::write(&dup_list, report_json).unwrap();

        let result = execute_process(dup_list, ProcessAction::Move, dest.clone(), true).await;
        assert!(result.is_ok());

        // Check that first file still exists (kept)
        assert!(file1.exists());

        // Check that second file was moved
        assert!(!file2.exists());
        assert!(dest.join("group_0").join("image2.jpg").exists());
    }

    #[tokio::test]
    async fn test_process_delete_action() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");

        // Create test files
        let file1 = temp_dir.path().join("image1.jpg");
        let file2 = temp_dir.path().join("image2.jpg");
        fs::write(&file1, "test content 1").unwrap();
        fs::write(&file2, "test content 2").unwrap();

        // Create duplicate report
        let group = DuplicateGroup {
            group_id: 0,
            representative_file: file1.to_string_lossy().to_string(),
            files: vec![
                DuplicateFile {
                    path: file1.to_string_lossy().to_string(),
                    hash: "hash1".to_string(),
                    distance_from_representative: 0,
                },
                DuplicateFile {
                    path: file2.to_string_lossy().to_string(),
                    hash: "hash2".to_string(),
                    distance_from_representative: 3,
                },
            ],
        };

        let report_json = create_test_duplicate_report(vec![group]).unwrap();
        fs::write(&dup_list, report_json).unwrap();

        let result = execute_process(dup_list, ProcessAction::Delete, PathBuf::new(), true).await;
        assert!(result.is_ok());

        // Check that first file still exists (kept)
        assert!(file1.exists());

        // Check that second file was deleted
        assert!(!file2.exists());
    }

    #[tokio::test]
    async fn test_process_multiple_groups() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let dest = temp_dir.path().join("moved");

        // Create test files
        let files: Vec<PathBuf> = (1..=6)
            .map(|i| {
                let file = temp_dir.path().join(format!("image{i}.jpg"));
                fs::write(&file, format!("test content {i}")).unwrap();
                file
            })
            .collect();

        // Create multiple groups
        let groups = vec![
            DuplicateGroup {
                group_id: 0,
                representative_file: files[0].to_string_lossy().to_string(),
                files: vec![
                    DuplicateFile {
                        path: files[0].to_string_lossy().to_string(),
                        hash: "hash1".to_string(),
                        distance_from_representative: 0,
                    },
                    DuplicateFile {
                        path: files[1].to_string_lossy().to_string(),
                        hash: "hash2".to_string(),
                        distance_from_representative: 2,
                    },
                    DuplicateFile {
                        path: files[2].to_string_lossy().to_string(),
                        hash: "hash3".to_string(),
                        distance_from_representative: 3,
                    },
                ],
            },
            DuplicateGroup {
                group_id: 1,
                representative_file: files[3].to_string_lossy().to_string(),
                files: vec![
                    DuplicateFile {
                        path: files[3].to_string_lossy().to_string(),
                        hash: "hash4".to_string(),
                        distance_from_representative: 0,
                    },
                    DuplicateFile {
                        path: files[4].to_string_lossy().to_string(),
                        hash: "hash5".to_string(),
                        distance_from_representative: 1,
                    },
                ],
            },
        ];

        let report_json = create_test_duplicate_report(groups).unwrap();
        fs::write(&dup_list, report_json).unwrap();

        let result = execute_process(dup_list, ProcessAction::Move, dest.clone(), true).await;
        assert!(result.is_ok());

        // Check that first files of each group still exist
        assert!(files[0].exists()); // group 0 first file
        assert!(files[3].exists()); // group 1 first file

        // Check that other files were moved
        assert!(!files[1].exists());
        assert!(!files[2].exists());
        assert!(!files[4].exists());

        // Check moved files exist in destination
        assert!(dest.join("group_0").join("image2.jpg").exists());
        assert!(dest.join("group_0").join("image3.jpg").exists());
        assert!(dest.join("group_1").join("image5.jpg").exists());
    }

    #[tokio::test]
    async fn test_process_with_missing_source_file() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let dest = temp_dir.path().join("moved");

        // Create only one file, but reference two in the report
        let file1 = temp_dir.path().join("image1.jpg");
        let file2 = temp_dir.path().join("image2.jpg"); // This one doesn't exist
        fs::write(&file1, "test content 1").unwrap();
        // Don't create file2

        let group = DuplicateGroup {
            group_id: 0,
            representative_file: file1.to_string_lossy().to_string(),
            files: vec![
                DuplicateFile {
                    path: file1.to_string_lossy().to_string(),
                    hash: "hash1".to_string(),
                    distance_from_representative: 0,
                },
                DuplicateFile {
                    path: file2.to_string_lossy().to_string(),
                    hash: "hash2".to_string(),
                    distance_from_representative: 3,
                },
            ],
        };

        let report_json = create_test_duplicate_report(vec![group]).unwrap();
        fs::write(&dup_list, report_json).unwrap();

        let result = execute_process(dup_list, ProcessAction::Move, dest, true).await;
        // Should succeed overall but with errors for missing files
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let dest = temp_dir.path().join("moved");

        // Create invalid JSON
        fs::write(&dup_list, "invalid json content").unwrap();

        let result = execute_process(dup_list, ProcessAction::Move, dest, true).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_confirm_action_data_types() {
        // Test that ProcessAction can be formatted
        let move_action = ProcessAction::Move;
        let delete_action = ProcessAction::Delete;

        assert!(format!("{move_action:?}").contains("Move"));
        assert!(format!("{delete_action:?}").contains("Delete"));
    }

    #[tokio::test]
    async fn test_process_with_representative_file() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let dest = temp_dir.path().join("moved");

        // Create test files
        let file1 = temp_dir.path().join("small.jpg");
        let file2 = temp_dir.path().join("large.jpg");
        let file3 = temp_dir.path().join("medium.jpg");
        fs::write(&file1, "s").unwrap(); // smallest
        fs::write(&file2, "large content").unwrap(); // largest
        fs::write(&file3, "medium").unwrap(); // medium

        // Create duplicate report with representative file pointing to large.jpg
        let group = DuplicateGroup {
            group_id: 0,
            representative_file: file2.to_string_lossy().to_string(),
            files: vec![
                DuplicateFile {
                    path: file1.to_string_lossy().to_string(),
                    hash: "hash1".to_string(),
                    distance_from_representative: 0,
                },
                DuplicateFile {
                    path: file2.to_string_lossy().to_string(),
                    hash: "hash2".to_string(),
                    distance_from_representative: 1,
                },
                DuplicateFile {
                    path: file3.to_string_lossy().to_string(),
                    hash: "hash3".to_string(),
                    distance_from_representative: 2,
                },
            ],
        };

        let report_json = create_test_duplicate_report(vec![group]).unwrap();
        fs::write(&dup_list, report_json).unwrap();

        let result = execute_process(dup_list, ProcessAction::Move, dest.clone(), true).await;
        assert!(result.is_ok());

        // Check that large.jpg (the representative file) still exists
        assert!(file2.exists());

        // Check that other files were moved
        assert!(!file1.exists());
        assert!(!file3.exists());
        assert!(dest.join("group_0").join("small.jpg").exists());
        assert!(dest.join("group_0").join("medium.jpg").exists());
    }

    #[tokio::test]
    async fn test_process_with_file_size_database() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let scan_db = temp_dir.path().join("scan.json");
        let dest = temp_dir.path().join("moved");

        // Create test files with different sizes
        let file1 = temp_dir.path().join("small.jpg");
        let file2 = temp_dir.path().join("large.jpg");
        let file3 = temp_dir.path().join("medium.jpg");
        fs::write(&file1, "s").unwrap(); // smallest (1 byte)
        fs::write(&file2, "large content").unwrap(); // largest (13 bytes)
        fs::write(&file3, "medium").unwrap(); // medium (6 bytes)

        // Create scan database with file size metadata
        let scan_data = format!(
            r#"{{
            "images": [
                {{
                    "file_path": "{}",
                    "hash": "hash1",
                    "hash_bits": 12345,
                    "metadata": {{"file_size": 1}}
                }},
                {{
                    "file_path": "{}",
                    "hash": "hash2",
                    "hash_bits": 67890,
                    "metadata": {{"file_size": 13}}
                }},
                {{
                    "file_path": "{}",
                    "hash": "hash3",
                    "hash_bits": 11111,
                    "metadata": {{"file_size": 6}}
                }}
            ],
            "scan_info": {{}}
        }}"#,
            file1.to_string_lossy(),
            file2.to_string_lossy(),
            file3.to_string_lossy()
        );
        fs::write(&scan_db, scan_data).unwrap();

        // Create duplicate report with representative file pointing to small.jpg (but largest should be kept due to size info)
        let group = DuplicateGroup {
            group_id: 0,
            representative_file: file1.to_string_lossy().to_string(),
            files: vec![
                DuplicateFile {
                    path: file1.to_string_lossy().to_string(),
                    hash: "hash1".to_string(),
                    distance_from_representative: 0,
                },
                DuplicateFile {
                    path: file2.to_string_lossy().to_string(),
                    hash: "hash2".to_string(),
                    distance_from_representative: 1,
                },
                DuplicateFile {
                    path: file3.to_string_lossy().to_string(),
                    hash: "hash3".to_string(),
                    distance_from_representative: 2,
                },
            ],
        };

        let report_json = create_test_duplicate_report(vec![group]).unwrap();
        fs::write(&dup_list, report_json).unwrap();

        let result = execute_process_with_scan_database(
            dup_list,
            ProcessAction::Move,
            dest.clone(),
            true,
            Some(scan_db),
        )
        .await;
        assert!(result.is_ok());

        // With file size information, large.jpg (largest file) should be kept
        assert!(file2.exists());

        // Other files should be moved
        assert!(!file1.exists());
        assert!(!file3.exists());
        assert!(dest.join("group_0").join("small.jpg").exists());
        assert!(dest.join("group_0").join("medium.jpg").exists());
    }

    #[tokio::test]
    async fn test_process_backward_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let dup_list = temp_dir.path().join("duplicates.json");
        let dest = temp_dir.path().join("moved");

        // Create test files
        let file1 = temp_dir.path().join("first.jpg");
        let file2 = temp_dir.path().join("second.jpg");
        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        // Create old format report (without original_index and is_original)
        let report_json = format!(
            r#"{{
            "total_groups": 1,
            "total_duplicates": 1,
            "threshold": 5,
            "groups": [{{
                "group_id": 0,
                "representative_file": "{}",
                "files": [{{
                    "path": "{}",
                    "hash": "hash1",
                    "distance_from_representative": 0
                }}, {{
                    "path": "{}",
                    "hash": "hash2",
                    "distance_from_representative": 1
                }}]
            }}]
        }}"#,
            file1.to_string_lossy(),
            file1.to_string_lossy(),
            file2.to_string_lossy()
        );
        fs::write(&dup_list, report_json).unwrap();

        let result = execute_process(dup_list, ProcessAction::Move, dest.clone(), true).await;
        assert!(result.is_ok());

        // For backward compatibility, first file should be kept
        assert!(file1.exists());
        assert!(!file2.exists());
    }
}
