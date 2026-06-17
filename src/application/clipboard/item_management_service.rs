use crate::domain::entities::{ClipboardItem, ItemId, Content, Category};
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;

pub struct ItemManagementService {
    #[allow(dead_code)] // Used internally via methods
    repository: Arc<dyn ClipboardRepository>,
}

impl ItemManagementService {
    pub fn new(repository: Arc<dyn ClipboardRepository>) -> Self {
        Self { repository }
    }

    /// Add a new text item to clipboard history
    pub fn add_text_item(&self, text: &str) -> Result<ItemId> {
        let category = Some(Category::from_text(text));
        let item = ClipboardItem::new(
            ItemId(0), // Will be assigned by repository
            Content::Text(text.to_string()),
            crate::domain::entities::MimeType::text_plain(),
            category,
        );
        
        // Check for duplicates
        if let Some(existing) = self.repository.find_by_content(text)? {
            self.repository.update_last_used(existing.id)?;
            return Ok(existing.id);
        }
        
        // Add new item (repository will assign ID)
        self.repository.save(&item)?;
        
        // Get the item back to get its assigned ID
        // In a real implementation, we'd have save return the ID
        // For now, we'll find by content again
        if let Some(saved) = self.repository.find_by_content(text)? {
            Ok(saved.id)
        } else {
            // Fallback: this shouldn't happen but return a placeholder
            Ok(ItemId(0))
        }
    }

    /// Get recent items with limit
    pub fn get_recent_items(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
        self.repository.find_recent(limit)
    }

    /// Get all pinned items
    pub fn get_pinned_items(&self) -> Result<Vec<ClipboardItem>> {
        self.repository.find_pinned()
    }

    /// Toggle pin status of an item
    pub fn toggle_pin(&self, id: ItemId) -> Result<()> {
        self.repository.toggle_pin(id)
    }

    /// Delete an item
    pub fn delete_item(&self, id: ItemId) -> Result<()> {
        self.repository.delete(id)
    }

    /// Get item by ID
    pub fn get_item(&self, id: ItemId) -> Result<Option<ClipboardItem>> {
        self.repository.find_by_id(id)
    }

    /// Get total count of items
    pub fn get_total_count(&self) -> Result<usize> {
        self.repository.count()
    }

    /// Rotate history to maintain maximum size
    pub fn rotate_history(&self, max_items: usize) -> Result<()> {
        self.repository.rotate_history(max_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Content, ItemId, MimeType};

    // Mock repository for testing
    struct MockClipboardRepository {
        items: std::sync::Mutex<Vec<ClipboardItem>>,
    }

    impl MockClipboardRepository {
        fn new() -> Self {
            Self {
                items: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl ClipboardRepository for MockClipboardRepository {
        fn save(&self, item: &ClipboardItem) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            let mut new_item = item.clone();
            new_item.id = ItemId(items.len() as i64 + 1);
            items.push(new_item);
            Ok(())
        }

        fn find_by_id(&self, id: ItemId) -> Result<Option<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.iter().find(|i| i.id == id).cloned())
        }

        fn find_all(&self) -> Result<Vec<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.clone())
        }

        fn find_recent(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.iter().rev().take(limit).cloned().collect())
        }

        fn find_pinned(&self) -> Result<Vec<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.iter().filter(|i| i.is_pinned).cloned().collect())
        }

        fn delete(&self, id: ItemId) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            items.retain(|i| i.id != id);
            Ok(())
        }

        fn update_pin(&self, id: ItemId, pinned: bool, order: i64) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.is_pinned = pinned;
                item.pin_order = order;
            }
            Ok(())
        }

        fn toggle_pin(&self, id: ItemId) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            let is_pinned = if let Some(item) = items.iter().find(|i| i.id == id) {
                item.is_pinned
            } else {
                return Ok(());
            };
            
            let new_pin_order = if !is_pinned {
                items.len() as i64
            } else {
                0
            };
            
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.is_pinned = !is_pinned;
                item.pin_order = new_pin_order;
            }
            Ok(())
        }

        fn count(&self) -> Result<usize> {
            let items = self.items.lock().unwrap();
            Ok(items.len())
        }

        fn rotate_history(&self, _max_items: usize) -> Result<()> {
            // Simple implementation for testing
            Ok(())
        }

        fn find_by_content(&self, content: &str) -> Result<Option<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.iter().find(|i| {
                match &i.content {
                    Content::Text(text) => text == content,
                    Content::Image(_) => false,
                }
            }).cloned())
        }

        fn update_last_used(&self, id: ItemId) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.update_last_used();
            }
            Ok(())
        }
    }

    #[test]
    fn test_add_text_item() {
        let repo = Arc::new(MockClipboardRepository::new());
        let service = ItemManagementService::new(repo);
        
        let id = service.add_text_item("test text").unwrap();
        assert!(id.value() > 0);
    }

    #[test]
    fn test_get_recent_items() {
        let repo = Arc::new(MockClipboardRepository::new());
        let service = ItemManagementService::new(repo.clone());
        
        service.add_text_item("item1").unwrap();
        service.add_text_item("item2").unwrap();
        
        let recent = service.get_recent_items(1).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_toggle_pin() {
        let repo = Arc::new(MockClipboardRepository::new());
        let service = ItemManagementService::new(repo);
        
        let id = service.add_text_item("test").unwrap();
        service.toggle_pin(id).unwrap();
        
        let item = service.get_item(id).unwrap();
        assert!(item.unwrap().pinned());
    }

    #[test]
    fn test_delete_item() {
        let repo = Arc::new(MockClipboardRepository::new());
        let service = ItemManagementService::new(repo);
        
        let id = service.add_text_item("test").unwrap();
        service.delete_item(id).unwrap();
        
        let item = service.get_item(id).unwrap();
        assert!(item.is_none());
    }

    #[test]
    fn test_duplicate_detection() {
        let repo = Arc::new(MockClipboardRepository::new());
        let service = ItemManagementService::new(repo);
        
        let id1 = service.add_text_item("duplicate").unwrap();
        let id2 = service.add_text_item("duplicate").unwrap();
        
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_get_total_count() {
        let repo = Arc::new(MockClipboardRepository::new());
        let service = ItemManagementService::new(repo);
        
        service.add_text_item("item1").unwrap();
        service.add_text_item("item2").unwrap();
        
        let count = service.get_total_count().unwrap();
        assert_eq!(count, 2);
    }
}