pub mod image_loader;
pub mod perceptual_hash;
pub mod storage;

#[allow(deprecated)]
pub use image_loader::ImageLoader; // 互換性のため
pub use image_loader::{ImageLoadStrategy, ImageLoaderBackend, ImageLoaderFactory, LoadResult};
#[allow(deprecated)]
pub use perceptual_hash::PerceptualHasher; // 互換性のため
pub use perceptual_hash::{
    ComparisonResult, HashAlgorithm, HashResult, HashStrategy, PerceptualHashBackend,
    PerceptualHashFactory,
};
pub use storage::{StorageBackend, StorageFactory, StorageItem, StorageType};
