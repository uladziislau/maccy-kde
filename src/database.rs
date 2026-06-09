use rusqlite::{params, Connection, OptionalExtension, Result};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use std::sync::Mutex;

#[derive(Debug, PartialEq)]
pub struct ClipboardItem {
    pub id: i64,
    pub value_text: String,
    pub is_pinned: bool,
    pub pin_order: i64,
    pub last_used_at: i64,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create database directory");
        }

        let conn = Connection::open(&db_path)?;
        Self::setup_schema(&conn)?;

        Ok(Database { conn: Mutex::new(conn) })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::setup_schema(&conn)?;
        Ok(Database { conn: Mutex::new(conn) })
    }

    fn setup_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value_text TEXT NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                pin_order INTEGER DEFAULT 0,
                last_used_at INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    fn get_db_path() -> PathBuf {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        #[cfg(target_os = "linux")]
        let path = format!("{}/.local/share/maccy-kde/history.db", home);
        #[cfg(target_os = "macos")]
        let path = format!("{}/Library/Application Support/maccy-kde/history.db", home);
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let path = format!("{}/.maccy-kde/history.db", home);

        PathBuf::from(path)
    }

    pub fn add_item(&self, text: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT id FROM clipboard_items WHERE value_text = ?1")?;
        let existing_id: Option<i64> = stmt.query_row(params![text], |row| row.get(0)).optional()?;
        drop(stmt);

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE clipboard_items SET last_used_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
        } else {
            conn.execute(
                "INSERT INTO clipboard_items (value_text, last_used_at) VALUES (?1, ?2)",
                params![text, now],
            )?;
        }

        self.rotate_history_locked(&conn)?;
        Ok(())
    }

    fn rotate_history_locked(&self, conn: &Connection) -> Result<()> {
        let max_items = 200;
        conn.execute(
            "DELETE FROM clipboard_items 
             WHERE is_pinned = 0 
               AND id NOT IN (
                   SELECT id FROM clipboard_items 
                   WHERE is_pinned = 0
                   ORDER BY last_used_at DESC 
                   LIMIT ?1
               )",
            params![max_items],
        )?;
        Ok(())
    }

    pub fn get_history(&self) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, value_text, is_pinned, pin_order, last_used_at 
             FROM clipboard_items 
             ORDER BY is_pinned DESC, pin_order ASC, last_used_at DESC 
             LIMIT 200"
        )?;

        let item_iter = stmt.query_map([], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                value_text: row.get(1)?,
                is_pinned: row.get::<_, i32>(2)? != 0,
                pin_order: row.get(3)?,
                last_used_at: row.get(4)?,
            })
        })?;

        let mut items = Vec::new();
        for item in item_iter {
            items.push(item?);
        }
        Ok(items)
    }

    pub fn toggle_pin(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT is_pinned FROM clipboard_items WHERE id = ?1")?;
        let is_pinned: i32 = stmt.query_row(params![id], |row| row.get(0))?;
        drop(stmt);
        
        let new_status = if is_pinned == 0 { 1 } else { 0 };
        conn.execute(
            "UPDATE clipboard_items SET is_pinned = ?1 WHERE id = ?2",
            params![new_status, id],
        )?;
        Ok(())
    }

    pub fn delete_item(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_history() {
        let db = Database::in_memory().unwrap();
        db.add_item("Hello").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.add_item("World").unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].value_text, "World"); // World is newer
        assert_eq!(history[1].value_text, "Hello");
    }

    #[test]
    fn test_toggle_pin() {
        let db = Database::in_memory().unwrap();
        db.add_item("To be pinned").unwrap();
        db.add_item("Normal").unwrap();

        let history = db.get_history().unwrap();
        let pin_id = history.iter().find(|i| i.value_text == "To be pinned").unwrap().id;

        db.toggle_pin(pin_id).unwrap();

        let new_history = db.get_history().unwrap();
        assert_eq!(new_history[0].value_text, "To be pinned");
        assert!(new_history[0].is_pinned);
    }

    #[test]
    fn test_rotation() {
        let db = Database::in_memory().unwrap();
        for i in 0..205 {
            db.add_item(&format!("Item {}", i)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 200);
        // "Item 204" was added last, so it should be the newest
        assert_eq!(history[0].value_text, "Item 204");
    }
}
