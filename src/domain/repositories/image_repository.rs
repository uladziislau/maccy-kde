use std::path::PathBuf;
use image::RgbaImage;
use crate::domain::entities::ItemId;
use crate::shared::Result;

pub trait ImageRepository: Send + Sync {
    fn save(&self, id: ItemId, image: &RgbaImage, mime_type: &str) -> Result<PathBuf>;
    
    fn load(&self, path: &PathBuf) -> Result<RgbaImage>;
    
    fn delete(&self, path: &PathBuf) -> Result<()>;
    
    fn get_cache_path(&self) -> PathBuf;
}