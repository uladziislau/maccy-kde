use super::item_view_model::ItemViewModel;
use std::collections::VecDeque;

/// State model for popup UI
#[derive(Debug, Clone)]
pub struct PopupState {
    /// Current search query
    pub query: String,
    /// Displayed items
    pub items: Vec<ItemViewModel>,
    /// Currently selected item index
    pub selected_index: usize,
    /// Maximum items to display
    pub max_items: usize,
    /// History of recent queries
    pub query_history: VecDeque<String>,
    /// Max history size
    pub max_history: usize,
}

impl PopupState {
    pub fn new(max_items: usize, max_history: usize) -> Self {
        Self {
            query: String::new(),
            items: Vec::new(),
            selected_index: 0,
            max_items,
            query_history: VecDeque::new(),
            max_history,
        }
    }

    /// Update search query
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = 0;
    }

    /// Update displayed items
    pub fn set_items(&mut self, items: Vec<ItemViewModel>) {
        self.items = items;
        if self.selected_index >= self.items.len() {
            self.selected_index = self.items.len().saturating_sub(1);
        }
    }

    /// Move selection up
    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Get selected item
    pub fn get_selected_item(&self) -> Option<&ItemViewModel> {
        self.items.get(self.selected_index)
    }

    /// Get selected item ID
    pub fn get_selected_id(&self) -> Option<String> {
        self.get_selected_item().map(|item| item.id.clone())
    }

    /// Check if any item is selected
    pub fn has_selection(&self) -> bool {
        !self.items.is_empty() && self.selected_index < self.items.len()
    }

    /// Reset selection to first item
    pub fn reset_selection(&mut self) {
        self.selected_index = 0;
    }

    /// Add query to history
    pub fn add_to_history(&mut self, query: String) {
        if query.is_empty() {
            return;
        }
        
        // Remove if already exists
        if let Some(pos) = self.query_history.iter().position(|q| q == &query) {
            self.query_history.remove(pos);
        }
        
        // Add to front
        self.query_history.push_front(query);
        
        // Trim if exceeds max
        if self.query_history.len() > self.max_history {
            self.query_history.pop_back();
        }
    }

    /// Get previous query from history
    pub fn get_previous_query(&self) -> Option<&String> {
        self.query_history.iter().next()
    }

    /// Clear query
    pub fn clear_query(&mut self) {
        self.query.clear();
        self.selected_index = 0;
    }

    /// Check if popup is empty (no items)
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get item count
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Get visible items (respecting max_items)
    pub fn get_visible_items(&self) -> Vec<&ItemViewModel> {
        self.items.iter().take(self.max_items).collect()
    }

    /// Set selected index directly
    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = index;
        }
    }
}

impl Default for PopupState {
    fn default() -> Self {
        Self::new(100, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType};

    fn create_mock_view_model(id: i64, text: &str) -> ItemViewModel {
        let item = ClipboardItem::new(
            ItemId(id),
            Content::Text(text.to_string()),
            MimeType::text_plain(),
            None,
        );
        ItemViewModel::from_domain(&item)
    }

    #[test]
    fn test_popup_state_creation() {
        let state = PopupState::new(50, 5);
        assert_eq!(state.max_items, 50);
        assert_eq!(state.max_history, 5);
        assert_eq!(state.selected_index, 0);
        assert!(state.items.is_empty());
    }

    #[test]
    fn test_set_query() {
        let mut state = PopupState::default();
        state.set_query("test".to_string());
        assert_eq!(state.query, "test");
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_set_items() {
        let mut state = PopupState::default();
        let items = vec![
            create_mock_view_model(1, "item1"),
            create_mock_view_model(2, "item2"),
        ];
        state.set_items(items);
        
        assert_eq!(state.item_count(), 2);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_move_selection_down() {
        let mut state = PopupState::default();
        let items = vec![
            create_mock_view_model(1, "item1"),
            create_mock_view_model(2, "item2"),
        ];
        state.set_items(items);
        
        state.move_selection_down();
        assert_eq!(state.selected_index, 1);
        
        // Can't go beyond last item
        state.move_selection_down();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_move_selection_up() {
        let mut state = PopupState::default();
        let items = vec![
            create_mock_view_model(1, "item1"),
            create_mock_view_model(2, "item2"),
        ];
        state.set_items(items);
        state.set_selected_index(1);
        
        state.move_selection_up();
        assert_eq!(state.selected_index, 0);
        
        // Can't go below first item
        state.move_selection_up();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_get_selected_item() {
        let mut state = PopupState::default();
        let items = vec![
            create_mock_view_model(1, "item1"),
            create_mock_view_model(2, "item2"),
        ];
        state.set_items(items);
        
        let selected = state.get_selected_item();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, "1");
    }

    #[test]
    fn test_get_selected_item_none() {
        let state = PopupState::default();
        let selected = state.get_selected_item();
        assert!(selected.is_none());
    }

    #[test]
    fn test_has_selection() {
        let mut state = PopupState::default();
        assert!(!state.has_selection());
        
        let items = vec![create_mock_view_model(1, "item1")];
        state.set_items(items);
        assert!(state.has_selection());
    }

    #[test]
    fn test_add_to_history() {
        let mut state = PopupState::default();
        state.add_to_history("query1".to_string());
        state.add_to_history("query2".to_string());
        
        assert_eq!(state.query_history.len(), 2);
        assert_eq!(state.get_previous_query(), Some(&"query2".to_string()));
    }

    #[test]
    fn test_add_duplicate_to_history() {
        let mut state = PopupState::default();
        state.add_to_history("query".to_string());
        state.add_to_history("query".to_string());
        
        assert_eq!(state.query_history.len(), 1);
    }

    #[test]
    fn test_history_max_size() {
        let mut state = PopupState::default();
        state.max_history = 3;
        
        for i in 0..5 {
            state.add_to_history(format!("query{}", i));
        }
        
        assert_eq!(state.query_history.len(), 3);
    }

    #[test]
    fn test_clear_query() {
        let mut state = PopupState::default();
        state.set_query("test".to_string());
        state.clear_query();
        
        assert_eq!(state.query, "");
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_is_empty() {
        let state = PopupState::default();
        assert!(state.is_empty());
    }

    #[test]
    fn test_get_visible_items() {
        let mut state = PopupState::default();
        state.max_items = 2;
        
        let items = vec![
            create_mock_view_model(1, "item1"),
            create_mock_view_model(2, "item2"),
            create_mock_view_model(3, "item3"),
        ];
        state.set_items(items);
        
        let visible = state.get_visible_items();
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_set_selected_index() {
        let mut state = PopupState::default();
        let items = vec![
            create_mock_view_model(1, "item1"),
            create_mock_view_model(2, "item2"),
        ];
        state.set_items(items);
        
        state.set_selected_index(1);
        assert_eq!(state.selected_index, 1);
        
        // Invalid index ignored
        state.set_selected_index(5);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_clone() {
        let mut state = PopupState::default();
        state.set_query("test".to_string());
        let cloned = state.clone();
        
        assert_eq!(state.query, cloned.query);
    }
}