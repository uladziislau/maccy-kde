use crate::domain::entities::ItemId;

pub struct RotationService;

impl RotationService {
    /// Determine which items should be deleted to maintain max_items limit
    /// Returns a list of item IDs to delete, excluding pinned items
    pub fn items_to_delete(
        total_count: usize, 
        max_items: usize, 
        pinned_count: usize
    ) -> Vec<ItemId> {
        if total_count <= max_items {
            return Vec::new();
        }
        
        let non_pinned_count = total_count - pinned_count;
        if non_pinned_count <= (max_items - pinned_count) {
            return Vec::new();
        }
        
        // In a real implementation, this would query the actual items
        // For now, return empty vector - the actual deletion logic
        // will be handled by the repository
        Vec::new()
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
}