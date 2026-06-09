use std::sync::Arc;
use crate::database::Database;
use log::{info, error};

#[cfg(target_os = "macos")]
pub async fn start_clipboard_monitor(db: Arc<Database>) {
    use arboard::Clipboard;
    use std::time::Duration;

    info!("Starting macOS clipboard monitor (polling mode)");
    
    tokio::task::spawn_blocking(move || {
        let mut ctx = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to init arboard: {}", e);
                return;
            }
        };
        
        let mut last_clipboard = String::new();
        
        loop {
            if let Ok(text) = ctx.get_text() {
                if text != last_clipboard && !text.trim().is_empty() {
                    last_clipboard = text.clone();
                    let trimmed = text.trim();
                    if let Err(e) = db.add_item(trimmed) {
                        error!("Failed to add item to DB: {}", e);
                    } else {
                        info!("Copied new item to DB: {:.20}...", trimmed);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(1000));
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
                error!("Failed to initialize Wayland clipboard listener: {:?}", e);
                return;
            }
        };

        for context in stream.paste_stream().flatten().flatten() {
            let bytes = context.to_vec(); // Ensure we have bytes
            if let Ok(text) = String::from_utf8(bytes) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if let Err(e) = db.add_item(trimmed) {
                        error!("Failed to add item to DB: {}", e);
                    } else {
                        info!("Copied new item to DB: {:.20}...", trimmed);
                    }
                }
            }
        }
    });
}
