mod database;
mod clipboard;
mod paster;
mod ipc;
mod autostart;

use clap::Parser;
use database::Database;
use log::{info, error};
use std::sync::Arc;
use slint::{ModelRc, VecModel, SharedString};
use std::rc::Rc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

slint::include_modules!();

/// Легковесный менеджер буфера обмена для KDE Plasma 6 (Wayland)
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Запустить фоновый демон
    #[arg(long)]
    daemon: bool,

    /// Запустить графическое окно
    #[arg(long)]
    popup: bool,

    /// Установить автостарт для KDE Plasma
    #[arg(long)]
    install_autostart: bool,

    /// Удалить автостарт для KDE Plasma
    #[arg(long)]
    remove_autostart: bool,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    if args.install_autostart {
        match autostart::install_autostart() {
            Ok(_) => println!("Автостарт успешно установлен!"),
            Err(e) => eprintln!("Ошибка установки автостарта: {}", e),
        }
        return;
    }

    if args.remove_autostart {
        match autostart::remove_autostart() {
            Ok(_) => println!("Автостарт успешно удален!"),
            Err(e) => eprintln!("Ошибка удаления автостарта: {}", e),
        }
        return;
    }

    // Если ни один флаг не указан, запускаем всё (для разработки)
    if !args.daemon && !args.popup {
        info!("Starting maccy-kde in dev mode (everything)...");
        run_all_in_one();
        return;
    }

    if args.daemon {
        info!("Starting maccy-kde daemon...");
        run_daemon();
    }

    if args.popup {
        info!("Starting maccy-kde popup...");
        run_popup();
    }
}

/// Для разработки: запускаем всё в одном процессе
fn run_all_in_one() {
    // Initialize the database
    let db = match Database::new() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    // Start background clipboard monitor on a separate thread
    let db_monitor = db.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        rt.block_on(async {
            clipboard::start_clipboard_monitor(db_monitor).await;
            // Keep the runtime alive
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });

    // Create the Slint UI
    let ui = MaccyMenu::new().expect("Failed to create Slint UI");

    // Load initial history
    let db_ui = db.clone();
    refresh_ui_items(&ui, &db_ui, "");

    // --- Callback: search changed ---
    let ui_weak = ui.as_weak();
    let db_search = db.clone();
    ui.on_search_changed(move |text| {
        let ui = ui_weak.upgrade().expect("UI was dropped");
        let query = text.to_string();
        refresh_ui_items(&ui, &db_search, &query);
    });

    // --- Callback: paste item ---
    let ui_weak = ui.as_weak();
    let db_paste = db.clone();
    ui.on_paste_item(move |id| {
        info!("Paste item id={}", id);
        // Find the item, touch it to update last_used_at, then paste
        if let Ok(history) = db_paste.get_history() {
            if let Some(item) = history.iter().find(|i| i.id == id as i64) {
                // Update last_used_at
                match item.data_type {
                    crate::database::DataType::Text => {
                        if let Some(text) = &item.value_text {
                            let _ = db_paste.add_text_item(text);
                            // Close window first, then paste into the focused app
                            let ui = ui_weak.upgrade().expect("UI was dropped");
                            if let Err(e) = ui.hide() {
                                error!("Failed to hide UI: {:?}", e);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            paster::paste_text(text);
                        }
                    },
                    crate::database::DataType::Image => {
                        // TODO: Обработка вставки изображений
                        if let Some(path) = &item.image_path {
                            let ui = ui_weak.upgrade().expect("UI was dropped");
                            if let Err(e) = ui.hide() {
                                error!("Failed to hide UI: {:?}", e);
                            }
                            info!("Pasting image from: {:?}", path);
                        }
                    }
                }
                return;
            }
        }
        let ui = ui_weak.upgrade().expect("UI was dropped");
        if let Err(e) = ui.hide() {
            error!("Failed to hide UI: {:?}", e);
        }
    });

    // --- Callback: delete item ---
    let ui_weak = ui.as_weak();
    let db_del = db.clone();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let _ = db_del.delete_item(id as i64);
        let ui = ui_weak.upgrade().expect("UI was dropped");
        refresh_ui_items(&ui, &db_del, &ui.get_search_text().to_string());
    });

    // --- Callback: toggle pin ---
    let ui_weak = ui.as_weak();
    let db_pin = db.clone();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let _ = db_pin.toggle_pin(id as i64);
        let ui = ui_weak.upgrade().expect("UI was dropped");
        refresh_ui_items(&ui, &db_pin, &ui.get_search_text().to_string());
    });

    // --- Callback: close ---
    let ui_weak = ui.as_weak();
    ui.on_request_close(move || {
        let ui = ui_weak.upgrade().expect("UI was dropped");
        if let Err(e) = ui.hide() {
            error!("Failed to hide UI: {:?}", e);
        }
    });

    // Run the Slint event loop
    ui.run().expect("Failed to run Slint event loop");
}

