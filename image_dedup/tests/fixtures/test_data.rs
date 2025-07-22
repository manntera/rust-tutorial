/// Common test data constants for processing tests
/// 
/// This module provides shared test data to avoid duplication across multiple test files.

#[cfg(test)]
use tempfile;

/// A minimal 1x1 PNG image in bytes (67 bytes total)
/// 
/// This is a valid PNG file that can be loaded by image processing libraries.
/// The image is 1x1 pixel, RGBA format, and can be used in tests that need
/// actual image data without requiring large test files.
pub const MINIMAL_PNG_DATA: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// Alias for MINIMAL_PNG_DATA for compatibility with test code
pub const SMALL_PNG: &[u8] = MINIMAL_PNG_DATA;

/// Creates a temporary PNG file with minimal valid data
/// 
/// Returns a tuple of (TempDir, PathBuf) where the PathBuf points to a valid PNG file.
/// The TempDir must be kept alive for the file to remain accessible.
/// 
/// # Example
/// ```
/// use std::fs;
/// let (temp_dir, png_file) = create_test_png_file("test.png");
/// assert!(png_file.exists());
/// ```
#[cfg(test)]
pub fn create_test_png_file(filename: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let png_file = temp_dir.path().join(filename);
    std::fs::write(&png_file, MINIMAL_PNG_DATA).expect("Failed to write PNG file");
    (temp_dir, png_file)
}

/// Creates multiple test PNG files in a temporary directory
/// 
/// Returns a tuple of (TempDir, Vec<PathBuf>) with the specified number of PNG files.
/// Files are named "test0.png", "test1.png", etc.
/// 
/// # Example
/// ```
/// let (temp_dir, png_files) = create_multiple_test_png_files(3);
/// assert_eq!(png_files.len(), 3);
/// ```
#[cfg(test)]
pub fn create_multiple_test_png_files(count: usize) -> (tempfile::TempDir, Vec<std::path::PathBuf>) {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let mut png_files = Vec::new();
    
    for i in 0..count {
        let filename = format!("test{}.png", i);
        let png_file = temp_dir.path().join(&filename);
        std::fs::write(&png_file, MINIMAL_PNG_DATA).expect("Failed to write PNG file");
        png_files.push(png_file);
    }
    
    (temp_dir, png_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_png_data_size() {
        assert_eq!(MINIMAL_PNG_DATA.len(), 67);
    }

    #[test]
    fn test_create_test_png_file() {
        let (_temp_dir, png_file) = create_test_png_file("test.png");
        assert!(png_file.exists());
        assert_eq!(png_file.file_name().unwrap().to_string_lossy(), "test.png");
        
        let content = std::fs::read(&png_file).unwrap();
        assert_eq!(content, MINIMAL_PNG_DATA);
    }

    #[test]
    fn test_create_multiple_test_png_files() {
        let (_temp_dir, png_files) = create_multiple_test_png_files(3);
        assert_eq!(png_files.len(), 3);
        
        for (i, file) in png_files.iter().enumerate() {
            assert!(file.exists());
            let expected_name = format!("test{}.png", i);
            assert_eq!(file.file_name().unwrap().to_string_lossy(), expected_name);
            
            let content = std::fs::read(file).unwrap();
            assert_eq!(content, MINIMAL_PNG_DATA);
        }
    }
}