/// Integration tests for maccy-kde
/// These tests verify end-to-end workflows across multiple layers

use crate::database::Database;
use crate::integration::RepositoryBridge;
use crate::domain::repositories::ClipboardRepository;
use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType, Category, Timestamp};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_bridge_integration() {
        // Create an in-memory database for test isolation
        let db = Database::in_memory().expect("Failed to create in-memory database");
        let db = Arc::new(db);
        
        // Create RepositoryBridge with real database
        let bridge = RepositoryBridge::new(db.clone());
        
        // Test adding an item through the bridge
        let test_text = "Test clipboard content";
        let item = ClipboardItem {
            id: ItemId::new(1),
            content: Content::Text(test_text.to_string()),
            mime_type: MimeType::new("text/plain".to_string()),
            category: Some(Category::Other),
            is_pinned: false,
            pin_order: 0,
            last_used_at: Timestamp::now(),
        };
        
        // Save item through bridge
        let result = bridge.save(&item);
        assert!(result.is_ok(), "Failed to save item through RepositoryBridge");
        
        // Verify we can retrieve the item - note: ID may differ after DB save
        let all_items = bridge.find_all().expect("Failed to get all items");
        assert!(!all_items.is_empty(), "No items found in database");
        
        // Find our item by content
        let saved_item = all_items.iter().find(|i| {
            match &i.content {
                Content::Text(text) => text == test_text,
                _ => false,
            }
        });
        assert!(saved_item.is_some(), "Saved item not found by content");
    }

    #[test]
    fn test_repository_bridge_text_workflow() {
        // Test complete text workflow: add → retrieve
        let db = Database::in_memory().expect("Failed to create in-memory database");
        let db = Arc::new(db);
        let bridge = RepositoryBridge::new(db.clone());
        
        let workflow_texts = vec![
            "First clipboard item",
            "Second clipboard item", 
            "Third clipboard item",
        ];
        
        for (i, text) in workflow_texts.iter().enumerate() {
            let item = ClipboardItem {
                id: ItemId::new(i as i64 + 1),
                content: Content::Text(text.to_string()),
                mime_type: MimeType::new("text/plain".to_string()),
                category: Some(Category::Other),
                is_pinned: false,
                pin_order: i as i64,
                last_used_at: Timestamp::now(),
            };
            
            assert!(bridge.save(&item).is_ok());
        }
        
        // Verify all items are retrievable
        let all_items = bridge.find_all().expect("Failed to retrieve all items");
        assert_eq!(all_items.len(), 3);
        
        // Verify content
        for text in &workflow_texts {
            let found = all_items.iter().any(|i| {
                match &i.content {
                    Content::Text(t) => t == text,
                    _ => false,
                }
            });
            assert!(found, "Text '{}' not found in retrieved items", text);
        }
    }

    #[test]
    fn test_repository_bridge_concurrent_operations() {
        // Test that RepositoryBridge handles concurrent operations correctly
        let db = Database::in_memory().expect("Failed to create in-memory database");
        let db = Arc::new(db);
        let bridge = Arc::new(RepositoryBridge::new(db.clone()));
        
        let mut handles = vec![];
        
        // Spawn multiple concurrent save operations
        for i in 0..5 {
            let bridge_clone = bridge.clone();
            let handle = std::thread::spawn(move || {
                let item = ClipboardItem {
                    id: ItemId::new(i as i64 + 100),
                    content: Content::Text(format!("Concurrent item {}", i)),
                    mime_type: MimeType::new("text/plain".to_string()),
                    category: Some(Category::Other),
                    is_pinned: false,
                    pin_order: i as i64,
                    last_used_at: Timestamp::now(),
                };
                
                bridge_clone.save(&item)
            });
            handles.push(handle);
        }
        
        // Wait for all operations to complete
        for handle in handles {
            let result = handle.join().expect("Thread panicked");
            assert!(result.is_ok(), "Concurrent save failed");
        }
        
        // Verify all items were saved
        let all_items = bridge.find_all().expect("Failed to retrieve items");
        assert_eq!(all_items.len(), 5);
    }
}