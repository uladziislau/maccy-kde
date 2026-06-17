use rusqlite::Connection;
use std::path::PathBuf;
use crate::infrastructure::system::paths::AppPaths;
use crate::shared::Result as MaccyResult;
use crate::shared::MaccyError;

pub struct SqlConnection;

impl SqlConnection {
    pub fn open(path: PathBuf) -> MaccyResult<Connection> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MaccyError::Database(format!("Failed to create directory: {}", e)))?;
        }
        
        Connection::open(&path)
            .map_err(|e| MaccyError::Database(format!("Failed to open database: {}", e)))
    }
    
    pub fn open_default() -> MaccyResult<Connection> {
        let db_path = AppPaths::database_path();
        Self::open(db_path)
    }
    
    pub fn open_in_memory() -> MaccyResult<Connection> {
        Connection::open_in_memory()
            .map_err(|e| MaccyError::Database(format!("Failed to open in-memory database: {}", e)))
    }
    
    pub fn setup_schema(conn: &Connection) -> MaccyResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value_text TEXT,
                image_path TEXT,
                data_type TEXT NOT NULL,
                raw_mime_type TEXT NOT NULL,
                category TEXT DEFAULT NULL,
                is_pinned INTEGER DEFAULT 0,
                pin_order INTEGER DEFAULT 0,
                last_used_at INTEGER NOT NULL
            )",
            [],
        ).map_err(|e| MaccyError::Database(format!("Failed to create table: {}", e)))?;
        
        // Add category column if it doesn't exist (for existing databases)
        conn.execute(
            "ALTER TABLE clipboard_items ADD COLUMN category TEXT DEFAULT NULL",
            [],
        ).ok(); // Ignore error if column already exists
        
        Ok(())
    }
}