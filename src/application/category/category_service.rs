use crate::domain::services::CategoryDetector;
use crate::domain::entities::{ClipboardItem, Category};

/// Service for managing category detection and assignment
pub struct CategoryService;

impl CategoryService {
    /// Detect category from text content
    pub fn detect_from_text(text: &str) -> Category {
        CategoryDetector::detect(text)
    }
    
    /// Ensure item has category, detect if missing
    pub fn ensure_item_category(item: &mut ClipboardItem) {
        if item.category.is_none() {
            let text = item.display_text();
            item.category = Some(Self::detect_from_text(&text));
        }
    }
    
    /// Detect category for multiple items
    pub fn detect_for_items(items: &mut [ClipboardItem]) {
        for item in items {
            Self::ensure_item_category(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Content, ItemId, MimeType, Timestamp};

    #[test]
    fn test_detect_from_text() {
        assert!(matches!(CategoryService::detect_from_text("https://example.com"), Category::Url));
        assert!(matches!(CategoryService::detect_from_text("user@example.com"), Category::Email));
        assert!(matches!(CategoryService::detect_from_text("@username"), Category::Account));
        assert!(matches!(CategoryService::detect_from_text("plain text"), Category::Other));
    }

    #[test]
    fn test_ensure_item_category() {
        let mut item = ClipboardItem {
            id: ItemId::new(1),
            content: Content::Text("https://example.com".to_string()),
            mime_type: MimeType::new("text/plain".to_string()),
            category: None,
            is_pinned: false,
            pin_order: 0,
            last_used_at: Timestamp::now(),
        };
        
        CategoryService::ensure_item_category(&mut item);
        
        assert!(matches!(item.category, Some(Category::Url)));
    }

    #[test]
    fn test_detect_for_items() {
        let mut items = vec![
            ClipboardItem {
                id: ItemId::new(1),
                content: Content::Text("https://example.com".to_string()),
                mime_type: MimeType::new("text/plain".to_string()),
                category: None,
                is_pinned: false,
                pin_order: 0,
                last_used_at: Timestamp::now(),
            },
            ClipboardItem {
                id: ItemId::new(2),
                content: Content::Text("user@example.com".to_string()),
                mime_type: MimeType::new("text/plain".to_string()),
                category: None,
                is_pinned: false,
                pin_order: 0,
                last_used_at: Timestamp::now(),
            },
        ];
        
        CategoryService::detect_for_items(&mut items);
        
        assert!(matches!(items[0].category, Some(Category::Url)));
        assert!(matches!(items[1].category, Some(Category::Email)));
    }

    #[test]
    fn test_existing_category_preserved() {
        let mut item = ClipboardItem {
            id: ItemId::new(1),
            content: Content::Text("https://example.com".to_string()),
            mime_type: MimeType::new("text/plain".to_string()),
            category: Some(Category::Other), // Explicitly set
            is_pinned: false,
            pin_order: 0,
            last_used_at: Timestamp::now(),
        };
        
        CategoryService::ensure_item_category(&mut item);
        
        // Existing category should be preserved
        assert!(matches!(item.category, Some(Category::Other)));
    }
}