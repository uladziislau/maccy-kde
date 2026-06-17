use crate::application::clipboard::{ItemManagementService, MonitorService};
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;

pub struct DaemonService {
    item_service: Arc<ItemManagementService>,
    monitor_service: Arc<MonitorService>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl DaemonService {
    pub fn new(repository: Arc<dyn ClipboardRepository>) -> Self {
        let item_service = Arc::new(ItemManagementService::new(repository.clone()));
        let monitor_service = Arc::new(MonitorService::new(repository));
        
        Self {
            item_service,
            monitor_service,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the daemon (simplified version for testing)
    pub async fn start(&self) -> Result<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // In a real implementation, this would:
        // 1. Start the clipboard monitor
        // 2. Start the IPC server
        // 3. Run until shutdown signal
        
        Ok(())
    }

    /// Stop the daemon
    pub fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        self.monitor_service.stop();
    }

    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get reference to item management service
    pub fn item_service(&self) -> &Arc<ItemManagementService> {
        &self.item_service
    }

    /// Get reference to monitor service
    pub fn monitor_service(&self) -> &Arc<MonitorService> {
        &self.monitor_service
    }

    /// Get clipboard statistics
    pub fn get_stats(&self) -> Result<DaemonStats> {
        let total_count = self.item_service.get_total_count()?;
        let pinned_count = self.item_service.get_pinned_items()?.len();
        
        Ok(DaemonStats {
            total_items: total_count,
            pinned_items: pinned_count,
            is_running: self.is_running(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DaemonStats {
    pub total_items: usize,
    pub pinned_items: usize,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType, Category};

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
    }

    impl ClipboardRepository for MockRepo {
        fn save(&self, item: &ClipboardItem) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            let mut new_item = item.clone();
            new_item.id = ItemId(items.len() as i64 + 1);
            items.push(new_item);
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

        fn find_recent(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.iter().rev().take(limit).cloned().collect())
        }

        fn find_pinned(&self) -> Result<Vec<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.iter().filter(|i| i.is_pinned).cloned().collect())
        }

        fn delete(&self, id: ItemId) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            items.retain(|i| i.id != id);
            Ok(())
        }

        fn update_pin(&self, id: ItemId, pinned: bool, order: i64) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.is_pinned = pinned;
                item.pin_order = order;
            }
            Ok(())
        }

        fn toggle_pin(&self, id: ItemId) -> Result<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.is_pinned = !item.is_pinned;
            }
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
    async fn test_daemon_service_creation() {
        let repo = Arc::new(MockRepo::new());
        let daemon = DaemonService::new(repo);
        
        assert!(!daemon.is_running());
        assert_eq!(daemon.get_stats().unwrap().total_items, 0);
    }

    #[test]
    fn test_daemon_start_stop() {
        let repo = Arc::new(MockRepo::new());
        let daemon = DaemonService::new(repo);
        
        assert!(!daemon.is_running());
        // Note: We can't easily test async start in sync context
        // but we can test the state management
        daemon.stop();
        assert!(!daemon.is_running());
    }

    #[test]
    fn test_daemon_item_service_access() {
        let repo = Arc::new(MockRepo::new());
        let daemon = DaemonService::new(repo);
        
        let item_service = daemon.item_service();
        let monitor_service = daemon.monitor_service();
        
        // Just verify we can access the services
        assert!(item_service.get_total_count().is_ok());
        assert!(!monitor_service.is_running());
    }

    #[test]
    fn test_daemon_stats() {
        let repo = Arc::new(MockRepo::new());
        let daemon = DaemonService::new(repo);
        
        let stats = daemon.get_stats().unwrap();
        assert_eq!(stats.total_items, 0);
        assert_eq!(stats.pinned_items, 0);
        assert!(!stats.is_running);
    }

    #[test]
    fn test_daemon_service_cloning() {
        let repo = Arc::new(MockRepo::new());
        let daemon = DaemonService::new(repo);
        
        // DaemonService itself is not Clone, but Arc<DaemonService> is
        let daemon_arc = Arc::new(daemon);
        assert!(Arc::strong_count(&daemon_arc) >= 1);
    }
}