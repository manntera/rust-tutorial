use anyhow::Result;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
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

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(0b0000, 0b0000), 0);
        assert_eq!(hamming_distance(0b1111, 0b0000), 4);
        assert_eq!(hamming_distance(0b1010, 0b0101), 4);
        assert_eq!(hamming_distance(0b1100, 0b1010), 2);
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
}