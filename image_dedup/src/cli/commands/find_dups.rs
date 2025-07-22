use anyhow::Result;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct HashEntry {
    file_path: String,
    hash: String,
    #[allow(dead_code)]
    algorithm: String,
    hash_bits: u64,
    #[allow(dead_code)]
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DuplicateGroup {
    group_id: usize,
    files: Vec<DuplicateFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DuplicateFile {
    path: String,
    hash: String,
    distance_from_first: u32,
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
        anyhow::bail!("Hash database file does not exist: {}", hash_database.display());
    }
    
    println!("🔍 画像重複検出ツール - find-dupsコマンド");
    println!("📄 ハッシュデータベース: {}", hash_database.display());
    println!("📄 出力ファイル: {}", output.display());
    println!("🎯 類似度閾値: {threshold} (ハミング距離)");
    
    // Read hash entries from JSON file
    let json_content = std::fs::read_to_string(&hash_database)?;
    let hash_entries: Vec<HashEntry> = serde_json::from_str(&json_content)?;
    
    println!("📊 読み込み完了: {}個のハッシュエントリ", hash_entries.len());
    
    // Group similar images
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut processed: Vec<bool> = vec![false; hash_entries.len()];
    let mut group_id = 0;
    
    for i in 0..hash_entries.len() {
        if processed[i] {
            continue;
        }
        
        let mut group = DuplicateGroup {
            group_id,
            files: vec![DuplicateFile {
                path: hash_entries[i].file_path.clone(),
                hash: hash_entries[i].hash.clone(),
                distance_from_first: 0,
            }],
        };
        
        processed[i] = true;
        let base_hash = hash_entries[i].hash_bits;
        
        // Find all similar images
        for j in (i + 1)..hash_entries.len() {
            if processed[j] {
                continue;
            }
            
            let distance = hamming_distance(base_hash, hash_entries[j].hash_bits);
            if distance <= threshold {
                group.files.push(DuplicateFile {
                    path: hash_entries[j].file_path.clone(),
                    hash: hash_entries[j].hash.clone(),
                    distance_from_first: distance,
                });
                processed[j] = true;
            }
        }
        
        // Only add groups with duplicates
        if group.files.len() > 1 {
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
    use tempfile::TempDir;
    use std::fs;

    fn create_test_hash_entry(file_path: &str, hash: &str, hash_bits: u64) -> HashEntry {
        HashEntry {
            file_path: file_path.to_string(),
            hash: hash.to_string(),
            algorithm: "DCT".to_string(),
            hash_bits,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        }
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
        let nested_output = temp_dir.path().join("nested").join("deep").join("duplicates.json");

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
        };

        let group = DuplicateGroup {
            group_id: 0,
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
}