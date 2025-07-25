use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DuplicateGroup {
    group_id: usize,
    representative_file: String,
    files: Vec<DuplicateFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DuplicateFile {
    path: String,
    hash: String,
    #[serde(alias = "distance_from_first")]
    distance_from_representative: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct DuplicatesReport {
    total_groups: usize,
    total_duplicates: usize,
    threshold: u32,
    groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FilteredReport {
    original_threshold: u32,
    filter_threshold: u32,
    filtered_groups: usize,
    filtered_duplicates: usize,
    groups: Vec<DuplicateGroup>,
}

/// Filter duplicate groups by minimum hash distance
pub async fn execute_filter_duplicates(input_json: PathBuf, min_distance: u32) -> Result<()> {
    // Validate input file
    if !input_json.exists() {
        anyhow::bail!("Input JSON file does not exist: {}", input_json.display());
    }

    println!("🔍 重複ファイルフィルターツール");
    println!("📄 入力ファイル: {}", input_json.display());
    println!("📏 最小ハッシュ距離: {min_distance}");

    // Read duplicates report from JSON file
    let json_content = std::fs::read_to_string(&input_json)?;
    let report: DuplicatesReport = serde_json::from_str(&json_content)?;

    println!(
        "📊 元レポート: {} グループ, {} 重複ファイル (閾値: {})",
        report.total_groups, report.total_duplicates, report.threshold
    );

    // Filter groups based on minimum distance
    let filtered_groups: Vec<DuplicateGroup> = report
        .groups
        .into_iter()
        .filter_map(|group| {
            let original_files_count = group.files.len();

            // Find files that meet the distance criteria (include representative file with distance 0)
            let mut filtered_files: Vec<DuplicateFile> = group
                .files
                .into_iter()
                .filter(|file| file.distance_from_representative >= min_distance || file.distance_from_representative == 0)
                .collect();

            // Remove duplicate file paths (keep only the first occurrence)
            filtered_files.sort_by(|a, b| a.path.cmp(&b.path));
            filtered_files.dedup_by(|a, b| a.path == b.path);

            // Keep group only if it has distance criteria files and more than one file total
            if filtered_files.len() > 1 && original_files_count > 1 {
                // Check if we have files that meet the distance criteria
                let has_distance_matches = filtered_files
                    .iter()
                    .any(|f| f.distance_from_representative >= min_distance);

                if has_distance_matches {
                    Some(DuplicateGroup {
                        group_id: group.group_id,
                        representative_file: group.representative_file,
                        files: filtered_files,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let filtered_duplicates: usize = filtered_groups
        .iter()
        .map(|g| g.files.len().saturating_sub(1))
        .sum();

    let filtered_report = FilteredReport {
        original_threshold: report.threshold,
        filter_threshold: min_distance,
        filtered_groups: filtered_groups.len(),
        filtered_duplicates,
        groups: filtered_groups,
    };

    println!("\n✅ フィルタリング完了!");
    println!("📊 フィルタ結果:");
    println!(
        "   - フィルタ後グループ数: {}",
        filtered_report.filtered_groups
    );
    println!(
        "   - フィルタ後重複ファイル数: {}",
        filtered_report.filtered_duplicates
    );

    // Display filtered results
    if filtered_report.filtered_groups > 0 {
        println!("\n📌 フィルタ結果 (距離 {min_distance} 以上):");
        for group in &filtered_report.groups {
            println!(
                "\n  グループ {} ({} ファイル):",
                group.group_id,
                group.files.len()
            );
            for file in &group.files {
                println!(
                    "    - {} (距離: {})",
                    file.path, file.distance_from_representative
                );
            }
        }
    } else {
        println!("\n❌ 指定された距離条件に一致するグループはありません");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_duplicate_file(
        path: &str,
        hash: &str,
        distance: u32,
    ) -> DuplicateFile {
        DuplicateFile {
            path: path.to_string(),
            hash: hash.to_string(),
            distance_from_representative: distance,
        }
    }

    fn create_test_duplicate_group(
        group_id: usize,
        representative_file: &str,
        files: Vec<DuplicateFile>,
    ) -> DuplicateGroup {
        DuplicateGroup {
            group_id,
            representative_file: representative_file.to_string(),
            files,
        }
    }

    fn create_test_duplicates_report(
        threshold: u32,
        groups: Vec<DuplicateGroup>,
    ) -> DuplicatesReport {
        let total_duplicates = groups.iter().map(|g| g.files.len() - 1).sum();
        DuplicatesReport {
            total_groups: groups.len(),
            total_duplicates,
            threshold,
            groups,
        }
    }

    #[tokio::test]
    async fn test_filter_duplicates_basic() {
        let temp_dir = TempDir::new().unwrap();
        let input_json = temp_dir.path().join("duplicates.json");

        // Create test data with various distances
        let groups = vec![
            create_test_duplicate_group(
                0,
                "original1.jpg",
                vec![
                    create_test_duplicate_file("original1.jpg", "hash1", 0),
                    create_test_duplicate_file("dup1_low.jpg", "hash2", 1), // distance 1
                    create_test_duplicate_file("dup1_high.jpg", "hash3", 5), // distance 5
                ],
            ),
            create_test_duplicate_group(
                1,
                "original2.jpg",
                vec![
                    create_test_duplicate_file("original2.jpg", "hash4", 0),
                    create_test_duplicate_file("dup2_low.jpg", "hash5", 2), // distance 2
                ],
            ),
        ];

        let report = create_test_duplicates_report(10, groups);
        let json = serde_json::to_string_pretty(&report).unwrap();
        fs::write(&input_json, json).unwrap();

        // Test filtering with min_distance = 3
        execute_filter_duplicates(input_json, 3).await.unwrap();
    }

    #[tokio::test]
    async fn test_filter_duplicates_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let input_json = temp_dir.path().join("duplicates.json");

        // Create test data with only low distances
        let groups = vec![create_test_duplicate_group(
            0,
            "original.jpg",
            vec![
                create_test_duplicate_file("original.jpg", "hash1", 0),
                create_test_duplicate_file("dup_low.jpg", "hash2", 1), // distance 1
            ],
        )];

        let report = create_test_duplicates_report(5, groups);
        let json = serde_json::to_string_pretty(&report).unwrap();
        fs::write(&input_json, json).unwrap();

        // Test filtering with min_distance = 5 (should find no matches)
        execute_filter_duplicates(input_json, 5).await.unwrap();
    }

    #[tokio::test]
    async fn test_filter_duplicates_preserves_original() {
        let temp_dir = TempDir::new().unwrap();
        let input_json = temp_dir.path().join("duplicates.json");

        // Create test data where original has distance 0
        let groups = vec![create_test_duplicate_group(
            0,
            "original.jpg",
            vec![
                create_test_duplicate_file("original.jpg", "hash1", 0),
                create_test_duplicate_file("dup1.jpg", "hash2", 3),
                create_test_duplicate_file("dup2.jpg", "hash3", 6),
            ],
        )];

        let report = create_test_duplicates_report(10, groups);
        let json = serde_json::to_string_pretty(&report).unwrap();
        fs::write(&input_json, json).unwrap();

        // Test filtering with min_distance = 4
        execute_filter_duplicates(input_json, 4).await.unwrap();
    }

    #[tokio::test]
    async fn test_filter_duplicates_nonexistent_file() {
        let nonexistent = PathBuf::from("nonexistent.json");

        let result = execute_filter_duplicates(nonexistent, 3).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_filter_duplicates_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let input_json = temp_dir.path().join("invalid.json");

        fs::write(&input_json, "invalid json content").unwrap();

        let result = execute_filter_duplicates(input_json, 3).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_filter_duplicates_empty_groups() {
        let temp_dir = TempDir::new().unwrap();
        let input_json = temp_dir.path().join("empty.json");

        let report = create_test_duplicates_report(5, vec![]);
        let json = serde_json::to_string_pretty(&report).unwrap();
        fs::write(&input_json, json).unwrap();

        execute_filter_duplicates(input_json, 3).await.unwrap();
    }

    #[test]
    fn test_filtered_report_serialization() {
        let file = create_test_duplicate_file("test.jpg", "hash1", 5);
        let group = create_test_duplicate_group(0, "test.jpg", vec![file]);

        let report = FilteredReport {
            original_threshold: 10,
            filter_threshold: 3,
            filtered_groups: 1,
            filtered_duplicates: 0,
            groups: vec![group],
        };

        // Test that structure can be serialized and deserialized
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: FilteredReport = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.original_threshold, 10);
        assert_eq!(deserialized.filter_threshold, 3);
        assert_eq!(deserialized.filtered_groups, 1);
    }

    #[tokio::test]
    async fn test_filter_duplicates_multiple_high_distance_files() {
        let temp_dir = TempDir::new().unwrap();
        let input_json = temp_dir.path().join("multiple.json");

        // Create test data with multiple files meeting distance criteria
        let groups = vec![create_test_duplicate_group(
            0,
            "original.jpg",
            vec![
                create_test_duplicate_file("original.jpg", "hash1", 0),
                create_test_duplicate_file("dup1.jpg", "hash2", 2), // below threshold
                create_test_duplicate_file("dup2.jpg", "hash3", 5), // above threshold
                create_test_duplicate_file("dup3.jpg", "hash4", 7), // above threshold
            ],
        )];

        let report = create_test_duplicates_report(10, groups);
        let json = serde_json::to_string_pretty(&report).unwrap();
        fs::write(&input_json, json).unwrap();

        execute_filter_duplicates(input_json, 4).await.unwrap();
    }
}
