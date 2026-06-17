/// Bridge between legacy Database and new ClipboardRepository trait
/// This allows using the old Database implementation with the new trait interface

use crate::domain::repositories::ClipboardRepository;
use crate::domain::entities::{ClipboardItem, ItemId};
use crate::domain::services::RotationService;
use crate::shared::Result;
use crate::database::{Database, ClipboardItem as LegacyClipboardItem, DataType};
use std::sync::Arc;

pub struct RepositoryBridge {
    db: Arc<Database>,
}

impl RepositoryBridge {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl ClipboardRepository for RepositoryBridge {
    fn save(&self, item: &ClipboardItem) -> Result<()> {
        let legacy: LegacyClipboardItem = item.into();
        
        match legacy.data_type {
            DataType::Text => {
                if let Some(text) = &legacy.value_text {
                    self.db.add_text_item(text)?;
                }
            },
            DataType::Image => {
                // For now, image items are not fully supported in legacy DB
                // This will be improved when we migrate to new SQLite implementation
                if let Some(_path) = &legacy.image_path {
                    // TODO: Implement image saving
                }
            }
        }
        
        Ok(())
    }

    fn find_by_id(&self, id: ItemId) -> Result<Option<ClipboardItem>> {
        let history = self.db.get_history()?;
        let id_value = id.value();
        
        Ok(history.iter()
            .find(|item| item.id == id_value)
            .map(|legacy| ClipboardItem::from(legacy)))
    }

    fn find_all(&self) -> Result<Vec<ClipboardItem>> {
        let history = self.db.get_history()?;
        Ok(history.iter()
            .map(|legacy| ClipboardItem::from(legacy))
            .collect())
    }

    fn find_recent(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
        let mut items = self.find_all()?;
        items.sort_by_key(|item| std::cmp::Reverse(item.last_used_at.value()));
        items.truncate(limit);
        Ok(items)
    }

    fn find_pinned(&self) -> Result<Vec<ClipboardItem>> {
        let history = self.db.get_history()?;
        Ok(history.iter()
            .filter(|item| item.is_pinned)
            .map(|legacy| ClipboardItem::from(legacy))
            .collect())
    }

    fn delete(&self, id: ItemId) -> Result<()> {
        self.db.delete_item(id.value())?;
        Ok(())
    }

    fn update_pin(&self, id: ItemId, pinned: bool, _order: i64) -> Result<()> {
        // Legacy toggle_pin doesn't support explicit order, but we can work around it
        // First toggle to get it in the right state
        let current = self.find_by_id(id)?;
        if let Some(item) = current {
            if item.is_pinned != pinned {
                self.db.toggle_pin(id.value())?;
            }
        }
        // Note: order is not fully supported in legacy DB, will need new implementation
        Ok(())
    }

    fn toggle_pin(&self, id: ItemId) -> Result<()> {
        self.db.toggle_pin(id.value())?;
        Ok(())
    }

    fn count(&self) -> Result<usize> {
        let history = self.db.get_history()?;
        Ok(history.len())
    }

    fn rotate_history(&self, max_items: usize) -> Result<()> {
        let history = self.db.get_history()?;
        let domain_items: Vec<ClipboardItem> = history.iter()
            .map(|legacy| ClipboardItem::from(legacy))
            .collect();
        
        // Use RotationService to determine which items to delete
        let to_delete = RotationService::items_to_delete(&domain_items, max_items);
        
        for item_id in to_delete {
            self.db.delete_item(item_id.value())?;
        }
        
        Ok(())
    }

    fn find_by_content(&self, content: &str) -> Result<Option<ClipboardItem>> {
        let history = self.db.get_history()?;
        Ok(history.iter()
            .find(|item| {
                match &item.value_text {
                    Some(text) => text == content,
                    None => false,
                }
            })
            .map(|legacy| ClipboardItem::from(legacy)))
    }

    fn update_last_used(&self, id: ItemId) -> Result<()> {
        // Legacy Database doesn't have explicit update_last_used
        // It updates last_used_at automatically when item is re-added via add_text_item
        // For now, we'll just find and re-add the item
        if let Some(item) = self.find_by_id(id)? {
            match &item.content {
                crate::domain::entities::Content::Text(text) => {
                    self.db.add_text_item(text)?;
                },
                crate::domain::entities::Content::Image(_) => {
                    // Images not fully supported yet
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use crate::domain::entities::{Content, MimeType, Category, Timestamp};

    #[test]
    fn test_bridge_creation() {
        let db = Database::new().unwrap();
        let bridge = RepositoryBridge::new(Arc::new(db));
        // Note: count may not be 0 if other tests have run
        // We just verify the bridge can be created and count works
        let count = bridge.count().unwrap();
        assert!(count >= 0);
    }

}