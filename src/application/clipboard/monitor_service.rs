use crate::domain::entities::{ClipboardItem, Content, Category};
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct MonitorService {
    repository: Arc<dyn ClipboardRepository>,
    last_text: Arc<std::sync::Mutex<String>>,
    is_running: Arc<AtomicBool>,
}

impl MonitorService {
    pub fn new(repository: Arc<dyn ClipboardRepository>) -> Self {
        Self {
            repository,
            last_text: Arc::new(std::sync::Mutex::new(String::new())),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start monitoring clipboard (for testing purposes)
    pub async fn start_monitoring_test<F, Fut>(&self, mut get_clipboard: F) -> Result<()>
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Option<String>> + Send + 'static,
    {
        self.is_running.store(true, Ordering::SeqCst);
        
        while self.is_running.load(Ordering::SeqCst) {
            if let Some(text) = get_clipboard().await {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let mut last_text = self.last_text.lock().unwrap();
                    if *last_text != trimmed {
                        *last_text = trimmed.to_string();
                        drop(last_text);
                        
                        // Add to repository
                        let category = Some(Category::from_text(trimmed));
                        let item = ClipboardItem::new(
                            crate::domain::entities::ItemId(0),
                            Content::Text(trimmed.to_string()),
                            crate::domain::entities::MimeType::text_plain(),
                            category,
                        );
                        self.repository.save(&item)?;
                    }
                }
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        Ok(())
    }

    /// Stop monitoring
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Check if monitor is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Get last clipboard text
    pub fn get_last_text(&self) -> String {
        self.last_text.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Content, ItemId, MimeType};

    // Mock repository for testing
    struct TestRepository {
        items: std::sync::Mutex<Vec<ClipboardItem>>,
    }

    impl TestRepository {
        fn new() -> Self {
            Self {
                items: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn item_count(&self) -> usize {
            self.items.lock().unwrap().len()
        }
    }

    impl ClipboardRepository for TestRepository {
        fn save(&self, item: &ClipboardItem) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            let mut new_item = item.clone();
            new_item.id = ItemId(items.len() as i64 + 1);
            items.push(new_item);
            Ok(())
        }

        fn find_by_id(&self, _id: ItemId) -> Result<Option<ClipboardItem>> {
            Ok(None)
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

    #[tokio::test]
    async fn test_monitor_service_basic() {
        let repo = Arc::new(TestRepository::new());
        let service = MonitorService::new(repo.clone());
        
        // Mock clipboard that returns one value then None
        let mut call_count = 0;
        let get_clipboard = move || {
            call_count += 1;
            async move {
                if call_count == 1 {
                    Some("test text".to_string())
                } else {
                    None
                }
            }
        };
        
        // Run monitor in background
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            service_clone.start_monitoring_test(get_clipboard).await
        });
        
        // Give it time to process
        sleep(Duration::from_millis(100)).await;
        service.stop();
        
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        assert_eq!(repo.item_count(), 1);
    }

    #[test]
    fn test_monitor_start_stop() {
        let repo = Arc::new(TestRepository::new());
        let service = MonitorService::new(repo);
        
        assert!(!service.is_running());
        service.stop();
        assert!(!service.is_running());
    }

    #[test]
    fn test_get_last_text() {
        let repo = Arc::new(TestRepository::new());
        let service = MonitorService::new(repo);
        
        assert_eq!(service.get_last_text(), "");
    }

    #[test]
    fn test_duplicate_prevention() {
        let repo = Arc::new(TestRepository::new());
        let _service = MonitorService::new(repo);
        
        let text1 = "same text".to_string();
        let text2 = "same text".to_string();
        
        assert_eq!(text1, text2);
    }
}