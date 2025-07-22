use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;
use serde::Deserialize;
use crate::cli::ProcessAction;

#[derive(Debug, Deserialize)]
struct DuplicatesReport {
    total_groups: usize,
    total_duplicates: usize,
    #[allow(dead_code)]
    threshold: u32,
    groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Deserialize)]
struct DuplicateGroup {
    group_id: usize,
    files: Vec<DuplicateFile>,
}

#[derive(Debug, Deserialize)]
struct DuplicateFile {
    path: String,
    #[allow(dead_code)]
    hash: String,
    #[allow(dead_code)]
    distance_from_first: u32,
}

/// Prompt user for confirmation
fn confirm_action(action: &ProcessAction, total_files: usize) -> Result<bool> {
    use std::io::{self, Write};
    
    print!("⚠️  {} files will be {}. Continue? [y/N]: ", 
           total_files, 
           match action {
               ProcessAction::Move => "moved",
               ProcessAction::Delete => "PERMANENTLY DELETED"
           });
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_lowercase() == "y")
}

/// Process duplicate images (move or delete)
pub async fn execute_process(
    duplicate_list: PathBuf,
    action: ProcessAction,
    dest: PathBuf,
    no_confirm: bool,
) -> Result<()> {
    // Validate input file
    if !duplicate_list.exists() {
        anyhow::bail!("Duplicate list file does not exist: {}", duplicate_list.display());
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
    
    println!("\n📊 重複情報:");
    println!("   - グループ数: {}", report.total_groups);
    println!("   - 重複ファイル総数: {}", report.total_duplicates);
    
    // Count files to process (keep first file in each group)
    let files_to_process: Vec<(usize, &DuplicateFile)> = report.groups.iter()
        .flat_map(|group| {
            group.files.iter()
                .skip(1) // Keep the first file
                .map(move |file| (group.group_id, file))
        })
        .collect();
    
    println!("   - 処理対象ファイル数: {} (各グループの最初のファイルは保持)", files_to_process.len());
    
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
    
    for (group_id, file) in files_to_process {
        let source_path = Path::new(&file.path);
        
        match &action {
            ProcessAction::Move => {
                let filename = source_path.file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
                let dest_subdir = dest.join(format!("group_{group_id}"));
                fs::create_dir_all(&dest_subdir)?;
                let dest_path = dest_subdir.join(filename);
                
                match fs::rename(source_path, &dest_path) {
                    Ok(_) => {
                        println!("✓ 移動: {} → {}", source_path.display(), dest_path.display());
                        success_count += 1;
                    }
                    Err(e) => {
                        eprintln!("✗ エラー: {} - {}", source_path.display(), e);
                        error_count += 1;
                    }
                }
            }
            ProcessAction::Delete => {
                match fs::remove_file(source_path) {
                    Ok(_) => {
                        println!("✓ 削除: {}", source_path.display());
                        success_count += 1;
                    }
                    Err(e) => {
                        eprintln!("✗ エラー: {} - {}", source_path.display(), e);
                        error_count += 1;
                    }
                }
            }
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
    use tempfile::TempDir;

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
        let empty_report = r#"{
            "total_groups": 0,
            "total_duplicates": 0,
            "threshold": 5,
            "groups": []
        }"#;
        fs::write(&dup_list, empty_report).unwrap();
        
        let result = execute_process(dup_list, ProcessAction::Move, dest, true).await;
        assert!(result.is_ok());
    }
}