use crate::domain::entities::{ClipboardItem, ItemId};
use crate::shared::Result;

pub trait ClipboardRepository: Send + Sync {
    fn save(&self, item: &ClipboardItem) -> Result<()>;
    
    fn find_by_id(&self, id: ItemId) -> Result<Option<ClipboardItem>>;
    
    fn find_all(&self) -> Result<Vec<ClipboardItem>>;
    
    fn find_recent(&self, limit: usize) -> Result<Vec<ClipboardItem>>;
    
    fn find_pinned(&self) -> Result<Vec<ClipboardItem>>;
    
    fn delete(&self, id: ItemId) -> Result<()>;
    
    fn update_pin(&self, id: ItemId, pinned: bool, order: i64) -> Result<()>;
    
    fn toggle_pin(&self, id: ItemId) -> Result<()>;
    
    fn count(&self) -> Result<usize>;
    
    fn rotate_history(&self, max_items: usize) -> Result<()>;
    
    fn find_by_content(&self, content: &str) -> Result<Option<ClipboardItem>>;
    
    fn update_last_used(&self, id: ItemId) -> Result<()>;
}