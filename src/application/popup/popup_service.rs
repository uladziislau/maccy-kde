use crate::domain::entities::{ClipboardItem, ItemId, Category};
use crate::application::search::FuzzySearchService;
use crate::application::search::CategoryFilterService;
use crate::application::category::CategoryService;
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;

pub struct PopupService {
    repository: Arc<dyn ClipboardRepository>,
    search_service: FuzzySearchService,
    max_items: usize,
}

impl PopupService {
    pub fn new(repository: Arc<dyn ClipboardRepository>, max_items: usize) -> Self {
        Self {
            repository,
            search_service: FuzzySearchService::new(),
            max_items,
        }
    }

    /// Get items for popup display with optional search query
    pub fn get_display_items(&self, query: Option<&str>) -> Result<Vec<ClipboardItem>> {
        let all_items = self.repository.find_all()?;
        
        let filtered: Vec<ClipboardItem> = if let Some(q) = query {
            self.search_service.search(&all_items, q).into_iter().cloned().collect()
        } else {
            all_items.clone()
        };
        
        // Separate pinned and unpinned (now owned items)
        let mut pinned: Vec<ClipboardItem> = filtered.iter()
            .filter(|i| i.pinned())
            .cloned()
            .collect();
        let mut unpinned: Vec<ClipboardItem> = filtered.iter()
            .filter(|i| !i.pinned())
            .cloned()
            .collect();
        
        // Auto-detect categories for items without them
        CategoryService::detect_for_items(&mut pinned);
        CategoryService::detect_for_items(&mut unpinned);
        
        // Sort pinned by pin_order
        pinned.sort_by_key(|i| i.pin_order);
        // Sort unpinned by last_used (most recent first)
        unpinned.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        
        // Combine and limit
        let mut result: Vec<ClipboardItem> = pinned;
        result.extend(unpinned);
        
        if result.len() > self.max_items {
            result.truncate(self.max_items);
        }
        
        Ok(result)
    }

    /// Get item by index from display list
    pub fn get_item_at_index(&self, index: usize, query: Option<&str>) -> Result<Option<ClipboardItem>> {
        let items = self.get_display_items(query)?;
        Ok(items.get(index).cloned())
    }

    /// Get selected item content for pasting
    pub fn get_item_for_paste(&self, item_id: ItemId) -> Result<Option<String>> {
        if let Some(item) = self.repository.find_by_id(item_id)? {
            match item.content {
                crate::domain::entities::Content::Text(text) => Ok(Some(text)),
                crate::domain::entities::Content::Image(_) => Ok(Some("[Image]".to_string())),
            }
        } else {
            Ok(None)
        }
    }

    /// Get display text for item (truncated if necessary)
    pub fn get_display_text(&self, item_id: ItemId, max_length: usize) -> Result<Option<String>> {
        if let Some(item) = self.repository.find_by_id(item_id)? {
            let text = item.display_text();
            if text.len() > max_length {
                Ok(Some(format!("{}...", &text[..max_length])))
            } else {
                Ok(Some(text))
            }
        } else {
            Ok(None)
        }
    }

    /// Get category badge for item
    pub fn get_category_badge(&self, item_id: ItemId) -> Result<Option<String>> {
        if let Some(item) = self.repository.find_by_id(item_id)? {
            if let Some(category) = item.category {
                Ok(Some(match category {
                    Category::Url => "🔗".to_string(),
                    Category::Email => "📧".to_string(),
                    Category::Account => "👤".to_string(),
                    Category::Picture => "🖼️".to_string(),
                    Category::Other => "📝".to_string(),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Check if item is pinned
    pub fn is_item_pinned(&self, item_id: ItemId) -> Result<bool> {
        if let Some(item) = self.repository.find_by_id(item_id)? {
            Ok(item.pinned())
        } else {
            Ok(false)
        }
    }

    /// Get total display count
    pub fn get_display_count(&self, query: Option<&str>) -> Result<usize> {
        self.get_display_items(query).map(|items| items.len())
    }

    /// Get recommended items based on usage
    pub fn get_recommended_items(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
        let all_items = self.repository.find_all()?;
        
        // Sort by usage count (simulated by recency for now)
        let mut items = all_items;
        items.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        
        Ok(items.into_iter().take(limit).collect())
    }

    /// Get items filtered by category
    pub fn get_items_by_category(&self, category: &Category) -> Result<Vec<ClipboardItem>> {
        let all_items = self.repository.find_all()?;
        let filtered = CategoryFilterService::filter_by_category(&all_items, category);
        Ok(filtered.into_iter().cloned().collect())
    }

    /// Count items by category
    pub fn count_by_category(&self, category: &Category) -> Result<usize> {
        let all_items = self.repository.find_all()?;
        Ok(CategoryFilterService::count_by_category(&all_items, category))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Content, ItemId, MimeType, Category, Timestamp};

    // Mock repository for testing
    struct MockRepo {
        items: std::sync::Mutex<Vec<ClipboardItem>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                items: std::sync::Mutex::new(Vec::new()),
            }
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
        fn save(&self, _item: &ClipboardItem) -> Result<()> {
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

        fn find_recent(&self, _limit: usize) -> Result<Vec<ClipboardItem>> {
            Ok(Vec::new())
        }

        fn find_pinned(&self) -> Result<Vec<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.iter().filter(|i| i.is_pinned).cloned().collect())
        }

        fn delete(&self, _id: ItemId) -> Result<()> {
            Ok(())
        }

        fn update_pin(&self, _id: ItemId, _pinned: bool, _order: i64) -> Result<()> {
            Ok(())
        }

        fn toggle_pin(&self, _id: ItemId) -> Result<()> {
            Ok(())
        }

        fn count(&self) -> Result<usize> {
            let items = self.items.lock().unwrap();
            Ok(items.len())
        }

        fn rotate_history(&self, _max_items: usize) -> Result<()> {
            Ok(())
        }

        fn find_by_content(&self, _content: &str) -> Result<Option<ClipboardItem>> {
            Ok(None)
        }

        fn update_last_used(&self, _id: ItemId) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_popup_service_creation() {
        let repo = Arc::new(MockRepo::new());
        let service = PopupService::new(repo, 100);
        assert_eq!(service.max_items, 100);
    }

    #[test]
    fn test_get_display_items_empty() {
        let repo = Arc::new(MockRepo::new());
        let service = PopupService::new(repo, 100);
        
        let items = service.get_display_items(None).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_get_display_items() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "item1", None);
        repo.add_item(2, "item2", None);
        
        let service = PopupService::new(repo, 100);
        let items = service.get_display_items(None).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_get_display_items_with_search() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "hello world", None);
        repo.add_item(2, "goodbye", None);
        
        let service = PopupService::new(repo, 100);
        let items = service.get_display_items(Some("hello")).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].display_text().contains("hello"));
    }

    #[test]
    fn test_get_item_at_index() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "first", None);
        repo.add_item(2, "second", None);
        
