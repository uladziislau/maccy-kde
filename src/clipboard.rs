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
                        slint::invoke_from_event_loop(|| {
                            crate::refresh_ui();
                        }).unwrap();
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

    info!("Starting Wayland clipboard monitor (event-driven)");

    tokio::task::spawn_blocking(move || {
        let mut stream = match WlClipboardPasteStream::init(WlListenType::ListenOnCopy) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to init Wayland clipboard listener: {:?}", e);
                return;
            }
        };

        for message in stream.paste_stream().flatten() {
            let content = message.context.context;
            let mime_type = message.context.mime_type;

            // Проверяем, текст ли это
            if mime_type.starts_with("text/") {
                if let Ok(text) = String::from_utf8(content.to_vec()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        if let Err(e) = db.add_text_item(trimmed) {
                            error!("Failed to add text item to DB: {}", e);
                        } else {
                            info!("Copied new text item to DB: {:.20}...", trimmed);
                        slint::invoke_from_event_loop(|| {
                            crate::refresh_ui();
                        }).unwrap();
                        }
                        continue;
                    }
                }
            }

            // Обработка изображений
            if mime_type.starts_with("image/") {
                if let Ok(img) = image::load_from_memory(&content) {
                    if let Err(e) = db.add_image_item(&img.to_rgba8(), &mime_type) {
                        error!("Failed to add image item to DB: {}", e);
                    } else {
                        info!("Copied new image item to DB (type: {})", mime_type);
                        slint::invoke_from_event_loop(|| {
                            crate::refresh_ui();
                        }).unwrap();
                    }
                }
            }
        }
    });
}

