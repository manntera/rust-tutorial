use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
struct HashEntry {
    file_path: String,
    hash: String,
    hash_bits: u64,
    metadata: Option<serde_json::Value>, // メタデータはオプショナル（旧フォーマット互換のため）
}

// 新しいフォーマット用の構造体
#[derive(Debug, Deserialize)]
struct ScanResult {
    images: Vec<HashEntry>,
    scan_info: serde_json::Value,
}

impl ScanResult {
    /// スキャン情報の統計を取得
    fn validate_scan_info(&self) -> bool {
        // scan_infoが有効なJSON構造を持っているかチェック
        self.scan_info.is_object()
    }
}

// 旧フォーマット互換用の構造体
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HashDatabase {
    NewFormat(ScanResult),
    OldFormat(Vec<HashEntry>),
}

#[derive(Debug, Serialize, Deserialize)]
struct DuplicateGroup {
    group_id: usize,
    original_index: usize,
    files: Vec<DuplicateFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DuplicateFile {
    path: String,
    hash: String,
    distance_from_first: u32,
    is_original: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct DuplicatesReport {
    total_groups: usize,
    total_duplicates: usize,
    threshold: u32,
    groups: Vec<DuplicateGroup>,
}

/// Calculate Hamming distance between two hash values
fn hamming_distance(hash1: u64, hash2: u64) -> u32 {
    (hash1 ^ hash2).count_ones()
}

/// Find duplicate images using hash database
pub async fn execute_find_dups(
    hash_database: PathBuf,
    output: PathBuf,
    threshold: u32,
) -> Result<()> {
    // Validate input file
    if !hash_database.exists() {
        anyhow::bail!(
            "Hash database file does not exist: {}",
            hash_database.display()
        );
    }

    println!("🔍 画像重複検出ツール - find-dupsコマンド");
    println!("📄 ハッシュデータベース: {}", hash_database.display());
    println!("📄 出力ファイル: {}", output.display());
    println!("🎯 類似度閾値: {threshold} (ハミング距離)");

    // Read hash entries from JSON file (supporting both old and new formats)
    let json_content = std::fs::read_to_string(&hash_database)?;
    let database: HashDatabase = serde_json::from_str(&json_content)?;

    let hash_entries = match database {
        HashDatabase::NewFormat(scan_result) => {
            // scan_infoの情報を表示
            if scan_result.validate_scan_info() {
                if let Some(algorithm) = scan_result.scan_info.get("algorithm") {
                    println!("🔧 ハッシュアルゴリズム: {algorithm}");
                }
                if let Some(total_files) = scan_result.scan_info.get("total_files") {
                    println!("📁 元スキャン対象ファイル数: {total_files}");
                }
            }
            scan_result.images
        }
        HashDatabase::OldFormat(entries) => {
            println!("⚠️  旧フォーマットのデータベースです");
            entries
        }
    };

    println!(
        "📊 読み込み完了: {}個のハッシュエントリ",
        hash_entries.len()
    );

    // Group similar images
    let mut groups = Vec::new();
    let mut group_id = 0;

    let mut processed = std::collections::HashSet::new();

    for i in 0..hash_entries.len() {
        if processed.contains(&i) {
            continue;
        }

        let base_entry = &hash_entries[i];
        let base_hash = base_entry.hash_bits;

        let mut group_files = vec![DuplicateFile {
            path: base_entry.file_path.clone(),
            hash: base_entry.hash.clone(),
            distance_from_first: 0,
            is_original: false, // Will be set after sorting
        }];

        processed.insert(i);

        // Find all similar images in remaining entries
        for (j, entry) in hash_entries.iter().enumerate() {
            if processed.contains(&j) {
                continue;
            }

            let distance = hamming_distance(base_hash, entry.hash_bits);
            if distance <= threshold {
                group_files.push(DuplicateFile {
                    path: entry.file_path.clone(),
                    hash: entry.hash.clone(),
                    distance_from_first: distance,
                    is_original: false,
                });
                processed.insert(j);
            }
        }

        // Only process groups with duplicates
        if group_files.len() > 1 {
            // Sort by file size (largest first) to determine the original
            let mut files_with_sizes: Vec<_> = group_files.into_iter()
                .map(|file| {
                    let file_size = hash_entries.iter()
                        .find(|e| e.file_path == file.path)
                        .and_then(|e| e.metadata.as_ref())
                        .and_then(|m| m.get("file_size"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    (file, file_size)
                })
                .collect();
            
            files_with_sizes.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by size descending
            
            // Find the index of the original (largest file)
            let original_index = 0; // After sorting, the first file is the largest
            
            // Get the hash of the new original file (largest file)
            let original_hash = hash_entries.iter()
                .find(|e| e.file_path == files_with_sizes[0].0.path)
                .map(|e| e.hash_bits)
                .unwrap_or(0);
            
            // Mark the original file and recalculate distances from the new original
            let sorted_files: Vec<DuplicateFile> = files_with_sizes.into_iter()
                .enumerate()
                .map(|(idx, (mut file, _))| {
                    file.is_original = idx == original_index;
                    
                    // Recalculate distance from the new original (largest file)
                    let file_hash = hash_entries.iter()
                        .find(|e| e.file_path == file.path)
                        .map(|e| e.hash_bits)
                        .unwrap_or(0);
                    file.distance_from_first = hamming_distance(original_hash, file_hash);
                    
                    file
                })
                .collect();
            
            let group = DuplicateGroup {
                group_id,
                original_index,
                files: sorted_files,
            };
            
            groups.push(group);
            group_id += 1;
        }
    }

    // Create report
    let total_duplicates: usize = groups.iter().map(|g| g.files.len() - 1).sum();
    let report = DuplicatesReport {
        total_groups: groups.len(),
        total_duplicates,
        threshold,
        groups,
    };

    // Create output directory if it doesn't exist
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Save report to JSON
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&output, json)?;

    println!("\n✅ 分析完了!");
    println!("📊 結果:");
    println!("   - 重複グループ数: {}", report.total_groups);
    println!("   - 重複ファイル総数: {}", report.total_duplicates);
    println!("📄 結果は {} に保存されました", output.display());

    // Display sample results
    if report.total_groups > 0 {
        println!("\n📌 重複例 (最初の3グループ):");
        for (idx, group) in report.groups.iter().take(3).enumerate() {
            println!("\n  グループ {} ({} ファイル):", idx + 1, group.files.len());
            for file in &group.files {
                println!("    - {} (距離: {})", file.path, file.distance_from_first);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_hash_entry(file_path: &str, hash: &str, hash_bits: u64) -> HashEntry {
        HashEntry {
            file_path: file_path.to_string(),
            hash: hash.to_string(),
            hash_bits,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_find_dups_new_format() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create new format database
        let new_format = r#"{
            "scan_info": {
                "algorithm": "dct",
                "timestamp": "2024-01-01T00:00:00Z",
                "total_files": 2
            },
            "images": [
                {
                    "file_path": "image1.jpg",
                    "hash": "hash1",
                    "hash_bits": 0,
                    "metadata": {"file_size": 1000}
                },
                {
                    "file_path": "image2.jpg",
                    "hash": "hash2",
                    "hash_bits": 1,
                    "metadata": {"file_size": 2000}
                }
            ]
        }"#;
        fs::write(&hash_db, new_format).unwrap();

        execute_find_dups(hash_db, output.clone(), 3).await.unwrap();

        // Check output file
        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();
        assert_eq!(report.total_groups, 1);
        assert_eq!(report.total_duplicates, 1);
    }

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(0b0000, 0b0000), 0);
        assert_eq!(hamming_distance(0b1111, 0b0000), 4);
        assert_eq!(hamming_distance(0b1010, 0b0101), 4);
        assert_eq!(hamming_distance(0b1100, 0b1010), 2);
        assert_eq!(hamming_distance(0b11111111, 0b00000000), 8);
    }

    #[tokio::test]
    async fn test_find_dups_nonexistent_database() {
        let nonexistent = PathBuf::from("nonexistent.json");
        let output = PathBuf::from("output.json");

        let result = execute_find_dups(nonexistent, output, 5).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_find_dups_empty_database() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create empty database
        fs::write(&hash_db, "[]").unwrap();

        execute_find_dups(hash_db, output.clone(), 5).await.unwrap();

        // Check output file
        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();
        assert_eq!(report.total_groups, 0);
        assert_eq!(report.total_duplicates, 0);
    }

    #[tokio::test]
    async fn test_find_dups_with_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create hash entries with some duplicates (similar hashes)
        let entries = vec![
            create_test_hash_entry("image1.jpg", "hash1", 0b0000_0000),
            create_test_hash_entry("image2.jpg", "hash2", 0b0000_0001), // distance 1 from first
            create_test_hash_entry("image3.jpg", "hash3", 0b0000_0011), // distance 2 from first
            create_test_hash_entry("image4.jpg", "hash4", 0b1111_1111), // distance 8 from first (not duplicate)
        ];

        let json = serde_json::to_string_pretty(&entries).unwrap();
        fs::write(&hash_db, json).unwrap();

        execute_find_dups(hash_db, output.clone(), 3).await.unwrap();

        // Check output file
        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();

        assert_eq!(report.total_groups, 1);
        assert_eq!(report.total_duplicates, 2); // image2 and image3 are duplicates of image1
        assert_eq!(report.threshold, 3);

        let group = &report.groups[0];
        assert_eq!(group.files.len(), 3);
        assert_eq!(group.files[0].path, "image1.jpg");
        assert_eq!(group.files[0].distance_from_first, 0);
        assert_eq!(group.files[1].distance_from_first, 1);
        assert_eq!(group.files[2].distance_from_first, 2);
    }

    #[tokio::test]
    async fn test_find_dups_no_duplicates_strict_threshold() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create hash entries with high distances
        let entries = vec![
            create_test_hash_entry("image1.jpg", "hash1", 0b0000_0000),
            create_test_hash_entry("image2.jpg", "hash2", 0b1111_0000), // distance 4
            create_test_hash_entry("image3.jpg", "hash3", 0b0000_1111), // distance 4
        ];

        let json = serde_json::to_string_pretty(&entries).unwrap();
        fs::write(&hash_db, json).unwrap();

        execute_find_dups(hash_db, output.clone(), 2).await.unwrap(); // strict threshold

        // Check output file
        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();

        assert_eq!(report.total_groups, 0);
        assert_eq!(report.total_duplicates, 0);
    }

    #[tokio::test]
    async fn test_find_dups_multiple_groups() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create hash entries with multiple groups
        let entries = vec![
            // Group 1: similar to 0b0000_0000
            create_test_hash_entry("image1.jpg", "hash1", 0b0000_0000),
            create_test_hash_entry("image2.jpg", "hash2", 0b0000_0001), // distance 1
            // Group 2: similar to 0b1111_0000
            create_test_hash_entry("image3.jpg", "hash3", 0b1111_0000),
            create_test_hash_entry("image4.jpg", "hash4", 0b1111_0001), // distance 1
            create_test_hash_entry("image5.jpg", "hash5", 0b1111_0011), // distance 2 from image3
            // Isolated image
            create_test_hash_entry("image6.jpg", "hash6", 0b1010_1010), // far from others
        ];

        let json = serde_json::to_string_pretty(&entries).unwrap();
        fs::write(&hash_db, json).unwrap();

        execute_find_dups(hash_db, output.clone(), 2).await.unwrap();

        // Check output file
        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();

        assert_eq!(report.total_groups, 2);
        assert_eq!(report.total_duplicates, 3); // (2-1) + (3-1) = 1 + 2 = 3

        // Check first group
        let group1 = &report.groups[0];
        assert_eq!(group1.files.len(), 2);
        assert_eq!(group1.files[0].path, "image1.jpg");

        // Check second group
        let group2 = &report.groups[1];
        assert_eq!(group2.files.len(), 3);
        assert_eq!(group2.files[0].path, "image3.jpg");
    }

    #[tokio::test]
    async fn test_find_dups_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create invalid JSON
        fs::write(&hash_db, "invalid json content").unwrap();

        let result = execute_find_dups(hash_db, output, 5).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_dups_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create database with single file
        let entries = vec![create_test_hash_entry("single.jpg", "hash1", 0b0000_0000)];
        let json = serde_json::to_string_pretty(&entries).unwrap();
        fs::write(&hash_db, json).unwrap();

        execute_find_dups(hash_db, output.clone(), 5).await.unwrap();

        // Check output file
        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();

        assert_eq!(report.total_groups, 0); // No groups with duplicates
        assert_eq!(report.total_duplicates, 0);
    }

