use crate::application::clipboard::ItemManagementService;
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;

/// Command to manage clipboard items
pub struct ItemCommand {
    service: Arc<ItemManagementService>,
}

impl ItemCommand {
    pub fn new(repository: Arc<dyn ClipboardRepository>) -> Self {
        Self {
            service: Arc::new(ItemManagementService::new(repository)),
        }
    }

    /// List recent items
    pub fn list_recent(&self, limit: usize) -> Result<Vec<ItemSummary>> {
        let items = self.service.get_recent_items(limit)?;
        let summaries: Vec<ItemSummary> = items
            .iter()
            .map(|item| ItemSummary {
                id: item.id.to_string(),
                display_text: item.display_text(),
                is_pinned: item.is_pinned,
                has_category: item.category.is_some(),
            })
            .collect();
        
        Ok(summaries)
    }

    /// List pinned items
    pub fn list_pinned(&self) -> Result<Vec<ItemSummary>> {
        let items = self.service.get_pinned_items()?;
        let summaries: Vec<ItemSummary> = items
            .iter()
            .map(|item| ItemSummary {
                id: item.id.to_string(),
                display_text: item.display_text(),
                is_pinned: item.is_pinned,
                has_category: item.category.is_some(),
            })
            .collect();
        
        Ok(summaries)
    }

    /// Delete item by ID
    pub fn delete(&self, id_str: &str) -> Result<()> {
        let id = id_str.parse::<i64>()
            .map_err(|e| crate::shared::MaccyError::Validation(format!("Invalid ID: {}", e)))?;
        let item_id = crate::domain::entities::ItemId(id);
        self.service.delete_item(item_id)
    }

    /// Toggle pin status
    pub fn toggle_pin(&self, id_str: &str) -> Result<()> {
        let id = id_str.parse::<i64>()
            .map_err(|e| crate::shared::MaccyError::Validation(format!("Invalid ID: {}", e)))?;
        let item_id = crate::domain::entities::ItemId(id);
        self.service.toggle_pin(item_id)
    }

    /// Get item details
    pub fn get_item(&self, id_str: &str) -> Result<Option<ItemDetails>> {
        let id = id_str.parse::<i64>()
            .map_err(|e| crate::shared::MaccyError::Validation(format!("Invalid ID: {}", e)))?;
        let item_id = crate::domain::entities::ItemId(id);
        
        if let Some(item) = self.service.get_item(item_id)? {
            Ok(Some(ItemDetails {
                id: item.id.to_string(),
                display_text: item.display_text(),
                is_pinned: item.is_pinned,
                category: item.category.as_ref().map(|c| format!("{:?}", c)),
                mime_type: item.mime_type.value().to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get total count
    pub fn count(&self) -> Result<usize> {
        self.service.get_total_count()
    }

    /// Clear all unpinned items
    pub fn clear_unpinned(&self) -> Result<usize> {
        let items = self.service.get_recent_items(1000)?;
        let mut deleted = 0;
        
        for item in items {
            if !item.is_pinned {
                self.service.delete_item(item.id)?;
                deleted += 1;
            }
        }
        
        Ok(deleted)
    }
}

#[derive(Debug, Clone)]
pub struct ItemSummary {
    pub id: String,
    pub display_text: String,
    pub is_pinned: bool,
    pub has_category: bool,
}

#[derive(Debug, Clone)]
pub struct ItemDetails {
    pub id: String,
    pub display_text: String,
    pub is_pinned: bool,
    pub category: Option<String>,
    pub mime_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType, Category};

    struct MockRepo {
        items: std::sync::Mutex<Vec<ClipboardItem>>,
    }

    impl MockRepo {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                items: std::sync::Mutex::new(Vec::new()),
            })
        }

        fn add_item(&self, id: i64, text: &str, category: Option<Category>) {
            let mut items = self.items.lock().unwrap();
            items.push(ClipboardItem::new(
                ItemId(id),
                Content::Text(text.to_string()),
                MimeType::text_plain(),
                category,
            ));
        }
    }

    impl ClipboardRepository for MockRepo {
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
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.is_pinned = !item.is_pinned;
            }
            Ok(())
        }

        fn count(&self) -> Result<usize> {
            let items = self.items.lock().unwrap();
            Ok(items.len())
        }

        fn rotate_history(&self, _max_items: usize) -> Result<()> {
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
    fn test_item_command_creation() {
        let repo = MockRepo::new();
        let command = ItemCommand::new(repo);
        
        assert_eq!(command.count().unwrap(), 0);
    }

    #[test]
    fn test_list_recent() {
        let repo = MockRepo::new();
        repo.add_item(1, "item1", None);
        repo.add_item(2, "item2", None);
        
        let command = ItemCommand::new(repo);
        let items = command.list_recent(10).unwrap();
        
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_list_pinned() {
        let repo = MockRepo::new();
        repo.add_item(1, "pinned", Some(Category::Url));
        
        // Pin the item
        repo.update_pin(crate::domain::entities::ItemId(1), true, 1).unwrap();
        
        let command = ItemCommand::new(repo);
        let items = command.list_pinned().unwrap();
        
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_delete_item() {
        let repo = MockRepo::new();
        repo.add_item(1, "to delete", None);
        
        let command = ItemCommand::new(repo);
        command.delete("1").unwrap();
        
        assert_eq!(command.count().unwrap(), 0);
    }

    #[test]
    fn test_delete_invalid_id() {
        let repo = MockRepo::new();
        let command = ItemCommand::new(repo);
        
        let result = command.delete("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_toggle_pin() {
        let repo = MockRepo::new();
        repo.add_item(1, "item", None);
        
        let command = ItemCommand::new(repo);
        command.toggle_pin("1").unwrap();
        
        let items = command.list_pinned().unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_get_item() {
        let repo = MockRepo::new();
        repo.add_item(1, "test item", None);
        
        let command = ItemCommand::new(repo);
        let item = command.get_item("1").unwrap();
        
        assert!(item.is_some());
        assert_eq!(item.unwrap().display_text, "test item");
    }

    #[test]
    fn test_get_item_not_found() {
        let repo = MockRepo::new();
        let command = ItemCommand::new(repo);
        
        let item = command.get_item("999");
        assert!(item.is_ok());
        assert!(item.unwrap().is_none());
    }

    #[test]
    fn test_clear_unpinned() {
        let repo = MockRepo::new();
        repo.add_item(1, "unpinned", None);
        
        let command = ItemCommand::new(repo);
        let deleted = command.clear_unpinned().unwrap();
        
        assert_eq!(deleted, 1);
        assert_eq!(command.count().unwrap(), 0);
    }
}