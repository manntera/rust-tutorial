// Worker - 単一ファイル処理機能

use crate::image_loader::ImageLoaderBackend;
use crate::perceptual_hash::PerceptualHashBackend;
use crate::core::types::{ProcessingResult, ProcessingMetadata};
use std::time::Instant;
use std::path::Path;

/// 単一ファイルの処理
pub async fn process_single_file<L, H>(
    loader: &L,
    hasher: &H,
    file_path: &str,
    _worker_id: usize,
) -> ProcessingResult
where
    L: ImageLoaderBackend,
    H: PerceptualHashBackend,
{
    let start_time = Instant::now();
    
    let result = async {
        // 画像読み込み
        let path = Path::new(file_path);
        let load_result = loader.load_from_path(path).await?;
        
        // ファイルサイズを取得
        let file_size = std::fs::metadata(file_path)?.len();
        
        // ハッシュ生成
        let hash_result = hasher.generate_hash(&load_result.image).await?;
        
        // メタデータ作成
        let metadata = ProcessingMetadata {
            file_size,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            image_dimensions: (load_result.image.width(), load_result.image.height()),
            was_resized: load_result.was_resized,
        };
        
        anyhow::Result::<(String, String, u64, ProcessingMetadata)>::Ok((
            hash_result.to_hex(),
            format!("{:?}", hash_result.algorithm),
            hash_result.to_u64(),
            metadata
        ))
    }.await;
    
    match result {
        Ok((hash, algorithm, hash_bits, metadata)) => ProcessingResult::Success {
            file_path: file_path.to_string(),
            hash,
            algorithm,
            hash_bits,
            metadata,
        },
        Err(error) => ProcessingResult::Error {
            file_path: file_path.to_string(),
            error: error.to_string(),
        },
    }
}