    #[tokio::test]
    async fn test_find_dups_output_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let nested_output = temp_dir
            .path()
            .join("nested")
            .join("deep")
            .join("duplicates.json");

        // Create empty database
        fs::write(&hash_db, "[]").unwrap();

        let result = execute_find_dups(hash_db, nested_output.clone(), 5).await;
        assert!(result.is_ok());
        assert!(nested_output.exists());
    }

    #[test]
    fn test_duplicate_structs_serialization() {
        let file = DuplicateFile {
            path: "test.jpg".to_string(),
            hash: "abcd1234".to_string(),
            distance_from_first: 5,
            is_original: true,
        };

        let group = DuplicateGroup {
            group_id: 0,
            original_index: 0,
            files: vec![file],
        };

        let report = DuplicatesReport {
            total_groups: 1,
            total_duplicates: 0,
            threshold: 5,
            groups: vec![group],
        };

        // Test that structures can be serialized and deserialized
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: DuplicatesReport = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.total_groups, 1);
        assert_eq!(deserialized.threshold, 5);
        assert_eq!(deserialized.groups[0].group_id, 0);
        assert_eq!(deserialized.groups[0].files[0].path, "test.jpg");
    }

    #[test]
    fn test_scan_result_validation() {
        let valid_json = r#"{
            "scan_info": {
                "algorithm": "dct",
                "timestamp": "2024-01-01T00:00:00Z"
            },
            "images": []
        }"#;

        let scan_result: ScanResult = serde_json::from_str(valid_json).unwrap();
        assert!(scan_result.validate_scan_info());

        let invalid_json = r#"{
            "scan_info": "invalid_structure",
            "images": []
        }"#;

        let invalid_scan_result: ScanResult = serde_json::from_str(invalid_json).unwrap();
        assert!(!invalid_scan_result.validate_scan_info());
    }

    #[tokio::test]
    async fn test_find_dups_prioritize_by_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create hash entries with file size metadata
        let entries = vec![
            HashEntry {
                file_path: "small.jpg".to_string(),
                hash: "hash1".to_string(),
                hash_bits: 0b0000_0000,
                metadata: Some(serde_json::json!({"file_size": 1000})),
            },
            HashEntry {
                file_path: "large.jpg".to_string(),
                hash: "hash2".to_string(),
                hash_bits: 0b0000_0001, // distance 1 from small.jpg
                metadata: Some(serde_json::json!({"file_size": 5000})),
            },
            HashEntry {
                file_path: "medium.jpg".to_string(),
                hash: "hash3".to_string(),
                hash_bits: 0b0000_0011, // distance 2 from small.jpg
                metadata: Some(serde_json::json!({"file_size": 3000})),
            },
        ];

        let json = serde_json::to_string_pretty(&entries).unwrap();
        fs::write(&hash_db, json).unwrap();

        execute_find_dups(hash_db, output.clone(), 3).await.unwrap();

        // Check output file
        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();

        assert_eq!(report.total_groups, 1);
        assert_eq!(report.groups[0].files.len(), 3);
        
        // Verify that the largest file (large.jpg) is marked as original (index 0 after sorting)
        assert_eq!(report.groups[0].original_index, 0);
        assert_eq!(report.groups[0].files[0].path, "large.jpg");
        assert_eq!(report.groups[0].files[0].is_original, true);
        
        // Verify other files are marked as duplicates
        assert_eq!(report.groups[0].files[1].is_original, false);
        assert_eq!(report.groups[0].files[2].is_original, false);
    }

    #[tokio::test]
    async fn test_find_dups_fallback_when_no_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let hash_db = temp_dir.path().join("hashes.json");
        let output = temp_dir.path().join("duplicates.json");

        // Create entries without file size metadata
        let entries = vec![
            HashEntry {
                file_path: "first.jpg".to_string(),
                hash: "hash1".to_string(),
                hash_bits: 0b0000_0000,
                metadata: None,
            },
            HashEntry {
                file_path: "second.jpg".to_string(),
                hash: "hash2".to_string(),
                hash_bits: 0b0000_0001,
                metadata: None,
            },
        ];

        let json = serde_json::to_string_pretty(&entries).unwrap();
        fs::write(&hash_db, json).unwrap();

        execute_find_dups(hash_db, output.clone(), 3).await.unwrap();

        let content = fs::read_to_string(&output).unwrap();
        let report: DuplicatesReport = serde_json::from_str(&content).unwrap();

        // When no file size is available, the first file should be the original
        assert_eq!(report.groups[0].original_index, 0);
        assert_eq!(report.groups[0].files[0].is_original, true);
    }
}
