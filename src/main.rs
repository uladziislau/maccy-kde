mod database;
mod clipboard;
mod paster;
mod ipc;
mod autostart;

mod bootstrap;
mod hotkey;
mod infrastructure;

use log::{info, error};
use database::{Database, ClipboardItem, DataType};
use slint::{ModelRc, VecModel, SharedString};
use std::sync::Arc;
use std::rc::Rc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

slint::include_modules!();

fn main() {
    env_logger::init();
    bootstrap::Bootstrap::from_args().run();
}

/// Запуск всего в одном процессе
pub fn run_all_in_one() {
    let db = match Database::new() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    let db_monitor = db.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        rt.block_on(async {
            clipboard::start_clipboard_monitor(db_monitor).await;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });

    let ui = MaccyMenu::new().expect("Failed to create Slint UI");

    // Start invisible. UI is shown/hidden only via hotkey callback.
    ui.hide().ok();

    refresh_ui(&ui, &db, "");

    // --- search ---
    let ui_weak = ui.as_weak();
    let db_search = db.clone();
    ui.on_search_changed(move |text| {
        let ui = ui_weak.upgrade().expect("UI was dropped");
        refresh_ui(&ui, &db_search, &text.to_string());
    });

    // --- paste ---
    let ui_weak = ui.as_weak();
    let db_paste = db.clone();
    ui.on_paste_item(move |id| {
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => return,
        };
        // Only handle paste when window is visible — ignore spurious calls
        if !ui.window().is_visible() {
            info!("on_paste_item id={} ignored (window not visible)", id);
            return;
        }
        info!("Paste item id={} visible={}", id, ui.window().is_visible());
        let text = if let Ok(items) = db_paste.get_history() {
            items.iter().find(|i| i.id == id as i64)
                .and_then(|item| item.value_text.clone())
        } else {
            None
        };
        if let Some(text) = text {
            let _ = db_paste.add_text_item(&text);
            let _ = ui.hide();
            // Paste action must not run inside the Slint event loop —
            // it uses subprocesses/osascript that can conflict with Cocoa.
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(150));
                paster::paste_text(&text);
            });
        } else {
            let _ = ui.hide();
        }
    });

    // --- delete ---
    let ui_weak = ui.as_weak();
    let db_del = db.clone();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let _ = db_del.delete_item(id as i64);
        let ui = ui_weak.upgrade().expect("UI was dropped");
        refresh_ui(&ui, &db_del, &ui.get_search_text().to_string());
    });

    // --- pin ---
    let ui_weak = ui.as_weak();
    let db_pin = db.clone();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let _ = db_pin.toggle_pin(id as i64);
        let ui = ui_weak.upgrade().expect("UI was dropped");
        refresh_ui(&ui, &db_pin, &ui.get_search_text().to_string());
    });

    // --- close ---
    let ui_weak = ui.as_weak();
    ui.on_request_close(move || {
        let ui = ui_weak.upgrade().expect("UI was dropped");
        if let Err(e) = ui.hide() {
            error!("Failed to hide UI: {:?}", e);
        }
    });

    // --- global hotkey ---
    let ui_weak = ui.as_weak();
    let db_hotkey = db.clone();
    if let Err(e) = hotkey::register(move || {
        info!("Hotkey callback invoked on AppKit main thread");
        let weak = ui_weak.clone();
        let db = db_hotkey.clone();
        slint::invoke_from_event_loop(move || {
            info!("Hotkey dispatched to Slint event loop");
            if let Some(ui) = weak.upgrade() {
                if ui.window().is_visible() {
                    info!("Hotkey: hiding window");
                    let _ = ui.hide();
                } else {
                    info!("Hotkey: showing window");
                    refresh_ui(&ui, &db, "");
                    if let Some((x, y)) = cursor_position() {
                        ui.window().set_position(slint::LogicalPosition { x: x as f32, y: y as f32 });
                    }
                    if let Err(e) = ui.show() {
                        error!("Failed to show UI from hotkey: {:?}", e);
                    }
                }
            }
        })
        .ok();
    }) {
        error!("Failed to register global hotkey: {}", e);
    }

    // Use run_event_loop_until_quit so that hide() doesn't exit the event loop.
    // The window is shown/hidden by the NSMenuItem hotkey.
    slint::run_event_loop_until_quit().expect("Failed to run Slint event loop");
}

fn format_relative_time(millis: i64) -> String {
    let now = chrono::Utc::now().timestamp_millis();
    let diff = (now - millis).abs() / 1000;

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

fn is_likely_code(text: &str) -> bool {
    let code_indicators = ["{", "}", ";", "fn ", "let ", "var ", "const ", "import ", "from ", "public ", "private ", "class "];
    if text.lines().count() > 1 {
        return true;
    }
    code_indicators.iter().any(|&ind| text.contains(ind))
}

fn filter_items<'a>(items: &'a [ClipboardItem], query: &str) -> Vec<&'a ClipboardItem> {
    if query.is_empty() {
        return items.iter().collect();
    }

    let (cat_filter, search_q) = parse_query(query);
    let matcher = SkimMatcherV2::default();

    let mut scored: Vec<_> = items
        .iter()
        .filter_map(|item| {
            if let Some(ref cat) = cat_filter {
                if item.category.as_ref() != Some(cat) {
                    return None;
                }
            }
            if !search_q.is_empty() {
                let text = item.value_text.as_deref().unwrap_or("Изображение");
                matcher.fuzzy_match(text, search_q).map(|s| (item, s))
            } else {
                Some((item, 100))
            }
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(item, _)| item).collect()
}

