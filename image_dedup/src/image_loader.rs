use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::path::Path;

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_image(path: &Path) -> Result<DynamicImage> {
        let image = image::open(path)
            .with_context(|| format!("Failed to load image from: {}", path.display()))?;

        Ok(image)
    }

    pub fn load_with_format(path: &Path, format: ImageFormat) -> Result<DynamicImage> {
        use std::fs::File;
        use std::io::BufReader;

        let file =
            File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;
        let reader = BufReader::new(file);

        let image = image::load(reader, format).with_context(|| {
            format!(
                "Failed to decode image with format {:?}: {}",
                format,
                path.display()
            )
        })?;

        Ok(image)
    }

    pub fn get_image_dimensions(image: &DynamicImage) -> (u32, u32) {
        (image.width(), image.height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn test_load_image() {
        let temp_dir = tempdir().unwrap();
        let image_path = temp_dir.path().join("test.png");

        let img = image::RgbImage::new(10, 10);
        img.save(&image_path).unwrap();

        let loaded = ImageLoader::load_image(&image_path).unwrap();
        let (width, height) = ImageLoader::get_image_dimensions(&loaded);

        assert_eq!(width, 10);
        assert_eq!(height, 10);
    }
}