/// Запустить демон
fn run_daemon() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime for daemon");
    rt.block_on(async {
        let db = match Database::new() {
            Ok(db) => Arc::new(db),
            Err(e) => {
                error!("Failed to initialize database: {}", e);
                return;
            }
        };

        // Start background clipboard monitor
        let db_monitor = db.clone();
        tokio::spawn(async {
            clipboard::start_clipboard_monitor(db_monitor).await;
        });

        // Start IPC server
        if let Err(e) = ipc::start_ipc_server(db).await {
            error!("Failed to start IPC server: {}", e);
        }
    });
}

/// Запустить popup
fn run_popup() {
    info!("Popup started, connecting to daemon...");
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime for popup");

    // Сначала попробуем подключиться к демону
    match rt.block_on(ipc::send_command(ipc::IpcCommand::GetHistory)) {
        Ok(_) => {
            // Если демон запущен, работаем через IPC
            run_popup_with_ipc(rt);
        },
        Err(_) => {
            // Если демон не запущен, запускаем всё в одном (для разработки)
            info!("Daemon not running, starting in single-process mode");
            run_all_in_one();
        }
    }
}

/// Запустить popup с подключением к демону через IPC
fn run_popup_with_ipc(rt: tokio::runtime::Runtime) {
    let ui = MaccyMenu::new().expect("Failed to create Slint UI for popup");

    // Загрузить начальный список
    let initial_history = match rt.block_on(ipc::send_command(ipc::IpcCommand::GetHistory)) {
        Ok(ipc::IpcResponse::History(items)) => items,
        _ => vec![]
    };
    refresh_ui_items_from_history(&ui, &initial_history, "");

    // --- Callback: search changed ---
    let ui_weak = ui.as_weak();
    let rt_for_search = rt.handle().clone();
    ui.on_search_changed(move |text| {
        let ui = ui_weak.upgrade().expect("UI was dropped");
        let query = text.to_string();
        if let Ok(ipc::IpcResponse::History(items)) = rt_for_search.block_on(ipc::send_command(ipc::IpcCommand::GetHistory)) {
            refresh_ui_items_from_history(&ui, &items, &query);
        }
    });

    // --- Callback: paste item ---
    let ui_weak = ui.as_weak();
    let rt_for_paste = rt.handle().clone();
    ui.on_paste_item(move |id| {
        info!("Paste item id={}", id);
        let ui = ui_weak.upgrade().expect("UI was dropped");
        if let Err(e) = ui.hide() {
            error!("Failed to hide UI: {:?}", e);
        }
        let _ = rt_for_paste.block_on(ipc::send_command(ipc::IpcCommand::SelectItem { id: id as i64 }));
    });

    // --- Callback: delete item ---
    let ui_weak = ui.as_weak();
    let rt_for_delete = rt.handle().clone();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let ui = ui_weak.upgrade().expect("UI was dropped");
        if let Ok(ipc::IpcResponse::History(items)) = rt_for_delete.block_on(ipc::send_command(ipc::IpcCommand::DeleteItem { id: id as i64 })) {
            refresh_ui_items_from_history(&ui, &items, &ui.get_search_text().to_string());
        }
    });

    // --- Callback: toggle pin ---
    let ui_weak = ui.as_weak();
    let rt_for_pin = rt.handle().clone();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let ui = ui_weak.upgrade().expect("UI was dropped");
        if let Ok(ipc::IpcResponse::History(items)) = rt_for_pin.block_on(ipc::send_command(ipc::IpcCommand::TogglePin { id: id as i64 })) {
            refresh_ui_items_from_history(&ui, &items, &ui.get_search_text().to_string());
        }
    });

    // --- Callback: close ---
    let ui_weak = ui.as_weak();
    ui.on_request_close(move || {
        let ui = ui_weak.upgrade().expect("UI was dropped");
        if let Err(e) = ui.hide() {
            error!("Failed to hide UI: {:?}", e);
        }
    });

    ui.run().expect("Failed to run Slint event loop");
}

