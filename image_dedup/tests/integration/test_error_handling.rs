// エラーハンドリングの統合テスト
use anyhow::Result;
use image_dedup::{
    engine::ProcessingEngine,
    image_loader::{standard::StandardImageLoader, ImageLoaderBackend},
    perceptual_hash::dct_hash::DCTHasher,
    storage::local::LocalStorageBackend,
    services::{DefaultProcessingConfig, NoOpProgressReporter, MemoryHashPersistence},
    core::error::ProcessingError,
};
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;

/// テスト用の破損画像ファイルを作成
fn create_corrupted_image_file(filename: &str) -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join(filename);
    // 意図的に不正なPNGヘッダーで破損ファイルを作成
    let corrupted_data = b"INVALID_PNG_DATA";
    fs::write(&file_path, corrupted_data)?;
    Ok((temp_dir, file_path))
}

/// 読み込み不可能なファイルを作成（権限エラー用）
fn create_unreadable_file(filename: &str) -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join(filename);
    fs::write(&file_path, b"some content")?;
    
    // Unixシステムでのみ権限を変更
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&file_path)?.permissions();
        perms.set_mode(0o000); // 読み取り不可
        fs::set_permissions(&file_path, perms)?;
    }
    
    Ok((temp_dir, file_path))
}

#[tokio::test]
async fn test_nonexistent_directory_error() -> Result<()> {
    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let result = engine.process_directory("nonexistent_directory").await;
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, ProcessingError::FileDiscoveryError { .. }));
    assert!(error.to_string().contains("nonexistent_directory"));
    Ok(())
}

#[tokio::test]
async fn test_empty_directory_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let empty_dir_path = temp_dir.path().to_str().unwrap();

    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let result = engine.process_directory(empty_dir_path).await?;
    
    // 空のディレクトリは正常に処理される（エラーではない）
    assert_eq!(result.total_files, 0);
    assert_eq!(result.processed_files, 0);
    assert_eq!(result.error_count, 0);
    Ok(())
}

#[tokio::test]
async fn test_corrupted_image_file_error() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (_temp_file, corrupted_file) = create_corrupted_image_file("corrupted.png")?;
    fs::copy(&corrupted_file, temp_dir.path().join("corrupted.png"))?;

    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let result = engine.process_directory(temp_dir.path().to_str().unwrap()).await?;
    
    // 破損したファイルはエラーとしてカウントされる
    assert_eq!(result.total_files, 1);
    assert_eq!(result.processed_files, 0);
    assert_eq!(result.error_count, 1);
    Ok(())
}

#[tokio::test]
async fn test_mixed_valid_invalid_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // 有効な1x1 PNGファイル
    const MINIMAL_PNG_DATA: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    // 有効なPNGファイルを作成
    fs::write(temp_dir.path().join("valid.png"), MINIMAL_PNG_DATA)?;
    
    // 破損したファイルを作成
    fs::write(temp_dir.path().join("corrupted.png"), b"INVALID_PNG_DATA")?;
    
    // テキストファイル（画像ではない）を作成
    fs::write(temp_dir.path().join("text.txt"), b"This is not an image")?;

    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let result = engine.process_directory(temp_dir.path().to_str().unwrap()).await?;
    
    // 有効なファイルは処理され、無効なファイルはエラーとしてカウント
    assert_eq!(result.total_files, 3); // PNG, PNG(破損), TXT
    assert_eq!(result.processed_files, 1); // 有効なPNGのみ
    assert_eq!(result.error_count, 2); // 破損PNG + TXT
    Ok(())
}

#[tokio::test]
#[cfg(unix)] // Unixシステムでのみ実行
async fn test_permission_error() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (_temp_file, unreadable_file) = create_unreadable_file("unreadable.png")?;
    fs::copy(&unreadable_file, temp_dir.path().join("unreadable.png"))?;
    
    // ファイルの権限を読み取り不可に変更
    use std::os::unix::fs::PermissionsExt;
    let unreadable_path = temp_dir.path().join("unreadable.png");
    let mut perms = fs::metadata(&unreadable_path)?.permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&unreadable_path, perms)?;

    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let result = engine.process_directory(temp_dir.path().to_str().unwrap()).await?;
    
    // 権限エラーでアクセスできないファイルはエラーとしてカウント
    assert_eq!(result.total_files, 1);
    assert_eq!(result.processed_files, 0);
    assert_eq!(result.error_count, 1);
    
    // 後始末のためにパーミッションを戻す
    perms.set_mode(0o644);
    fs::set_permissions(&unreadable_path, perms)?;
    Ok(())
}

#[tokio::test]
async fn test_invalid_configuration_error() -> Result<()> {
    // 並列タスク数0で設定エラーをテスト
    let config = DefaultProcessingConfig::new(1).with_max_concurrent(0);
    
    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        config,
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let temp_dir = TempDir::new()?;
    let result = engine.process_directory(temp_dir.path().to_str().unwrap()).await;
    
    // 不正な設定はエラーを発生させる
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, ProcessingError::ConfigurationError { .. }));
    Ok(())
}

#[tokio::test]
async fn test_file_path_edge_cases() -> Result<()> {
    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(1),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    // 空の文字列
    let result1 = engine.process_directory("").await;
    assert!(result1.is_err());
    
    // 非常に長いパス
    let long_path = "a".repeat(1000);
    let result2 = engine.process_directory(&long_path).await;
    assert!(result2.is_err());
    
    // 特殊文字を含むパス
    let special_path = "/path/with/特殊文字/and spaces/and-symbols!@#$%";
    let result3 = engine.process_directory(special_path).await;
    assert!(result3.is_err()); // 存在しないため
    Ok(())
}

#[tokio::test]
async fn test_concurrent_access_error_resilience() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // 複数のファイルを作成
    const MINIMAL_PNG_DATA: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    
    for i in 0..10 {
        fs::write(
            temp_dir.path().join(format!("test{}.png", i)),
            MINIMAL_PNG_DATA
        )?;
    }

    // 高並列度で処理してエラー耐性をテスト
    let engine = ProcessingEngine::new(
        StandardImageLoader::new(),
        DCTHasher::new(8),
        LocalStorageBackend::new(),
        DefaultProcessingConfig::new(8).with_max_concurrent(16),
        NoOpProgressReporter::new(),
        MemoryHashPersistence::new(),
    );

    let result = engine.process_directory(temp_dir.path().to_str().unwrap()).await?;
    
    // すべてのファイルが正常に処理されることを確認
    assert_eq!(result.total_files, 10);
    assert_eq!(result.processed_files, 10);
    assert_eq!(result.error_count, 0);
    Ok(())
}