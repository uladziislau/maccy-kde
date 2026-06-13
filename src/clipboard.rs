use std::sync::Arc;
use crate::database::Database;
use log::{info, error};

#[cfg(target_os = "macos")]
pub async fn start_clipboard_monitor(db: Arc<Database>) {
    use arboard::Clipboard;
    use std::time::Duration;
    use image::RgbaImage;

    info!("Starting macOS clipboard monitor (polling mode)");
    
    tokio::task::spawn_blocking(move || {
        let mut ctx = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to init arboard: {}", e);
                return;
            }
        };
        
        let mut last_text = String::new();
        let mut last_image_hash: Option<u64> = None;

        loop {
            // Проверяем текст
            if let Ok(text) = ctx.get_text() {
                if text != last_text && !text.trim().is_empty() {
                    last_text = text.clone();
                    let trimmed = text.trim();
                    if let Err(e) = db.add_text_item(trimmed) {
                        error!("Failed to add text item to DB: {}", e);
                    } else {
                        info!("Copied new text item to DB: {:.20}...", trimmed);
                    }
                }
            }

            // Проверяем изображение
            if let Ok(image) = ctx.get_image() {
                // Вычисляем простой хэш для сравнения
                let hash = compute_image_hash(&image.bytes);
                if last_image_hash != Some(hash) {
                    last_image_hash = Some(hash);
                    // Конвертируем в RgbaImage
                    if let Some(rgba_image) = RgbaImage::from_raw(
                        image.width as u32,
                        image.height as u32,
                        image.bytes.to_vec()
                    ) {
                        if let Err(e) = db.add_image_item(&rgba_image, "image/png") {
                            error!("Failed to add image item to DB: {}", e);
                        } else {
                            info!("Copied new image item to DB");
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

#[cfg(target_os = "linux")]
pub async fn start_clipboard_monitor(db: Arc<Database>) {
    use wayland_clipboard_listener::{WlClipboardPasteStream, WlListenType};
    use image::RgbaImage;

    info!("Starting Wayland clipboard monitor (event-driven)");

    tokio::task::spawn_blocking(move || {
        let mut stream = match WlClipboardPasteStream::init(WlListenType::ListenOnCopy) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to init Wayland clipboard listener: {:?}", e);
                return;
            }
        };

        for context in stream.paste_stream().flatten().flatten() {
            // Проверяем, текст ли это
            if let Ok(text) = String::from_utf8(context.to_vec()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if let Err(e) = db.add_text_item(trimmed) {
                        error!("Failed to add text item to DB: {}", e);
                    } else {
                        info!("Copied new text item to DB: {:.20}...", trimmed);
                    }
                    continue;
                }
            }

            // Попытка обработки изображений
            // Wayland clipboard использует MIME types для определения типа данных
            // Изображения обычно передаются как image/png, image/jpeg, и т.д.
            // Для полной поддержки требуется:
            // 1. Проверка MIME типов в контексте
            // 2. Запрос данных в нужном формате через Wayland протоколы
            // 3. Декодирование изображения из байтов
            // 4. Конвертация в RgbaImage для хранения
            // Текущая библиотека wayland-clipboard-listener предоставляет только сырые байты
            // без информации о MIME типах, что делает надежную обработку изображений сложной
            let bytes = context.to_vec();
            // Проверяем, могут ли байты быть изображением (простая эвристика)
            if let Some(image_type) = detect_image_type(&bytes) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    if let Some(rgba_image) = img.to_rgba8().as_raw().as_rgba() {
                        if let Err(e) = db.add_image_item(rgba_image, image_type) {
                            error!("Failed to add {} image item to DB: {}", image_type, e);
                        } else {
                            info!("Copied new {} image item to DB", image_type);
                        }
                    }
                }
            }
        }
    });
}

/// Простой хэш для сравнения изображений (чтобы не дублировать)
pub fn compute_image_hash(bytes: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Определяет тип изображения по magic bytes
pub fn detect_image_type(bytes: &[u8]) -> Option<&'static str> {
    // JPEG magic bytes: FF D8 FF (3 bytes)
    if bytes.len() >= 3 && bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A (8 bytes)
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    None
}

#[cfg(test)]
mod clipboard_utils_tests {
    use super::*;

    #[test]
    fn test_compute_image_hash() {
        let bytes1 = b"test data";
        let bytes2 = b"test data";
        let bytes3 = b"different data";

        assert_eq!(compute_image_hash(bytes1), compute_image_hash(bytes2));
        assert_ne!(compute_image_hash(bytes1), compute_image_hash(bytes3));
    }

    #[test]
    fn test_detect_image_type_png() {
        // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(detect_image_type(&png_bytes), Some("image/png"));
    }

    #[test]
    fn test_detect_image_type_jpeg() {
        // JPEG magic bytes: FF D8 FF
        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0x00, 0x00, 0x00];
        assert_eq!(detect_image_type(&jpeg_bytes), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_image_type_unknown() {
        let unknown_bytes = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_image_type(&unknown_bytes), None);
    }

    #[test]
    fn test_detect_image_type_too_short() {
        let short_bytes = [0x89, 0x50];
        assert_eq!(detect_image_type(&short_bytes), None);
    }

    #[test]
    fn test_detect_image_type_empty() {
        let empty_bytes: [u8; 0] = [];
        assert_eq!(detect_image_type(&empty_bytes), None);
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod macos_clipboard_tests {
    use super::*;
    use arboard::Clipboard;
    use std::time::Duration;
    use tokio::time::{sleep, timeout};

    // These tests are marked as ignored because the clipboard monitor runs in an infinite
    // loop in a blocking task that cannot be easily stopped from within a test.
    // The monitor is designed to run as a long-lived daemon process.
    // Manual testing is required for clipboard functionality:
    // 1. Run: cargo run -- --daemon
    // 2. Copy some text to clipboard
    // 3. Check the database at ~/Library/Application Support/maccy-kde/history.db
    // 4. Or run: cargo run -- --popup to see the UI

    #[tokio::test]
    #[ignore]
    async fn test_macos_clipboard_text_monitor() {
        // Создаем временную базу данных
        let db = Arc::new(Database::in_memory().unwrap());

        // Запускаем монитор буфера обмена с таймаутом
        let db_clone = db.clone();
        let monitor_handle = tokio::spawn(async move {
            start_clipboard_monitor(db_clone).await;
            // Keep the runtime alive
            loop {
                sleep(Duration::from_secs(3600)).await;
            }
        });

        // Даем время монитору на инициализацию
        sleep(Duration::from_millis(100)).await;

        // Копируем текст в буфер обмена
        let test_text = "Test clipboard content for macOS";
        let mut ctx = Clipboard::new().unwrap();
        ctx.set_text(test_text).unwrap();

        // Ждем polling (500ms + небольшой запас) с таймаутом
        let result = timeout(Duration::from_secs(5), async {
            sleep(Duration::from_millis(700)).await;
            
            // Проверяем, что текст появился в базе
            let history = db.get_history().unwrap();
            assert!(!history.is_empty(), "History should not be empty after clipboard copy");
            
            // Ищем наш текст в истории
            let found = history.iter().any(|item| item.value_text.as_deref() == Some(test_text));
            assert!(found, "Test text should be found in clipboard history");
        }).await;

        // Останавливаем монитор
        monitor_handle.abort();
        
        // Проверяем результат
        result.expect("Test timed out");
    }

    #[tokio::test]
    #[ignore]
    async fn test_macos_clipboard_text_deduplication() {
        // Создаем временную базу данных
        let db = Arc::new(Database::in_memory().unwrap());

        // Запускаем монитор буфера обмена
        let db_clone = db.clone();
        let monitor_handle = tokio::spawn(async move {
            start_clipboard_monitor(db_clone).await;
            loop {
                sleep(Duration::from_secs(3600)).await;
            }
        });

        // Даем время монитору на инициализацию
        sleep(Duration::from_millis(100)).await;

        // Копируем текст в буфер обмена
        let test_text = "Deduplication test text";
        let mut ctx = Clipboard::new().unwrap();
        ctx.set_text(test_text).unwrap();

        // Ждем polling
        sleep(Duration::from_millis(700)).await;

        // Проверяем, что текст появился один раз
        let history = db.get_history().unwrap();
        let count = history.iter().filter(|item| item.value_text.as_deref() == Some(test_text)).count();
        assert_eq!(count, 1, "Text should appear only once (deduplication)");

        // Копируем тот же текст снова
        ctx.set_text(test_text).unwrap();

        // Ждем polling
        sleep(Duration::from_millis(700)).await;

        // Проверяем, что текст все еще один раз (не дублируется)
        let history = db.get_history().unwrap();
        let count = history.iter().filter(|item| item.value_text.as_deref() == Some(test_text)).count();
        assert_eq!(count, 1, "Text should still appear only once after second copy");

        // Останавливаем монитор
        monitor_handle.abort();
    }

    #[tokio::test]
    #[ignore]
    async fn test_macos_clipboard_empty_text_ignored() {
        // Создаем временную базу данных
        let db = Arc::new(Database::in_memory().unwrap());

        // Запускаем монитор буфера обмена
        let db_clone = db.clone();
        let monitor_handle = tokio::spawn(async move {
            start_clipboard_monitor(db_clone).await;
            loop {
                sleep(Duration::from_secs(3600)).await;
            }
        });

        // Даем время монитору на инициализацию
        sleep(Duration::from_millis(100)).await;

        // Копируем пустой текст
        let empty_text = "   ";
        let mut ctx = Clipboard::new().unwrap();
        ctx.set_text(empty_text).unwrap();

        // Ждем polling
        sleep(Duration::from_millis(700)).await;

        // Проверяем, что пустой текст не добавился
        let history = db.get_history().unwrap();
        assert!(history.is_empty(), "Empty/whitespace-only text should not be added to history");

        // Останавливаем монитор
        monitor_handle.abort();
    }
}
