use crate::presentation::{PresentationService, PresentationStats};
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;

/// Command to show/hide popup
pub struct PopupCommand {
    presentation_service: Option<PresentationService>,
    repository: Arc<dyn ClipboardRepository>,
    max_items: usize,
}

impl PopupCommand {
    pub fn new(repository: Arc<dyn ClipboardRepository>, max_items: usize) -> Self {
        Self {
            presentation_service: None,
            repository,
            max_items,
        }
    }

    /// Show popup with clipboard history
    pub fn show(&mut self) -> Result<PopupResult> {
        self.presentation_service = Some(PresentationService::new(
            self.repository.clone(),
            self.max_items,
        ));
        
        let service = self.presentation_service.as_mut().unwrap();
        service.refresh()?;
        
        let stats = service.get_stats();
        
        Ok(PopupResult {
            shown: true,
            item_count: stats.total_items,
            has_selection: stats.has_selection,
        })
    }

    /// Hide popup
    pub fn hide(&mut self) -> Result<PopupResult> {
        self.presentation_service = None;
        
        Ok(PopupResult {
            shown: false,
            item_count: 0,
            has_selection: false,
        })
    }

    /// Move selection in popup
    pub fn move_selection(&mut self, direction: SelectionDirection) -> Result<PopupResult> {
        if let Some(service) = &mut self.presentation_service {
            match direction {
                SelectionDirection::Up => service.move_selection_up(),
                SelectionDirection::Down => service.move_selection_down(),
            }
            
            let stats = service.get_stats();
            Ok(PopupResult {
                shown: true,
                item_count: stats.total_items,
                has_selection: stats.has_selection,
            })
        } else {
            Err(crate::shared::MaccyError::Validation(
                "Popup is not shown".to_string()
            ))
        }
    }

    /// Select item and paste it
    pub fn select_and_paste(&self) -> Result<Option<String>> {
        if let Some(service) = &self.presentation_service {
            service.get_selected_for_paste()
        } else {
            Err(crate::shared::MaccyError::Validation(
                "Popup is not shown".to_string()
            ))
        }
    }

    /// Search in popup
    pub fn search(&mut self, query: String) -> Result<PopupResult> {
        if let Some(service) = &mut self.presentation_service {
            service.set_search_query(query)?;
            
            let stats = service.get_stats();
            Ok(PopupResult {
                shown: true,
                item_count: stats.total_items,
                has_selection: stats.has_selection,
            })
        } else {
            Err(crate::shared::MaccyError::Validation(
                "Popup is not shown".to_string()
            ))
        }
    }

    /// Get popup statistics
    pub fn get_stats(&self) -> Result<Option<PresentationStats>> {
        if let Some(service) = &self.presentation_service {
            Ok(Some(service.get_stats()))
        } else {
            Ok(None)
        }
    }

    /// Check if popup is shown
    pub fn is_shown(&self) -> bool {
        self.presentation_service.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionDirection {
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub struct PopupResult {
    pub shown: bool,
    pub item_count: usize,
    pub has_selection: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType};

    struct MockRepo {
        items: std::sync::Mutex<Vec<ClipboardItem>>,
    }

    impl MockRepo {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                items: std::sync::Mutex::new(Vec::new()),
            })
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
    fn test_popup_command_creation() {
        let repo = MockRepo::new();
        let command = PopupCommand::new(repo, 100);
        
        assert!(!command.is_shown());
    }

    #[test]
    fn test_show_popup() {
        let repo = MockRepo::new();
        repo.add_item(1, "test item");
        
        let mut command = PopupCommand::new(repo, 100);
        let result = command.show().unwrap();
        
        assert!(result.shown);
        assert!(result.item_count >= 1);
    }

    #[test]
    fn test_hide_popup() {
        let repo = MockRepo::new();
        let mut command = PopupCommand::new(repo, 100);
        
        command.show().unwrap();
        assert!(command.is_shown());
        
        let result = command.hide().unwrap();
        assert!(!result.shown);
        assert!(!command.is_shown());
    }

    #[test]
    fn test_move_selection() {
        let repo = MockRepo::new();
        repo.add_item(1, "item1");
        repo.add_item(2, "item2");
        
        let mut command = PopupCommand::new(repo, 100);
        command.show().unwrap();
        
        let result = command.move_selection(SelectionDirection::Down).unwrap();
        assert!(result.shown);
    }

    #[test]
    fn test_move_selection_not_shown() {
        let repo = MockRepo::new();
        let mut command = PopupCommand::new(repo, 100);
        
        let result = command.move_selection(SelectionDirection::Down);
        assert!(result.is_err());
    }

    #[test]
    fn test_select_and_paste() {
        let repo = MockRepo::new();
        repo.add_item(1, "paste me");
        
        let mut command = PopupCommand::new(repo, 100);
        command.show().unwrap();
        
        let content = command.select_and_paste().unwrap();
        assert_eq!(content, Some("paste me".to_string()));
    }

    #[test]
    fn test_search() {
        let repo = MockRepo::new();
        repo.add_item(1, "hello world");
        repo.add_item(2, "goodbye world");
        
        let mut command = PopupCommand::new(repo, 100);
        command.show().unwrap();
        
        let result = command.search("hello".to_string()).unwrap();
        assert!(result.shown);
        assert!(result.item_count >= 1);
    }

    #[test]
    fn test_get_stats() {
        let repo = MockRepo::new();
        repo.add_item(1, "item");
        
        let mut command = PopupCommand::new(repo, 100);
        command.show().unwrap();
        
        let stats = command.get_stats().unwrap();
        assert!(stats.is_some());
    }

    #[test]
    fn test_get_stats_not_shown() {
        let repo = MockRepo::new();
        let command = PopupCommand::new(repo, 100);
        
        let stats = command.get_stats().unwrap();
        assert!(stats.is_none());
    }
}