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

            // TODO: Обработка изображений на Wayland требует дополнительной работы
            // (Wayland clipboard protocols для изображений сложнее)
        }
    });
}

/// Простой хэш для сравнения изображений (чтобы не дублировать)
fn compute_image_hash(bytes: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