fn parse_query(query: &str) -> (Option<database::Category>, &str) {
    if query.starts_with('@') {
        let parts: Vec<&str> = query.splitn(2, ' ').collect();
        let cat = match parts[0].to_lowercase().as_str() {
            "@url" => Some(database::Category::Url),
            "@email" => Some(database::Category::Email),
            "@account" => Some(database::Category::Account),
            "@picture" => Some(database::Category::Picture),
            "@other" => Some(database::Category::Other),
            _ => None,
        };
        (cat, if parts.len() > 1 { parts[1].trim() } else { "" })
    } else {
        (None, query)
    }
}

fn refresh_ui(ui: &MaccyMenu, db: &Arc<Database>, query: &str) {
    let items = match db.get_history() {
        Ok(items) => items,
        Err(e) => {
            error!("Failed to get history: {}", e);
            return;
        }
    };

    let filtered = filter_items(&items, query);
    let entries: Vec<ClipboardEntry> = filtered.iter().enumerate().map(|(i, item)| item_to_entry(item, i)).collect();
    let model = Rc::new(VecModel::from(entries));
    ui.set_items(ModelRc::from(model));
    ui.set_current_index(0);
}

fn item_to_entry(item: &ClipboardItem, index: usize) -> ClipboardEntry {
    let display_text = match &item.value_text {
        Some(text) if text.chars().count() > 100 => {
            let truncated: String = text.chars().take(100).collect();
            format!("{}…", truncated)
        }
        Some(text) => text.clone(),
        None => "📷 Изображение".to_string(),
    };

    let is_code = item.value_text.as_ref().map(|t| is_likely_code(t)).unwrap_or(false);

    ClipboardEntry {
        id: item.id as i32,
        text: SharedString::from(display_text),
        timestamp: SharedString::from(format_relative_time(item.last_used_at)),
        is_pinned: item.is_pinned,
        is_image: item.data_type == DataType::Image,
        is_code,
        shortcut_index: if (index as i32) < 9 { (index as i32) + 1 } else { 0 },
    }
}

fn cursor_position() -> Option<(f64, f64)> {
    use enigo::{Enigo, Mouse, Settings};
    let enigo = Enigo::new(&Settings::default()).ok()?;
    let (x, y) = enigo.location().ok()?;
    Some((x as f64, y as f64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::{ClipboardItem, DataType, Category};

    fn make_item(id: i64, text: &str) -> ClipboardItem {
        ClipboardItem {
            id,
            value_text: Some(text.to_string()),
            image_path: None,
            data_type: DataType::Text,
            raw_mime_type: "text/plain".to_string(),
            category: None,
            is_pinned: false,
            pin_order: 0,
            last_used_at: chrono::Utc::now().timestamp(),
        }
    }

    #[test]
    fn test_filter_empty_query() {
        let items = vec![make_item(1, "Hello"), make_item(2, "World")];
        assert_eq!(filter_items(&items, "").len(), 2);
    }

    #[test]
    fn test_filter_exact_match() {
        let items = vec![make_item(1, "Hello World"), make_item(2, "Test")];
        assert_eq!(filter_items(&items, "Hello").len(), 1);
    }

    #[test]
    fn test_filter_no_match() {
        let items = vec![make_item(1, "Hello")];
        assert!(filter_items(&items, "xyz").is_empty());
    }

    #[test]
    fn test_filter_cyrillic() {
        let items = vec![make_item(1, "Привет мир")];
        assert_eq!(filter_items(&items, "Прив").len(), 1);
    }

    #[test]
    fn test_filter_emoji() {
        let items = vec![make_item(1, "Hello 🌍")];
        assert_eq!(filter_items(&items, "🌍").len(), 1);
    }

    #[test]
    fn test_filter_by_category() {
        let items = vec![
            ClipboardItem {
                id: 1, value_text: Some("https://example.com".to_string()),
                image_path: None, data_type: DataType::Text,
                raw_mime_type: "text/plain".to_string(),
                category: Some(Category::Url), is_pinned: false,
                pin_order: 0, last_used_at: 0,
            },
            ClipboardItem {
                id: 2, value_text: Some("user@example.com".to_string()),
                image_path: None, data_type: DataType::Text,
                raw_mime_type: "text/plain".to_string(),
                category: Some(Category::Email), is_pinned: false,
                pin_order: 0, last_used_at: 0,
            },
        ];
        assert_eq!(filter_items(&items, "@url").len(), 1);
        assert_eq!(filter_items(&items, "@email").len(), 1);
    }
}
