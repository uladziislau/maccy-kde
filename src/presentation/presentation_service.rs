use crate::application::popup::PopupService;
use crate::application::search::FuzzySearchService;
use crate::domain::repositories::ClipboardRepository;
use crate::presentation::models::{ItemViewModel, PopupState};
use crate::shared::Result;
use std::sync::Arc;

/// Service for managing presentation layer logic
pub struct PresentationService {
    popup_service: Arc<PopupService>,
    search_service: FuzzySearchService,
    state: PopupState,
}

impl PresentationService {
    pub fn new(repository: Arc<dyn ClipboardRepository>, max_items: usize) -> Self {
        let popup_service = Arc::new(PopupService::new(repository.clone(), max_items));
        let search_service = FuzzySearchService::new();
        let state = PopupState::new(max_items, 10);
        
        Self {
            popup_service,
            search_service,
            state,
        }
    }

    /// Update popup state with current data
    pub fn update_state(&mut self) -> Result<()> {
        let query = if self.state.query.is_empty() {
            None
        } else {
            Some(self.state.query.as_str())
        };
        
        let items = self.popup_service.get_display_items(query)?;
        let view_models: Vec<ItemViewModel> = items
            .iter()
            .map(ItemViewModel::from_domain)
            .collect();
        
        self.state.set_items(view_models);
        Ok(())
    }

    /// Set search query and update state
    pub fn set_search_query(&mut self, query: String) -> Result<()> {
        self.state.set_query(query);
        self.update_state()
    }

    /// Move selection up
    pub fn move_selection_up(&mut self) {
        self.state.move_selection_up();
    }

    /// Move selection down
    pub fn move_selection_down(&mut self) {
        self.state.move_selection_down();
    }

    /// Get current state
    pub fn get_state(&self) -> &PopupState {
        &self.state
    }

    /// Get mutable state reference
    pub fn get_state_mut(&mut self) -> &mut PopupState {
        &mut self.state
    }

    /// Get selected item for paste
    pub fn get_selected_for_paste(&self) -> Result<Option<String>> {
        if let Some(id) = self.state.get_selected_id() {
            let item_id = self.parse_item_id(&id)?;
            self.popup_service.get_item_for_paste(item_id)
        } else {
            Ok(None)
        }
    }

    /// Clear search and reset state
    pub fn clear_search(&mut self) -> Result<()> {
        if !self.state.query.is_empty() {
            self.state.add_to_history(self.state.query.clone());
        }
        self.state.clear_query();
        self.update_state()
    }

    /// Add current query to history
    pub fn add_query_to_history(&mut self) {
        if !self.state.query.is_empty() {
            self.state.add_to_history(self.state.query.clone());
        }
    }

    /// Get previous query from history
    pub fn get_previous_query(&self) -> Option<String> {
        self.state.get_previous_query().cloned()
    }

    /// Get popup statistics
    pub fn get_stats(&self) -> PresentationStats {
        PresentationStats {
            total_items: self.state.item_count(),
            selected_index: self.state.selected_index,
            has_selection: self.state.has_selection(),
            query_length: self.state.query.len(),
        }
    }

    /// Parse item ID from string
    fn parse_item_id(&self, id_str: &str) -> Result<crate::domain::entities::ItemId> {
        id_str.parse::<i64>()
            .map(crate::domain::entities::ItemId)
            .map_err(|e| crate::shared::MaccyError::Database(format!("Invalid item ID: {}", e)))
    }

    /// Refresh data from repository
    pub fn refresh(&mut self) -> Result<()> {
        self.update_state()
    }
}

#[derive(Debug, Clone)]
pub struct PresentationStats {
    pub total_items: usize,
    pub selected_index: usize,
    pub has_selection: bool,
    pub query_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType};

    // Mock repository
    struct MockRepo {
        items: std::sync::Mutex<Vec<ClipboardItem>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                items: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn add_item(&self, id: i64, text: &str) {
            let mut items = self.items.lock().unwrap();
            items.push(ClipboardItem::new(
                ItemId(id),
                Content::Text(text.to_string()),
                MimeType::text_plain(),
                None,
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
            Ok(Vec::new())
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
    fn test_presentation_service_creation() {
        let repo = Arc::new(MockRepo::new());
        let service = PresentationService::new(repo, 100);
        
        assert_eq!(service.get_state().max_items, 100);
        assert_eq!(service.get_state().item_count(), 0);
    }

    #[test]
    fn test_update_state() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "test item");
        
        let mut service = PresentationService::new(repo, 100);
        service.update_state().unwrap();
        
        assert_eq!(service.get_state().item_count(), 1);
    }

    #[test]
    fn test_set_search_query() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "hello world");
        repo.add_item(2, "goodbye world");
        
        let mut service = PresentationService::new(repo, 100);
        service.set_search_query("hello".to_string()).unwrap();
        
        assert_eq!(service.get_state().query, "hello");
        assert_eq!(service.get_state().item_count(), 1);
    }

    #[test]
    fn test_move_selection() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "item1");
        repo.add_item(2, "item2");
        
        let mut service = PresentationService::new(repo, 100);
        service.update_state().unwrap();
        
        service.move_selection_down();
        assert_eq!(service.get_state().selected_index, 1);
        
        service.move_selection_up();
        assert_eq!(service.get_state().selected_index, 0);
    }

    #[test]
    fn test_get_selected_for_paste() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "paste me");
        
        let mut service = PresentationService::new(repo, 100);
        service.update_state().unwrap();
        
        let content = service.get_selected_for_paste().unwrap();
        assert_eq!(content, Some("paste me".to_string()));
    }

    #[test]
    fn test_clear_search() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "item1");
        repo.add_item(2, "item2");
        
        let mut service = PresentationService::new(repo, 100);
        service.set_search_query("test".to_string()).unwrap();
        service.clear_search().unwrap();
        
        assert_eq!(service.get_state().query, "");
        assert_eq!(service.get_state().selected_index, 0);
    }

    #[test]
    fn test_add_query_to_history() {
        let repo = Arc::new(MockRepo::new());
        let mut service = PresentationService::new(repo, 100);
        
        service.set_search_query("query1".to_string()).unwrap();
        service.add_query_to_history();
        
        let history = service.get_previous_query();
        assert_eq!(history, Some("query1".to_string()));
    }

    #[test]
    fn test_get_stats() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "item1");
        
        let mut service = PresentationService::new(repo, 100);
        service.update_state().unwrap();
        
        let stats = service.get_stats();
        assert_eq!(stats.total_items, 1);
        assert!(stats.has_selection);
    }

    #[test]
    fn test_refresh() {
        let repo = Arc::new(MockRepo::new());
        repo.add_item(1, "item1");
        
        let mut service = PresentationService::new(repo, 100);
        service.refresh().unwrap();
        
        assert_eq!(service.get_state().item_count(), 1);
    }
}