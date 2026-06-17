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
        let daemon = Arc::new(DaemonService::new(repository));
        
        // Start daemon (monitor clipboard)
        if let Err(e) = daemon.start().await {
            error!("Failed to start daemon: {}", e);
            return;
        }
        
        // Start IPC server with legacy database for now
        // TODO: Migrate IPC to use new architecture
        if let Err(e) = ipc::start_ipc_server(db).await {
            error!("Failed to start IPC server: {}", e);
        }
        
        // Keep runtime alive
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });
}