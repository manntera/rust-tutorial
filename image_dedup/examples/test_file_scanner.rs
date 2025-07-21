use anyhow::Result;
use image_dedup::FileScanner;
use std::path::Path;

fn main() -> Result<()> {
    println!("=== FileScanner 動作確認 ===\n");

    // テスト用の画像ディレクトリを作成
    let test_dir = Path::new("G:\共有ドライブ\メディア\画像\temp\かかお");
    if !test_dir.exists() {
        println!("テスト用ディレクトリを作成します: {}", test_dir.display());
        std::fs::create_dir_all(test_dir)?;
        
        // サンプル画像を作成
        create_sample_images(test_dir)?;
    }

    // ディレクトリをスキャン
    println!("\nディレクトリをスキャン中: {}", test_dir.display());
    let files = FileScanner::scan_directory(test_dir)?;
    
    println!("\n見つかった画像ファイル: {} 個", files.len());
    println!("----------------------------------------");
    
    for (i, file) in files.iter().enumerate() {
        println!("{:3}. {}", i + 1, file.display());
        
        // ファイルサイズも表示
        if let Ok(metadata) = std::fs::metadata(file) {
            let size = metadata.len();
            let size_kb = size as f64 / 1024.0;
            println!("     サイズ: {:.2} KB", size_kb);
        }
    }
    
    // 拡張子別の統計
    println!("\n拡張子別の統計:");
    println!("----------------------------------------");
    let mut ext_counts = std::collections::HashMap::new();
    
    for file in &files {
        if let Some(ext) = file.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            *ext_counts.entry(ext_str).or_insert(0) += 1;
        }
    }
    
    for (ext, count) in ext_counts.iter() {
        println!("  .{}: {} ファイル", ext, count);
    }
    
    // サブディレクトリのテスト
    let sub_dir = test_dir.join("subdirectory");
    if !sub_dir.exists() {
        std::fs::create_dir_all(&sub_dir)?;
        create_sample_image(&sub_dir.join("nested_image.png"))?;
    }
    
    println!("\nサブディレクトリを含むスキャン:");
    let all_files = FileScanner::scan_directory(test_dir)?;
    println!("合計: {} ファイル (サブディレクトリ含む)", all_files.len());
    
    Ok(())
}

fn create_sample_images(dir: &Path) -> Result<()> {
    // 異なる拡張子のサンプル画像を作成
    let extensions = vec![
        "sample1.jpg",
        "sample2.jpeg",
        "sample3.png",
        "sample4.gif",
        "sample5.bmp",
        "sample6.tiff",
        "not_image.txt",  // 画像ではないファイル
    ];
    
    for filename in extensions {
        let path = dir.join(filename);
        
        if filename.ends_with(".txt") {
            // テキストファイルを作成
            std::fs::write(&path, "This is not an image file")?;
        } else {
            create_sample_image(&path)?;
        }
    }
    
    Ok(())
}

fn create_sample_image(path: &Path) -> Result<()> {
    use image::{ImageBuffer, RgbImage};
    
    let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
        let r = ((x * 255) / 100) as u8;
        let g = ((y * 255) / 100) as u8;
        let b = (((x + y) * 255) / 200) as u8;
        image::Rgb([r, g, b])
    });
    
    img.save(path)?;
    Ok(())
}