/// Refresh UI из списка ClipboardItem
fn refresh_ui_items_from_history(ui: &MaccyMenu, items: &[database::ClipboardItem], query: &str) {
    let filtered = filter_items_fuzzy(items, query);

    let entries: Vec<ClipboardEntry> = filtered
        .iter()
        .map(|item| {
            let display_text = match &item.value_text {
                Some(text) if text.chars().count() > 100 => {
                    let truncated: String = text.chars().take(100).collect();
                    format!("{}…", truncated)
                },
                Some(text) => text.clone(),
                None => "📷 Изображение".to_string(),
            };
            ClipboardEntry {
                id: item.id as i32,
                text: SharedString::from(display_text),
                is_pinned: item.is_pinned,
                shortcut_index: 0, // assigned by Slint via index
            }
        })
        .collect();

    let model = Rc::new(VecModel::from(entries));
    ui.set_items(ModelRc::from(model));
    ui.set_current_index(0);
}

#[cfg(test)]
mod fuzzy_search_tests {
    use super::*;
    use crate::database::{ClipboardItem, DataType};

    fn create_test_item(id: i64, text: &str) -> ClipboardItem {
        ClipboardItem {
            id,
            value_text: Some(text.to_string()),
            image_path: None,
            data_type: DataType::Text,
            raw_mime_type: "text/plain".to_string(),
            is_pinned: false,
            pin_order: 0,
            last_used_at: chrono::Utc::now().timestamp(),
        }
    }

    #[test]
    fn test_filter_items_fuzzy_empty_query() {
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Test Item"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_items_fuzzy_exact_match() {
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Test Item"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "Hello");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].value_text, Some("Hello World".to_string()));
    }

    #[test]
    fn test_filter_items_fuzzy_partial_match() {
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Hello There"),
            create_test_item(3, "Test Item"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "He");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_items_fuzzy_no_match() {
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Test Item"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "xyz");
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_items_fuzzy_cyrillic() {
        let items = vec![
            create_test_item(1, "Привет мир"),
            create_test_item(2, "Тестовый элемент"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "Прив");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].value_text, Some("Привет мир".to_string()));
    }

    #[test]
    fn test_filter_items_fuzzy_emoji() {
        let items = vec![
            create_test_item(1, "Hello 🌍"),
            create_test_item(2, "Test 🚀"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "🌍");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].value_text, Some("Hello 🌍".to_string()));
    }

    #[test]
    fn test_filter_items_fuzzy_image_item() {
        let items = vec![
            ClipboardItem {
                id: 1,
                value_text: None,
                image_path: Some(std::path::PathBuf::from("/path/to/image.png")),
                data_type: DataType::Image,
                raw_mime_type: "image/png".to_string(),
                is_pinned: false,
                pin_order: 0,
                last_used_at: chrono::Utc::now().timestamp(),
            },
            create_test_item(2, "Test Item"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "Изображение");
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].value_text.is_none());
    }

    #[test]
    fn test_filter_items_fuzzy_case_sensitivity() {
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "HELLO THERE"),
        ];
        
        let filtered = filter_items_fuzzy(&items, "hello");
        // fuzzy-matcher is case-insensitive by default
        assert!(filtered.len() >= 1);
    }
}

/// Filter clipboard items using fuzzy search
pub fn filter_items_fuzzy<'a>(items: &'a [database::ClipboardItem], query: &str) -> Vec<&'a database::ClipboardItem> {
    if query.is_empty() {
        return items.iter().collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<_> = items
        .iter()
        .filter_map(|item| {
            let search_text = match &item.value_text {
                Some(text) => text,
                None => "Изображение",
            };
            matcher.fuzzy_match(search_text, query).map(|score| (item, score))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(item, _)| item).collect()
}

/// Refresh the item list in the UI, applying optional fuzzy search filter
fn refresh_ui_items(ui: &MaccyMenu, db: &Arc<Database>, query: &str) {
    let items = match db.get_history() {
        Ok(items) => items,
        Err(e) => {
            error!("Failed to get history: {}", e);
            return;
        }
    };

    let filtered = filter_items_fuzzy(&items, query);

    let entries: Vec<ClipboardEntry> = filtered
        .iter()
        .map(|item| {
            let display_text = match &item.value_text {
                Some(text) if text.chars().count() > 100 => {
                    let truncated: String = text.chars().take(100).collect();
                    format!("{}…", truncated)
                },
                Some(text) => text.clone(),
                None => "📷 Изображение".to_string(),
            };
            ClipboardEntry {
                id: item.id as i32,
                text: SharedString::from(display_text),
                is_pinned: item.is_pinned,
                shortcut_index: 0, // assigned by Slint via index
            }
        })
        .collect();

    let model = Rc::new(VecModel::from(entries));
    ui.set_items(ModelRc::from(model));
    ui.set_current_index(0);
}