        let service = PopupService::new(repo, 100);
        let item = service.get_item_at_index(0, None).unwrap();
        assert!(item.is_some());
    }

    #[test]
    fn test_get_item_for_paste_text() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "test text", None);
        
        let service = PopupService::new(repo, 100);
        let content = service.get_item_for_paste(ItemId(1)).unwrap();
        assert_eq!(content, Some("test text".to_string()));
    }

    #[test]
    fn test_get_display_text() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "short text", None);
        
        let service = PopupService::new(repo, 100);
        let text = service.get_display_text(ItemId(1), 10).unwrap();
        assert_eq!(text, Some("short text".to_string()));
    }

    #[test]
    fn test_get_display_text_truncated() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "this is a very long text that should be truncated", None);
        
        let service = PopupService::new(repo, 100);
        let text = service.get_display_text(ItemId(1), 10).unwrap();
        assert!(text.unwrap().ends_with("..."));
    }

    #[test]
    fn test_get_category_badge() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "https://example.com", Some(Category::Url));
        
        let service = PopupService::new(repo, 100);
        let badge = service.get_category_badge(ItemId(1)).unwrap();
        assert_eq!(badge, Some("🔗".to_string()));
    }

    #[test]
    fn test_get_category_badge_none() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "plain text", None);
        
        let service = PopupService::new(repo, 100);
        let badge = service.get_category_badge(ItemId(1)).unwrap();
        assert_eq!(badge, None);
    }

    #[test]
    fn test_is_item_pinned() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "test", None);
        
        let service = PopupService::new(repo, 100);
        let is_pinned = service.is_item_pinned(ItemId(1)).unwrap();
        assert!(!is_pinned);
    }

    #[test]
    fn test_get_display_count() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "item1", None);
        repo.add_item(2, "item2", None);
        
        let service = PopupService::new(repo, 100);
        let count = service.get_display_count(None).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_items_by_category() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "https://example.com", Some(Category::Url));
        repo.add_item(2, "https://github.com", Some(Category::Url));
        repo.add_item(3, "user@example.com", Some(Category::Email));
        repo.add_item(4, "plain text", Some(Category::Other));
        
        let service = PopupService::new(repo, 100);
        let url_items = service.get_items_by_category(&Category::Url).unwrap();
        assert_eq!(url_items.len(), 2);
        
        let email_items = service.get_items_by_category(&Category::Email).unwrap();
        assert_eq!(email_items.len(), 1);
    }

    #[test]
    fn test_count_by_category() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "https://example.com", Some(Category::Url));
        repo.add_item(2, "https://github.com", Some(Category::Url));
        repo.add_item(3, "user@example.com", Some(Category::Email));
        
        let service = PopupService::new(repo, 100);
        let url_count = service.count_by_category(&Category::Url).unwrap();
        assert_eq!(url_count, 2);
        
        let email_count = service.count_by_category(&Category::Email).unwrap();
        assert_eq!(email_count, 1);
        
        let account_count = service.count_by_category(&Category::Account).unwrap();
        assert_eq!(account_count, 0);
    }

    #[test]
    fn test_max_items_limit() {
        let repo = Arc::new(MockRepo::new());
        for i in 1..=150 {
            repo.add_item(i, &format!("item{}", i), None);
        }
        
        let service = PopupService::new(repo, 50);
        let items = service.get_display_items(None).unwrap();
        assert!(items.len() <= 50);
    }
}