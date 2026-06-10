use rusqlite::{params, Connection, OptionalExtension, Result};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use image::RgbaImage;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum DataType {
    Text,
    Image,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub value_text: Option<String>,
    pub image_path: Option<PathBuf>,
    pub data_type: DataType,
    pub raw_mime_type: String,
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
        #[cfg(target_os = "linux")]
        {
            let cache_home = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| std::env::temp_dir().display().to_string());
                    PathBuf::from(home).join(".cache")
                });
            cache_home.join("maccy-kde")
        }

        #[cfg(target_os = "macos")]
        {
            std::env::temp_dir().join("maccy-kde-cache")
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            std::env::temp_dir().join("maccy-kde-cache")
        }
    }

    fn setup_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value_text TEXT,
                image_path TEXT,
                data_type TEXT NOT NULL,
                raw_mime_type TEXT NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                pin_order INTEGER DEFAULT 0,
                last_used_at INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    fn get_db_path() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            // Пытаемся получить XDG_DATA_HOME, или используем стандартное значение
            let data_home = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| std::env::temp_dir().display().to_string());
                    PathBuf::from(home).join(".local").join("share")
                });
            let maccy_dir = data_home.join("maccy-kde");
            let _ = std::fs::create_dir_all(&maccy_dir);
            return maccy_dir.join("history.db");
        }

        #[cfg(target_os = "macos")]
        {
            // На macOS для разработки
            let temp_dir = std::env::temp_dir();
            temp_dir.join("maccy-kde-history.db")
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let temp_dir = std::env::temp_dir();
            temp_dir.join("maccy-kde-history.db")
        }
    }

    /// Добавляет текстовый элемент в историю
    pub fn add_text_item(&self, text: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let conn = self.conn.lock().unwrap();

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
                "INSERT INTO clipboard_items (value_text, data_type, raw_mime_type, last_used_at) VALUES (?1, 'Text', 'text/plain', ?2)",
                params![text, now],
            )?;
        }

        self.rotate_history_locked(&conn)?;
        Ok(())
    }

    /// Добавляет изображение в историю
    pub fn add_image_item(&self, image: &RgbaImage, mime_type: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();

        // Вычисляем хэш изображения для дедупликации
        let hash = self.compute_image_hash(image);
        let filename = format!("{}.png", hash);
        let cache_path = Self::get_cache_path();
        let image_path = cache_path.join(&filename);

        let conn = self.conn.lock().unwrap();

        // Проверяем, есть ли уже такое изображение в БД
        let mut stmt = conn.prepare("SELECT id FROM clipboard_items WHERE image_path LIKE ?1 AND data_type = 'Image'")?;
        let existing_id: Option<i64> = stmt.query_row(params![format!("%{}", filename)], |row| row.get(0)).optional()?;
        drop(stmt);

        if let Some(id) = existing_id {
            // Обновляем время использования
            conn.execute(
                "UPDATE clipboard_items SET last_used_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
        } else {
            // Сохраняем изображение в кэш, если файла еще нет
            if !image_path.exists() {
                fs::create_dir_all(&cache_path).map_err(|e| rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("Failed to create cache directory: {}", e)),
                ))?;
                image.save(&image_path).map_err(|e| rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("Failed to save image: {}", e)),
                ))?;
            }

            // Добавляем запись в БД
            conn.execute(
                "INSERT INTO clipboard_items (image_path, data_type, raw_mime_type, last_used_at) VALUES (?1, 'Image', ?2, ?3)",
                params![image_path.to_str(), mime_type, now],
            )?;
        }

        self.rotate_history_locked(&conn)?;
        Ok(())
    }

    fn compute_image_hash(&self, image: &RgbaImage) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        image.as_raw().hash(&mut hasher);
        format!("{:x}", hasher.finish())
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, value_text, image_path, data_type, raw_mime_type, is_pinned, pin_order, last_used_at 
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
                is_pinned: row.get::<_, i32>(5)? != 0,
                pin_order: row.get(6)?,
                last_used_at: row.get(7)?,
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

    #[test]
    fn test_image_deduplication() {
        let db = Database::in_memory().unwrap();
        let img = RgbaImage::from_fn(10, 10, |_, _| Rgba([255, 255, 255, 255]));

        db.add_image_item(&img, "image/png").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.add_image_item(&img, "image/png").unwrap();

        let history = db.get_history().unwrap();
        // Должна быть только одна запись с обновленным временем (хотя в текущей реализации add_image_item
        // если запись найдена, она просто обновляет last_used_at, не добавляя новую)
        assert_eq!(history.len(), 1);
    }
}
