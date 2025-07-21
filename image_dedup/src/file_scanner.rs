use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileScanner;

impl FileScanner {
    pub fn scan_directory(directory: &Path) -> Result<Vec<PathBuf>> {
        let mut file_paths = Vec::new();

        for entry in WalkDir::new(directory) {
            let entry = entry?;

            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    let ext = extension.to_string_lossy().to_lowercase();
                    if Self::is_image_extension(&ext) {
                        file_paths.push(entry.path().to_path_buf());
                    }
                }
            }
        }

        Ok(file_paths)
    }

    fn is_image_extension(extension: &str) -> bool {
        matches!(
            extension,
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scan_directory() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join("image1.jpg"), b"dummy").unwrap();
        fs::write(temp_path.join("image2.png"), b"dummy").unwrap();
        fs::write(temp_path.join("document.txt"), b"dummy").unwrap();

        let result = FileScanner::scan_directory(temp_path).unwrap();

        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .any(|p| p.file_name().unwrap() == "image1.jpg")
        );
        assert!(
            result
                .iter()
                .any(|p| p.file_name().unwrap() == "image2.png")
        );
    }

    #[test]
    fn test_is_image_extension() {
        assert!(FileScanner::is_image_extension("jpg"));
        assert!(FileScanner::is_image_extension("png"));
        assert!(!FileScanner::is_image_extension("txt"));
    }
}
