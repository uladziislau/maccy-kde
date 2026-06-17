/// Database module - legacy SQLite database implementation
/// This will be gradually replaced by the new infrastructure layer

use rusqlite::{params, Connection, OptionalExtension, Result};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use image::RgbaImage;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum DataType {
    Text,
    Image,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Category {
    Url,
    Email,
    Account,
    Picture,
    Other,
}

impl ToString for Category {
    fn to_string(&self) -> String {
        match self {
            Category::Url => "Url".to_string(),
            Category::Email => "Email".to_string(),
            Category::Account => "Account".to_string(),
            Category::Picture => "Picture".to_string(),
            Category::Other => "Other".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub value_text: Option<String>,
    pub image_path: Option<PathBuf>,
    pub data_type: DataType,
    pub raw_mime_type: String,
    pub category: Option<Category>,
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
            fs::create_dir_all(parent).map_err(|e| rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("Failed to create directory: {}", e)),
            ))?;
        }
        // Создаем директорию для кэша изображений
        let _ = fs::create_dir_all(Self::get_cache_path());

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

    fn get_cache_path() -> PathBuf {
        // Use the new AppPaths infrastructure
        crate::infrastructure::system::paths::AppPaths::images_cache_path()
    }

    fn setup_schema(conn: &Connection) -> Result<()> {
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
        )?;
        
        // Add category column if it doesn't exist (for existing databases)
        conn.execute(
            "ALTER TABLE clipboard_items ADD COLUMN category TEXT DEFAULT NULL",
            [],
        ).ok(); // Ignore error if column already exists
        
        Ok(())
    }

    fn get_db_path() -> PathBuf {
        // Use the new AppPaths infrastructure
        crate::infrastructure::system::paths::AppPaths::database_path()
    }

    /// Детектирует категорию текстового контента
    fn detect_category(text: &str) -> Option<Category> {
        // URL detection
        if text.starts_with("http://") || text.starts_with("https://") || 
           text.starts_with("www.") || (text.contains(".") && text.contains("/")) {
            return Some(Category::Url);
        }
        
        // Email detection
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        if email_regex.is_match(text) {
            return Some(Category::Email);
        }
        
        // Account/Username detection (two patterns: @username or plain username)
        let account_regex_with_at = regex::Regex::new(r"^@[a-zA-Z0-9_]+$").unwrap();
        let account_regex_plain = regex::Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap();
        
        if account_regex_with_at.is_match(text) || (account_regex_plain.is_match(text) && !text.contains(" ") && !text.contains("@")) {
            return Some(Category::Account);
        }
        
        Some(Category::Other)
    }

    /// Добавляет текстовый элемент в историю
    pub fn add_text_item(&self, text: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let category = Self::detect_category(text).map(|c| c.to_string());
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(format!("Mutex poisoned: {}", e)),
        ))?;

        // Проверяем, есть ли уже такой текст
        let mut stmt = conn.prepare("SELECT id FROM clipboard_items WHERE value_text = ?1 AND data_type = 'Text'")?;
        let existing_id: Option<i64> = stmt.query_row(params![text], |row| row.get(0)).optional()?;
        drop(stmt);

        if let Some(id) = existing_id {
            // Обновляем время использования
            conn.execute(
                "UPDATE clipboard_items SET last_used_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
        } else {
            // Добавляем новый элемент
            conn.execute(
                "INSERT INTO clipboard_items (value_text, data_type, raw_mime_type, category, last_used_at) VALUES (?1, 'Text', 'text/plain', ?2, ?3)",
                params![text, category, now],
            )?;
        }

        self.rotate_history_locked(&conn)?;
        Ok(())
    }

    /// Добавляет изображение в историю
    pub fn add_image_item(&self, image: &RgbaImage, mime_type: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(format!("Mutex poisoned: {}", e)),
        ))?;

        // Сохраняем изображение в кэш
        let cache_path = Self::get_cache_path();
        fs::create_dir_all(&cache_path).map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
            Some(format!("Failed to create cache directory: {}", e)),
        ))?;
        let filename = format!("{}.png", Uuid::new_v4());
        let image_path = cache_path.join(&filename);
        image.save(&image_path).map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
            Some(format!("Failed to save image: {}", e)),
        ))?;

        // Добавляем запись в БД
        conn.execute(
            "INSERT INTO clipboard_items (image_path, data_type, raw_mime_type, category, last_used_at) VALUES (?1, 'Image', ?2, 'Picture', ?3)",
            params![image_path.to_str(), mime_type, now],
        )?;

        self.rotate_history_locked(&conn)?;
        Ok(())
    }

    fn rotate_history_locked(&self, conn: &Connection) -> Result<()> {
        let max_items = 200;

        // Сначала получаем id элементов, которые будем удалять
        let mut stmt = conn.prepare(
            "SELECT id, image_path FROM clipboard_items 
             WHERE is_pinned = 0 
               AND id NOT IN (
                   SELECT id FROM clipboard_items 
                   WHERE is_pinned = 0
                   ORDER BY last_used_at DESC 
                   LIMIT ?1
               )"
        )?;
        let items_to_delete = stmt.query_map(params![max_items], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?
            ))
        })?;

        // Удаляем файлы изображений и записи из БД
        for item in items_to_delete {
            let (id, image_path_str) = item?;
            if let Some(path_str) = image_path_str {
                let _ = fs::remove_file(PathBuf::from(path_str));
            }
            conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        }

        Ok(())
    }

    pub fn get_history(&self) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(format!("Mutex poisoned: {}", e)),
        ))?;
        let mut stmt = conn.prepare(
            "SELECT id, value_text, image_path, data_type, raw_mime_type, category, is_pinned, pin_order, last_used_at 
             FROM clipboard_items 
             ORDER BY is_pinned DESC, pin_order ASC, last_used_at DESC 
             LIMIT 200",
        )?;

        let item_iter = stmt.query_map([], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                value_text: row.get(1)?,
                image_path: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                data_type: match row.get::<_, String>(3)?.as_str() {
                    "Image" => DataType::Image,
                    _ => DataType::Text,
                },
                raw_mime_type: row.get(4)?,
                category: row.get::<_, Option<String>>(5)?.and_then(|s| match s.as_str() {
                    "Url" => Some(Category::Url),
                    "Email" => Some(Category::Email),
                    "Account" => Some(Category::Account),
                    "Picture" => Some(Category::Picture),
                    "Other" => Some(Category::Other),
                    _ => None,
                }),
                is_pinned: row.get::<_, i32>(6)? != 0,
                pin_order: row.get(7)?,
                last_used_at: row.get(8)?,
            })
        })?;

        let mut items = Vec::new();
        for item in item_iter {
            items.push(item?);
        }
        Ok(items)
    }

    pub fn toggle_pin(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(format!("Mutex poisoned: {}", e)),
        ))?;
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
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(format!("Mutex poisoned: {}", e)),
        ))?;
        // Сначала получаем путь к изображению
        let mut stmt = conn.prepare("SELECT image_path FROM clipboard_items WHERE id = ?1")?;
        let image_path_str: Option<String> = stmt.query_row(params![id], |row| row.get(0)).optional()?;
        drop(stmt);

        // Удаляем файл изображения, если есть
        if let Some(path_str) = image_path_str {
            let _ = fs::remove_file(PathBuf::from(path_str));
        }

        // Удаляем запись из БД
        conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn test_add_and_get_history() {
        let db = Database::in_memory().unwrap();
        db.add_text_item("Hello").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.add_text_item("World").unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].value_text, Some("World".to_string()));
        assert_eq!(history[1].value_text, Some("Hello".to_string()));
    }

    #[test]
    fn test_toggle_pin() {
        let db = Database::in_memory().unwrap();
        db.add_text_item("To be pinned").unwrap();
        db.add_text_item("Normal").unwrap();

        let history = db.get_history().unwrap();
        let pin_id = history.iter().find(|i| i.value_text == Some("To be pinned".to_string())).unwrap().id;

        db.toggle_pin(pin_id).unwrap();

        let new_history = db.get_history().unwrap();
        assert_eq!(new_history[0].value_text, Some("To be pinned".to_string()));
        assert!(new_history[0].is_pinned);
    }

    #[test]
    fn test_empty_string() {
        let db = Database::in_memory().unwrap();
        db.add_text_item("").unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value_text, Some("".to_string()));
    }

    #[test]
    fn test_category_detection_url() {
        assert_eq!(Database::detect_category("https://example.com"), Some(Category::Url));
        assert_eq!(Database::detect_category("http://test.org"), Some(Category::Url));
        assert_eq!(Database::detect_category("www.example.com"), Some(Category::Url));
        assert_eq!(Database::detect_category("example.com/path"), Some(Category::Url));
    }

    #[test]
    fn test_category_detection_email() {
        assert_eq!(Database::detect_category("test@example.com"), Some(Category::Email));
        assert_eq!(Database::detect_category("user.name@domain.org"), Some(Category::Email));
        assert_eq!(Database::detect_category("not-an-email"), Some(Category::Other));
    }

    #[test]
    fn test_category_detection_account() {
        assert_eq!(Database::detect_category("@username"), Some(Category::Account));
        assert_eq!(Database::detect_category("john_doe"), Some(Category::Account));
        assert_eq!(Database::detect_category("user123"), Some(Category::Account));
        assert_eq!(Database::detect_category("not a username"), Some(Category::Other)); // contains space
        assert_eq!(Database::detect_category("user@example.com"), Some(Category::Email)); // email takes precedence
    }

    #[test]
    fn test_category_classification_in_db() {
        let db = Database::in_memory().unwrap();
        
        db.add_text_item("https://github.com").unwrap();
        db.add_text_item("user@example.com").unwrap();
        db.add_text_item("@myusername").unwrap();
        db.add_text_item("regular text").unwrap();
        
        let history = db.get_history().unwrap();
        
        // Find and check URL
        let url_item = history.iter().find(|i| i.value_text == Some("https://github.com".to_string())).unwrap();
        assert_eq!(url_item.category, Some(Category::Url));
        
        // Find and check Email
        let email_item = history.iter().find(|i| i.value_text == Some("user@example.com".to_string())).unwrap();
        assert_eq!(email_item.category, Some(Category::Email));
        
        // Find and check Account
        let account_item = history.iter().find(|i| i.value_text == Some("@myusername".to_string())).unwrap();
        assert_eq!(account_item.category, Some(Category::Account));
        
        // Find and check Other
        let other_item = history.iter().find(|i| i.value_text == Some("regular text".to_string())).unwrap();
        assert_eq!(other_item.category, Some(Category::Other));
    }

    #[test]
    fn test_whitespace_only_string() {
        let db = Database::in_memory().unwrap();
        db.add_text_item("   ").unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value_text, Some("   ".to_string()));
    }

    #[test]
    fn test_very_long_string() {
        let db = Database::in_memory().unwrap();
        let long_text = "A".repeat(100000);
        db.add_text_item(&long_text).unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value_text, Some(long_text));
    }

    #[test]
    fn test_special_characters() {
        let db = Database::in_memory().unwrap();
        let special_text = "Test with émojis 🎉 and spëcial çhars\n\tand \"quotes\"";
        db.add_text_item(special_text).unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value_text, Some(special_text.to_string()));
    }

    #[test]
    fn test_cyrillic_text() {
        let db = Database::in_memory().unwrap();
        let cyrillic_text = "Привет мир! Тест кириллицы";
        db.add_text_item(cyrillic_text).unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value_text, Some(cyrillic_text.to_string()));
    }

    #[test]
    fn test_duplicate_items() {
        let db = Database::in_memory().unwrap();
        db.add_text_item("Duplicate").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.add_text_item("Duplicate").unwrap();

        let history = db.get_history().unwrap();
        // Database deduplicates items with the same text
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value_text, Some("Duplicate".to_string()));
    }

    #[test]
    fn test_delete_nonexistent_item() {
        let db = Database::in_memory().unwrap();
        let result = db.delete_item(999999);
        // Deleting a nonexistent item returns Ok (no-op)
        assert!(result.is_ok());
    }

    #[test]
    fn test_toggle_pin_nonexistent_item() {
        let db = Database::in_memory().unwrap();
        let result = db.toggle_pin(999999);
        // Toggling pin on nonexistent item returns error
        assert!(result.is_err());
    }

    #[test]
    fn test_rotation() {
        let db = Database::in_memory().unwrap();
        for i in 0..205 {
            db.add_text_item(&format!("Item {}", i)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 200);
        assert_eq!(history[0].value_text, Some("Item 204".to_string()));
    }

    #[test]
    fn test_add_image() {
        let db = Database::in_memory().unwrap();
        // Создаем простое изображение 10x10
        let img = RgbaImage::from_fn(10, 10, |x, y| {
            if (x + y) % 2 == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });

        db.add_image_item(&img, "image/png").unwrap();
        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert!(matches!(history[0].data_type, DataType::Image));
    }
}
