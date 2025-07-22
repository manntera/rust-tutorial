use anyhow::Result;
use std::path::PathBuf;
use crate::{
    image_loader::standard::StandardImageLoader,
    perceptual_hash::dct_hash::DCTHasher,
    storage::local::LocalStorageBackend,
    engine::ProcessingEngine,
    services::{
        DefaultProcessingConfig,
        ConsoleProgressReporter,
        StreamingJsonHashPersistence,
    },
    core::ProcessingConfig,
};

/// Scan command implementation
pub async fn execute_scan(
    target_directory: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    force: bool,
) -> Result<()> {
    // Validate target directory
    if !target_directory.exists() {
        anyhow::bail!("Target directory does not exist: {}", target_directory.display());
    }
    
    if !target_directory.is_dir() {
        anyhow::bail!("Target path is not a directory: {}", target_directory.display());
    }

    // Check if output file exists and handle --force flag
    if output.exists() && !force {
        anyhow::bail!(
            "Output file already exists: {}. Use --force to overwrite.",
            output.display()
        );
    }

    // Create output directory if it doesn't exist
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Determine thread count
    let thread_count = threads.unwrap_or_else(num_cpus::get);
    
    println!("🚀 画像重複検出ツール - scanコマンド");
    println!("📂 対象ディレクトリ: {}", target_directory.display());
    println!("📄 出力ファイル: {}", output.display());
    println!("🧵 使用スレッド数: {thread_count}");

    // Build processing engine
    let engine = ProcessingEngine::new(
        StandardImageLoader::with_max_dimension(512),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::default()
            .with_max_concurrent(thread_count * 2)
            .with_batch_size(50)
            .with_progress_reporting(true),
        ConsoleProgressReporter::new(),
        StreamingJsonHashPersistence::new(&output),
    );

    println!("⚙️  設定:");
    println!("   - 最大並列数: {}", engine.config().max_concurrent_tasks());
    println!("   - バッチサイズ: {}", engine.config().batch_size());
    println!("   - バッファサイズ: {}", engine.config().channel_buffer_size());

    // Execute processing
    let start_time = std::time::Instant::now();
    
    let target_str = target_directory.to_string_lossy();
    match engine.process_directory_with_config(
        &target_str,
        engine.config(),
        engine.reporter(),
        engine.persistence(),
    ).await {
        Ok(summary) => {
            let elapsed = start_time.elapsed();
            
            println!("\n✅ 処理完了!");
            println!("📊 処理結果:");
            println!("   - 対象ファイル数: {}", summary.total_files);
            println!("   - 成功処理数: {}", summary.processed_files);
            println!("   - エラー数: {}", summary.error_count);
            println!("   - 総処理時間: {:.2}秒", elapsed.as_secs_f64());
            println!("   - 平均処理時間: {:.2}ms/ファイル", summary.average_time_per_file_ms);
            
            if summary.error_count > 0 {
                println!("⚠️  {}個のファイルでエラーが発生しました", summary.error_count);
            }
            
            println!("📄 結果は {} に保存されました", output.display());
        }
        Err(error) => {
            anyhow::bail!("処理エラー: {}", error);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_scan_nonexistent_directory() {
        let nonexistent_dir = PathBuf::from("nonexistent_directory");
        let output = PathBuf::from("output.json");
        
        let result = execute_scan(nonexistent_dir, output, None, false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_scan_file_instead_of_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let output = PathBuf::from("output.json");
        
        let result = execute_scan(file_path, output, None, false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[tokio::test]
    async fn test_scan_existing_output_without_force() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("existing_output.json");
        fs::write(&output, "existing content").unwrap();
        
        let target_dir = TempDir::new().unwrap();
        
        let result = execute_scan(target_dir.path().to_path_buf(), output, None, false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}