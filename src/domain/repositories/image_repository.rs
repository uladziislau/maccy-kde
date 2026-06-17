use std::path::PathBuf;
use image::RgbaImage;
use crate::domain::entities::ItemId;
use crate::shared::Result;

#[allow(dead_code)] // Reserved for future image storage implementation
pub trait ImageRepository: Send + Sync {
    fn save(&self, id: ItemId, image: &RgbaImage, mime_type: &str) -> Result<PathBuf>;
    
    fn load(&self, path: &PathBuf) -> Result<RgbaImage>;
    
    fn delete(&self, path: &PathBuf) -> Result<()>;
    
    fn get_cache_path(&self) -> PathBuf;
}