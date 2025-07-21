use anyhow::Result;
use std::path::Path;

// 並列処理APIをインポート
use image_dedup::{
    image_loader::standard::StandardImageLoader,
    perceptual_hash::dct_hash::DCTHasher,
    storage::local::LocalStorageBackend,
    processing::{
        ProcessingEngine,
        DefaultProcessingConfig,
        ProcessingConfig,
        ConsoleProgressReporter,
        StreamingJsonHashPersistence,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 画像重複検出ツール - 並列処理版");
    
    // 1. 出力ディレクトリ設定
    let target_dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let output_file = Path::new(&target_dir).join("image_hashes.json");
    
    println!("📂 対象ディレクトリ: {target_dir}");
    println!("📄 出力ファイル: {}", output_file.display());
    
    // 2. 並列処理エンジン構築
    let engine = ProcessingEngine::new(
        StandardImageLoader::with_max_dimension(512), // 画像リサイズ設定
        DCTHasher::new(8), // 8x8 DCTハッシュ
        LocalStorageBackend::new(),
        DefaultProcessingConfig::default()
            .with_max_concurrent(num_cpus::get() * 2) // CPU数x2の並列度
            .with_batch_size(50) // バッチサイズ50
            .with_progress_reporting(true), // 進捗報告有効
        ConsoleProgressReporter::new(), // コンソール出力
        StreamingJsonHashPersistence::new(&output_file), // ストリーミングJSON出力
    );
    
    println!("⚙️  設定:");
    println!("   - 最大並列数: {}", engine.config().max_concurrent_tasks());
    println!("   - バッチサイズ: {}", engine.config().batch_size());
    println!("   - バッファサイズ: {}", engine.config().channel_buffer_size());
    
    // 3. 並列処理実行
    let start_time = std::time::Instant::now();
    
    match engine.process_directory_with_config(
        &target_dir,
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
            
            println!("📄 結果は {} に保存されました", output_file.display());
        }
        Err(error) => {
            eprintln!("❌ エラー: {error}");
            std::process::exit(1);
        }
    }
    
    Ok(())
}
