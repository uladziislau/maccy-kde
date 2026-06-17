/// Daemon module - background daemon process management
/// Uses new Clean Architecture with DaemonService

use log::{error, info};
use std::sync::Arc;
use crate::application::daemon::{DaemonService, DaemonRuntime};
use crate::integration::RepositoryBridge;
use crate::database::Database;
use crate::ipc;

pub fn run() {
    info!("Starting daemon with new Clean Architecture...");
    
    let rt = match DaemonRuntime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create daemon runtime: {}", e);
            return;
        }
    };
    
    rt.block_on(async {
        // Initialize database and create repository bridge
        let db = match Database::new() {
            Ok(db) => Arc::new(db),
            Err(e) => {
                error!("Failed to initialize database: {}", e);
                return;
            }
        };
        
        let repository = Arc::new(RepositoryBridge::new(db.clone()));
        
        // Create daemon service
        let daemon = Arc::new(DaemonService::new(repository.clone()));
        
        // Get reference to services for advanced operations
        let item_service = daemon.item_service().clone();
        let _monitor_service = daemon.monitor_service().clone(); // Reserved for future clipboard monitoring
        
        // Start daemon (monitor clipboard)
        if let Err(e) = daemon.start().await {
            error!("Failed to start daemon: {}", e);
            return;
        }
        
        // Start clipboard monitoring through MonitorService
        // Note: This is a stub - real clipboard monitoring would need actual clipboard access
        // For now, the legacy clipboard.rs handles the real monitoring
        info!("Clipboard monitor service initialized (legacy clipboard.rs handles real monitoring)");
        
        // Example of using ItemManagementService for operations
        if let Ok(count) = item_service.get_total_count() {
            info!("Current clipboard item count: {}", count);
        }
        
        // Use ItemManagementService to get recent items for stats
        if let Ok(recent_items) = item_service.get_recent_items(5) {
            info!("Recent items: {}", recent_items.len());
        }
        
        // Start IPC server with legacy database for now
        // TODO: Migrate IPC to use new architecture (item_service)
        if let Err(e) = ipc::start_ipc_server(db).await {
            error!("Failed to start IPC server: {}", e);
        }
        
        // Keep runtime alive
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });
}