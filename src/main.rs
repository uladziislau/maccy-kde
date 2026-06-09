mod database;
mod clipboard;

use database::Database;
use log::{info, error};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting maccy-kde...");

    // Initialize the database
    let db = match Database::new() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    // Start background clipboard monitor
    clipboard::start_clipboard_monitor(db.clone()).await;

    // TODO: Initialize Slint UI and Global Hotkeys
    
    // For now, loop forever so the monitor can run
    info!("Background daemon is running...");
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
