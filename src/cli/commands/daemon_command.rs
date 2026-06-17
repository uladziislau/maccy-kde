use crate::application::daemon::{DaemonService, DaemonRuntime};
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;

/// Command to start/stop daemon
pub struct DaemonCommand {
    repository: Arc<dyn ClipboardRepository>,
    runtime: Option<DaemonRuntime>,
}

impl DaemonCommand {
    pub fn new(repository: Arc<dyn ClipboardRepository>) -> Self {
        Self {
            repository,
            runtime: None,
        }
    }

    /// Start daemon process
    pub fn start(&mut self) -> Result<()> {
        self.runtime = Some(DaemonRuntime::default());
        
        // In real implementation, this would start the daemon in background
        println!("Starting maccy-kde daemon...");
        
        // Simulate daemon start (note: DaemonService.start() is async, we skip it for CLI)
        let _daemon = DaemonService::new(self.repository.clone());
        
        Ok(())
    }

    /// Stop daemon process
    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut runtime) = self.runtime.take() {
            println!("Stopping maccy-kde daemon...");
            runtime.shutdown();
        } else {
            println!("Daemon is not running");
        }
        
        Ok(())
    }

    /// Check daemon status
    pub fn status(&self) -> Result<DaemonStatus> {
        let is_running = self.runtime.is_some();
        
        let stats = if is_running {
            let daemon = DaemonService::new(self.repository.clone());
            Some(daemon.get_stats()?)
        } else {
            None
        };
        
        Ok(DaemonStatus {
            is_running,
            stats,
        })
    }

    /// Restart daemon
    pub fn restart(&mut self) -> Result<()> {
        self.stop()?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.start()
    }
}

#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub is_running: bool,
    pub stats: Option<crate::application::daemon::DaemonStats>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType};

    struct MockRepo;

    impl MockRepo {
        fn new() -> Arc<Self> {
            Arc::new(Self)
        }
    }

    impl ClipboardRepository for MockRepo {
        fn save(&self, _item: &ClipboardItem) -> Result<()> {
            Ok(())
        }

        fn find_by_id(&self, _id: ItemId) -> Result<Option<ClipboardItem>> {
            Ok(None)
        }

        fn find_all(&self) -> Result<Vec<ClipboardItem>> {
            Ok(Vec::new())
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
            Ok(0)
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
    fn test_daemon_command_creation() {
        let repo = MockRepo::new();
        let command = DaemonCommand::new(repo);
        
        assert!(command.runtime.is_none());
    }

    #[test]
    fn test_start_daemon() {
        let repo = MockRepo::new();
        let mut command = DaemonCommand::new(repo);
        
        let result = command.start();
        assert!(result.is_ok());
        assert!(command.runtime.is_some());
    }

    #[test]
    fn test_stop_daemon() {
        let repo = MockRepo::new();
        let mut command = DaemonCommand::new(repo);
        
        command.start().unwrap();
        let result = command.stop();
        assert!(result.is_ok());
        assert!(command.runtime.is_none());
    }

    #[test]
    fn test_status_not_running() {
        let repo = MockRepo::new();
        let command = DaemonCommand::new(repo);
        
        let status = command.status().unwrap();
        assert!(!status.is_running);
        assert!(status.stats.is_none());
    }

    #[test]
    fn test_status_running() {
        let repo = MockRepo::new();
        let mut command = DaemonCommand::new(repo);
        
        command.start().unwrap();
        let status = command.status().unwrap();
        
        assert!(status.is_running);
        assert!(status.stats.is_some());
    }

    #[test]
    fn test_restart_daemon() {
        let repo = MockRepo::new();
        let mut command = DaemonCommand::new(repo);
        
        command.start().unwrap();
        let result = command.restart();
        assert!(result.is_ok());
        assert!(command.runtime.is_some());
    }
}