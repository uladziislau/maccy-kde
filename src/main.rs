mod database;

use database::Database;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("Starting maccy-kde...");

    // Initialize the database
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    // Add a test item
    if let Err(e) = db.add_item("Hello from Maccy-KDE!") {
        error!("Failed to add test item: {}", e);
    } else {
        info!("Successfully added test item to the database.");
    }

    // Print current history
    match db.get_history() {
        Ok(history) => {
            info!("--- Current Clipboard History ---");
            for item in history {
                info!(
                    "[{}] {} (Pinned: {})",
                    item.id,
                    item.value_text,
                    item.is_pinned
                );
            }
            info!("---------------------------------");
        }
        Err(e) => {
            error!("Failed to get history: {}", e);
        }
    }
}
