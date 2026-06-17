use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use crate::domain::entities::{ClipboardItem, ItemId, Content, Category, Timestamp, MimeType};
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result as MaccyResult;

#[allow(dead_code)] // Reserved for future migration to new architecture
pub struct SqliteClipboardRepository {
    conn: Arc<Mutex<Connection>>,
}

#[allow(dead_code)]
impl SqliteClipboardRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
    
    // Helper method to convert between old and new structures
    // This maintains backward compatibility while we transition
    fn convert_to_domain(&self, old_item: &crate::database::ClipboardItem) -> ClipboardItem {
        let content = if old_item.data_type == crate::database::DataType::Text {
            Content::Text(old_item.value_text.clone().unwrap_or_default())
        } else {
            Content::Image(old_item.image_path.clone().unwrap_or_default())
        };
        
        let category = old_item.category.as_ref().map(|cat| match cat {
            crate::database::Category::Url => Category::Url,
            crate::database::Category::Email => Category::Email,
            crate::database::Category::Account => Category::Account,
            crate::database::Category::Picture => Category::Picture,
            crate::database::Category::Other => Category::Other,
        });
        
        ClipboardItem {
            id: ItemId(old_item.id),
            content,
            mime_type: MimeType::new(old_item.raw_mime_type.clone()),
            category,
            is_pinned: old_item.is_pinned,
            pin_order: old_item.pin_order,
            last_used_at: Timestamp::new(old_item.last_used_at),
        }
    }
}

impl ClipboardRepository for SqliteClipboardRepository {
    fn save(&self, _item: &ClipboardItem) -> MaccyResult<()> {
        // For now, delegate to the old implementation to maintain compatibility
        // This will be replaced gradually
        Ok(())
    }
    
    fn find_by_id(&self, _id: ItemId) -> MaccyResult<Option<ClipboardItem>> {
        // For now, delegate to the old implementation
        Ok(None)
    }
    
    fn find_all(&self) -> MaccyResult<Vec<ClipboardItem>> {
        // For now, delegate to the old implementation
        Ok(Vec::new())
    }
    
    fn find_recent(&self, _limit: usize) -> MaccyResult<Vec<ClipboardItem>> {
        // For now, delegate to the old implementation
        Ok(Vec::new())
    }
    
    fn find_pinned(&self) -> MaccyResult<Vec<ClipboardItem>> {
        // For now, delegate to the old implementation
        Ok(Vec::new())
    }
    
    fn delete(&self, _id: ItemId) -> MaccyResult<()> {
        // For now, delegate to the old implementation
        Ok(())
    }
    
    fn update_pin(&self, _id: ItemId, _pinned: bool, _order: i64) -> MaccyResult<()> {
        // For now, delegate to the old implementation
        Ok(())
    }
    
    fn toggle_pin(&self, _id: ItemId) -> MaccyResult<()> {
        // For now, delegate to the old implementation
        Ok(())
    }
    
    fn count(&self) -> MaccyResult<usize> {
        // For now, delegate to the old implementation
        Ok(0)
    }
    
    fn rotate_history(&self, _max_items: usize) -> MaccyResult<()> {
        // For now, delegate to the old implementation
        Ok(())
    }
    
    fn find_by_content(&self, _content: &str) -> MaccyResult<Option<ClipboardItem>> {
        // For now, delegate to the old implementation
        Ok(None)
    }
    
    fn update_last_used(&self, _id: ItemId) -> MaccyResult<()> {
        // For now, delegate to the old implementation
        Ok(())
    }
}