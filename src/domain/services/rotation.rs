use crate::domain::entities::{ItemId, ClipboardItem};

#[allow(dead_code)] // Used in RepositoryBridge
pub struct RotationService;

#[allow(dead_code)] // Methods used in RepositoryBridge
impl RotationService {
    /// Determine which items should be deleted to maintain max_items limit
    /// Returns a list of item IDs to delete, excluding pinned items and prioritizing oldest
    pub fn items_to_delete(
        items: &[ClipboardItem], 
        max_items: usize
    ) -> Vec<ItemId> {
        let total_count = items.len();
        if total_count <= max_items {
            return Vec::new();
        }
        
        // Separate pinned and unpinned
        let pinned: Vec<_> = items.iter()
            .filter(|i| i.is_pinned)
            .collect();
        let unpinned: Vec<_> = items.iter()
            .filter(|i| !i.is_pinned)
            .collect();
        
        let pinned_count = pinned.len();
        let max_non_pinned = if pinned_count >= max_items {
            0
        } else {
            max_items - pinned_count
        };
        
        if unpinned.len() <= max_non_pinned {
            return Vec::new();
        }
        
        // Sort unpinned by last_used_at (oldest first for deletion)
        let mut unpinned_sorted: Vec<_> = unpinned.iter()
            .collect();
        unpinned_sorted.sort_by(|a, b| a.last_used_at.cmp(&b.last_used_at));
        
        // Take the oldest unpinned items that exceed the limit
        let to_delete_count = unpinned_sorted.len() - max_non_pinned;
        unpinned_sorted
            .iter()
            .take(to_delete_count)
            .map(|item| item.id)
            .collect()
    }
    
    pub fn should_rotate(total_count: usize, max_items: usize) -> bool {
        total_count > max_items
    }
    
    pub fn max_non_pinned_items(max_items: usize, pinned_count: usize) -> usize {
        if pinned_count >= max_items {
            0
        } else {
            max_items - pinned_count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType, Category, Timestamp};

    #[test]
    fn test_should_rotate() {
        assert!(!RotationService::should_rotate(10, 20));
        assert!(RotationService::should_rotate(25, 20));
    }

    #[test]
    fn test_max_non_pinned_items() {
        assert_eq!(RotationService::max_non_pinned_items(200, 0), 200);
        assert_eq!(RotationService::max_non_pinned_items(200, 10), 190);
        assert_eq!(RotationService::max_non_pinned_items(200, 200), 0);
    }

    #[test]
    fn test_items_to_delete_basic() {
        let items = vec![
            create_test_item(1, true, 100),
            create_test_item(2, false, 90),
            create_test_item(3, false, 80),
        ];
        
        // With max_items=2, only 1 unpinned should remain, so delete 1
        let to_delete = RotationService::items_to_delete(&items, 2);
        assert_eq!(to_delete.len(), 1);
        assert!(to_delete.contains(&ItemId::new(3))); // Delete oldest unpinned
    }

    #[test]
    fn test_items_to_delete_preserves_pinned() {
        let items = vec![
            create_test_item(1, true, 100), // pinned
            create_test_item(2, false, 90),
            create_test_item(3, false, 80),
            create_test_item(4, true, 70), // pinned
        ];
        
        let to_delete = RotationService::items_to_delete(&items, 2);
        // Both pinned should be preserved, only unpinned deleted
        assert!(!to_delete.contains(&ItemId::new(1)));
        assert!(!to_delete.contains(&ItemId::new(4)));
    }

    #[test]
    fn test_items_to_delete_no_rotation_needed() {
        let items = vec![
            create_test_item(1, false, 100),
            create_test_item(2, false, 90),
        ];
        
        let to_delete = RotationService::items_to_delete(&items, 5);
        assert!(to_delete.is_empty());
    }

    fn create_test_item(id: i64, is_pinned: bool, last_used: i64) -> ClipboardItem {
        ClipboardItem {
            id: ItemId::new(id),
            content: Content::Text(format!("Item {}", id)),
            mime_type: MimeType::new("text/plain".to_string()),
            category: Some(Category::Other),
            is_pinned,
            pin_order: 0,
            last_used_at: Timestamp::new(last_used),
        }
    }
}