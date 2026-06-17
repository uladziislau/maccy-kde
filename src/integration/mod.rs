/// Integration layer for gradual migration from old to new architecture
/// This module provides adapters to bridge between legacy code and new Clean Architecture

mod repository_bridge;

pub use repository_bridge::RepositoryBridge;

use crate::database::{Database, ClipboardItem as LegacyClipboardItem, DataType, Category as LegacyCategory};
use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType, Category};
use crate::shared::Result;
use std::path::PathBuf;

/// Adapter that wraps the new ClipboardRepository and provides Database-like interface
/// This allows gradual migration by replacing Database usage with this adapter
pub struct DatabaseAdapter {
    // For now, we'll use the legacy Database internally
    // This will be replaced with new ClipboardRepository in future steps
    legacy_db: Database,
    use_new_architecture: bool,
}

impl DatabaseAdapter {
    /// Create new adapter (currently uses legacy database, can be switched to new architecture)
    pub fn new() -> Result<Self> {
        let legacy_db = Database::new()?;
        Ok(Self {
            legacy_db,
            use_new_architecture: false, // Will be enabled after full migration
        })
    }

    /// Get history (compatible with Database interface)
    pub fn get_history(&self) -> Result<Vec<LegacyClipboardItem>> {
        Ok(self.legacy_db.get_history()?)
    }

    /// Add text item (compatible with Database interface)
    pub fn add_text_item(&self, text: &str) -> Result<i64> {
        self.legacy_db.add_text_item(text)?;
        Ok(0) // Return dummy ID for now, will be improved
    }

    /// Delete item (compatible with Database interface)
    pub fn delete_item(&self, id: i64) -> Result<()> {
        self.legacy_db.delete_item(id)?;
        Ok(())
    }

    /// Toggle pin (compatible with Database interface)
    pub fn toggle_pin(&self, id: i64) -> Result<()> {
        self.legacy_db.toggle_pin(id)?;
        Ok(())
    }

    /// Rotate history (compatible with Database interface)
    /// Note: Legacy Database doesn't have this method, so we implement a basic version
    pub fn rotate_history(&self, max_items: usize) -> Result<()> {
        let history = self.legacy_db.get_history()?;
        let non_pinned_count = history.iter().filter(|item| !item.is_pinned).count();
        
        if non_pinned_count > max_items {
            // Delete oldest unpinned items
            let mut items_to_delete: Vec<_> = history.iter()
                .filter(|item| !item.is_pinned)
                .collect();
            items_to_delete.sort_by_key(|item| item.last_used_at);
            
            let items_to_remove = non_pinned_count - max_items;
            for item in items_to_delete.iter().take(items_to_remove) {
                self.legacy_db.delete_item(item.id)?;
            }
        }
        Ok(())
    }

    /// Enable use of new architecture (for testing and gradual rollout)
    #[cfg(test)]
    pub fn set_use_new_architecture(&mut self, use_new: bool) {
        self.use_new_architecture = use_new;
    }
}

/// Convert legacy ClipboardItem to domain ClipboardItem
impl From<&LegacyClipboardItem> for ClipboardItem {
    fn from(legacy: &LegacyClipboardItem) -> Self {
        let content = match legacy.data_type {
            DataType::Text => {
                legacy.value_text.as_ref()
                    .map(|t| Content::Text(t.clone()))
                    .unwrap_or(Content::Text(String::new()))
            },
            DataType::Image => {
                legacy.image_path.as_ref()
                    .map(|p| Content::Image(p.clone()))
                    .unwrap_or(Content::Image(PathBuf::new()))
            }
        };

        let category = legacy.category.as_ref()
            .and_then(|c| match c {
                LegacyCategory::Url => Some(Category::Url),
                LegacyCategory::Email => Some(Category::Email),
                LegacyCategory::Account => Some(Category::Account),
                LegacyCategory::Picture => Some(Category::Picture),
                LegacyCategory::Other => Some(Category::Other),
            });

        let mime_type = match legacy.data_type {
            DataType::Text => MimeType::text_plain(),
            DataType::Image => MimeType::image_png(),
        };

        ClipboardItem::new(
            ItemId(legacy.id),
            content,
            mime_type,
            category,
        )
    }
}

/// Convert domain ClipboardItem to legacy ClipboardItem
impl From<&ClipboardItem> for LegacyClipboardItem {
    fn from(domain: &ClipboardItem) -> Self {
        let (data_type, value_text, image_path) = match &domain.content {
            Content::Text(text) => (
                DataType::Text,
                Some(text.clone()),
                None,
            ),
            Content::Image(path) => (
                DataType::Image,
                None,
                Some(path.clone()),
            ),
        };

        let category = domain.category.as_ref()
            .map(|c| match c {
                Category::Url => LegacyCategory::Url,
                Category::Email => LegacyCategory::Email,
                Category::Account => LegacyCategory::Account,
                Category::Picture => LegacyCategory::Picture,
                Category::Other => LegacyCategory::Other,
            });

        LegacyClipboardItem {
            id: domain.id.value(),
            value_text,
            image_path,
            data_type,
            raw_mime_type: domain.mime_type.value().to_string(),
            category,
            is_pinned: domain.is_pinned,
            pin_order: domain.pin_order,
            last_used_at: domain.last_used_at.value(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = DatabaseAdapter::new();
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_legacy_to_domain_conversion() {
        let legacy = LegacyClipboardItem {
            id: 1,
            value_text: Some("test".to_string()),
            image_path: None,
            data_type: DataType::Text,
            raw_mime_type: "text/plain".to_string(),
            category: None,
            is_pinned: false,
            pin_order: 0,
            last_used_at: 0,
        };

        let domain: ClipboardItem = (&legacy).into();
        assert_eq!(domain.id.value(), 1);
    }

    #[test]
    fn test_domain_to_legacy_conversion() {
        let domain = ClipboardItem::new(
            ItemId(1),
            Content::Text("test".to_string()),
            MimeType::text_plain(),
            None,
        );

        let legacy: LegacyClipboardItem = (&domain).into();
        assert_eq!(legacy.id, 1);
    }
}