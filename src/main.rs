mod database;
mod clipboard;
mod paster;
mod ipc;
mod autostart;

use fs2::FileExt;
use std::fs::File;
use clap::Parser;
use database::Database;
use log::{info, error};
use std::sync::Arc;
use slint::{ModelRc, VecModel, SharedString};
use std::rc::Rc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

slint::include_modules!();

use std::sync::Mutex;

struct GlobalState {
    ui: Option<slint::Weak<MaccyMenu>>,
    db: Arc<Database>,
}

lazy_static::lazy_static! {
    static ref STATE: Mutex<Option<GlobalState>> = Mutex::new(None);
}

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

    let _lock_file = if args.daemon || (!args.daemon && !args.popup) {
        let lock_path = std::env::temp_dir().join("maccy-kde.lock");
        let file = File::create(&lock_path).expect("Failed to create lock file");
        if file.try_lock_exclusive().is_err() {
            if args.daemon {
                eprintln!("Daemon is already running.");
                std::process::exit(1);
            }
            None
        } else {
            Some(file)
        }
    } else {
        None
    };

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
        info!("Requesting popup from daemon...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(ipc::send_command(ipc::IpcCommand::ShowPopup)) {
            Ok(_) => {
                info!("Popup requested successfully");
                return;
            },
            Err(e) => {
                error!("Failed to request popup: {}", e);
                info!("Starting popup in standalone mode...");
                run_popup();
                return;
            }
        }
    }
}

async fn register_global_shortcut() -> Result<(), Box<dyn std::error::Error>> {
    info!("Global shortcut registration (placeholder) - currently rely on KDE shortcut to 'maccy-kde --popup'");
    Ok(())
}

pub fn show_ui() {
    let mut state_lock = STATE.lock().unwrap();
    if let Some(state) = state_lock.as_mut() {
        if let Some(ui_weak) = &state.ui {
            if let Some(ui) = ui_weak.upgrade() {
                // Apply blur effect (KDE/Wayland specific)
                #[cfg(target_os = "linux")]
                {
                    let _ = apply_blur_effect(&ui);
                }

                ui.show().unwrap();
                refresh_ui_items(&ui, &state.db, "");
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn apply_blur_effect(_ui: &MaccyMenu) -> Result<(), Box<dyn std::error::Error>> {
    // In Slint with Wayland/KDE, we usually set a property or use DBus
    // For now, this is a placeholder for the DBus call mentioned in doc 04
    Ok(())
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
            .unwrap();
        rt.block_on(async {
            clipboard::start_clipboard_monitor(db_monitor).await;
            // Keep the runtime alive
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });

    // Create the Slint UI
    let ui = MaccyMenu::new().unwrap();

    // Load initial history
    let db_ui = db.clone();
    refresh_ui_items(&ui, &db_ui, "");

    // --- Callback: search changed ---
    let ui_weak = ui.as_weak();
    let db_search = db.clone();
    ui.on_search_changed(move |text| {
        let ui = ui_weak.unwrap();
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
                            let ui = ui_weak.unwrap();
                            ui.hide().unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            paster::paste_text(text);
                        }
                    },
                    crate::database::DataType::Image => {
                        // TODO: Обработка вставки изображений
                        if let Some(path) = &item.image_path {
                            let ui = ui_weak.unwrap();
                            ui.hide().unwrap();
                            info!("Pasting image from: {:?}", path);
                        }
                    }
                }
                return;
            }
        }
        let ui = ui_weak.unwrap();
        ui.hide().unwrap();
    });

    // --- Callback: delete item ---
    let ui_weak = ui.as_weak();
    let db_del = db.clone();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let _ = db_del.delete_item(id as i64);
        let ui = ui_weak.unwrap();
        refresh_ui_items(&ui, &db_del, &ui.get_search_text().to_string());
    });

    // --- Callback: toggle pin ---
    let ui_weak = ui.as_weak();
    let db_pin = db.clone();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let _ = db_pin.toggle_pin(id as i64);
        let ui = ui_weak.unwrap();
        refresh_ui_items(&ui, &db_pin, &ui.get_search_text().to_string());
    });

    // --- Callback: close ---
    let ui_weak = ui.as_weak();
    ui.on_request_close(move || {
        let ui = ui_weak.unwrap();
        let _ = ui.hide();
    });

    let ui_weak = ui.as_weak();
    ui.window().on_close_requested(move || {
        let ui = ui_weak.unwrap();
        let _ = ui.hide();
        slint::CloseRequestResponse::KeepWindowShown
    });

    // Run the Slint event loop
    ui.run().unwrap();
}

/// Запустить демон
fn run_daemon() {
    let db = match Database::new() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    let ui = MaccyMenu::new().unwrap();
    setup_ui_callbacks(&ui, &db);

    {
        let mut state_lock = STATE.lock().unwrap();
        *state_lock = Some(GlobalState {
            ui: Some(ui.as_weak()),
            db: db.clone(),
        });
    }

    // Register global shortcut via DBus (KDE kglobalaccel)
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            if let Err(e) = register_global_shortcut().await {
                error!("Failed to register global shortcut: {}", e);
            }
        });
    });

    // Start background threads
    let db_monitor = db.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            tokio::spawn(async move {
                clipboard::start_clipboard_monitor(db_monitor).await;
            });

            if let Err(e) = ipc::start_ipc_server(db).await {
                error!("Failed to start IPC server: {}", e);
            }
        });
    });

    info!("Daemon running with UI prepared.");
    // slint::set_quit_on_last_window_closed(false); // Only in newer Slint or different API
    ui.run().unwrap();
}

fn setup_ui_callbacks(ui: &MaccyMenu, db: &Arc<Database>) {
    // Initial data
    refresh_ui_items(ui, db, "");

    // --- Callback: search changed ---
    let ui_weak = ui.as_weak();
    let db_search = db.clone();
    ui.on_search_changed(move |text| {
        let ui = ui_weak.unwrap();
        let query = text.to_string();
        refresh_ui_items(&ui, &db_search, &query);
    });

    // --- Callback: paste item ---
    let ui_weak = ui.as_weak();
    let db_paste = db.clone();
    ui.on_paste_item(move |id| {
        info!("Paste item id={}", id);
        if let Ok(history) = db_paste.get_history() {
            if let Some(item) = history.iter().find(|i| i.id == id as i64) {
                match item.data_type {
                    crate::database::DataType::Text => {
                        if let Some(text) = &item.value_text {
                            let _ = db_paste.add_text_item(text);
                            let ui = ui_weak.unwrap();
                            ui.hide().unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            paster::paste_text(text);
                        }
                    },
                    crate::database::DataType::Image => {
                        if let Some(path) = &item.image_path {
                            let ui = ui_weak.unwrap();
                            let _ = ui.hide();
                            info!("Pasting image from: {:?}", path);
                            paster::paste_image(path);
                        }
                    }
                }
                return;
            }
        }
        let ui = ui_weak.unwrap();
        ui.hide().unwrap();
    });

    // --- Callback: delete item ---
    let ui_weak = ui.as_weak();
    let db_del = db.clone();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let _ = db_del.delete_item(id as i64);
        let ui = ui_weak.unwrap();
        refresh_ui_items(&ui, &db_del, &ui.get_search_text().to_string());
    });

    // --- Callback: toggle pin ---
    let ui_weak = ui.as_weak();
    let db_pin = db.clone();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let _ = db_pin.toggle_pin(id as i64);
        let ui = ui_weak.unwrap();
        refresh_ui_items(&ui, &db_pin, &ui.get_search_text().to_string());
    });

    // --- Callback: close ---
    let ui_weak = ui.as_weak();
    ui.on_request_close(move || {
        let ui = ui_weak.unwrap();
        let _ = ui.hide();
    });

    let ui_weak = ui.as_weak();
    ui.window().on_close_requested(move || {
        let ui = ui_weak.unwrap();
        let _ = ui.hide();
        slint::CloseRequestResponse::KeepWindowShown
    });
}

/// Запустить popup
fn run_popup() {
    info!("Popup started, connecting to daemon...");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Сначала попробуем подключиться к демону
    match rt.block_on(ipc::send_command(ipc::IpcCommand::GetHistory { query: None })) {
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
    let ui = MaccyMenu::new().unwrap();

    // Загрузить начальный список
    let initial_history = match rt.block_on(ipc::send_command(ipc::IpcCommand::GetHistory { query: None })) {
        Ok(ipc::IpcResponse::History(items)) => items,
        _ => vec![]
    };
    refresh_ui_items_from_history(&ui, &initial_history, "");

    // --- Callback: search changed ---
    let ui_weak = ui.as_weak();
    let rt_for_search = rt.handle().clone();
    ui.on_search_changed(move |text| {
        let ui = ui_weak.unwrap();
        let query = text.to_string();
        if let Ok(ipc::IpcResponse::History(items)) = rt_for_search.block_on(ipc::send_command(ipc::IpcCommand::GetHistory { query: Some(query.clone()) })) {
            refresh_ui_items_from_history(&ui, &items, ""); // items are already filtered
        }
    });

    // --- Callback: paste item ---
    let ui_weak = ui.as_weak();
    let rt_for_paste = rt.handle().clone();
    ui.on_paste_item(move |id| {
        info!("Paste item id={}", id);
        let ui = ui_weak.unwrap();
        ui.hide().unwrap();
        let _ = rt_for_paste.block_on(ipc::send_command(ipc::IpcCommand::SelectItem { id: id as i64 }));
    });

    // --- Callback: delete item ---
    let ui_weak = ui.as_weak();
    let rt_for_delete = rt.handle().clone();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let ui = ui_weak.unwrap();
        if let Ok(ipc::IpcResponse::History(items)) = rt_for_delete.block_on(ipc::send_command(ipc::IpcCommand::DeleteItem { id: id as i64 })) {
            refresh_ui_items_from_history(&ui, &items, &ui.get_search_text().to_string());
        }
    });

    // --- Callback: toggle pin ---
    let ui_weak = ui.as_weak();
    let rt_for_pin = rt.handle().clone();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let ui = ui_weak.unwrap();
        if let Ok(ipc::IpcResponse::History(items)) = rt_for_pin.block_on(ipc::send_command(ipc::IpcCommand::TogglePin { id: id as i64 })) {
            refresh_ui_items_from_history(&ui, &items, &ui.get_search_text().to_string());
        }
    });

    // --- Callback: close ---
    let ui_weak = ui.as_weak();
    ui.on_request_close(move || {
        let ui = ui_weak.unwrap();
        ui.hide().unwrap();
    });

    ui.run().unwrap();
}

/// Refresh UI из списка ClipboardItem
fn refresh_ui_items_from_history(ui: &MaccyMenu, items: &[database::ClipboardItem], query: &str) {
    let filtered: Vec<&database::ClipboardItem> = if query.is_empty() {
        items.iter().collect()
    } else {
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
    };

    let entries: Vec<ClipboardEntry> = filtered
        .iter()
        .map(|item| {
            let display_text = match &item.value_text {
                Some(text) if text.len() > 100 => format!("{}…", &text[..100]),
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

/// Refresh the item list in the UI, applying optional fuzzy search filter
fn refresh_ui_items(ui: &MaccyMenu, db: &Arc<Database>, query: &str) {
    let items = match db.get_history() {
        Ok(items) => items,
        Err(e) => {
            error!("Failed to get history: {}", e);
            return;
        }
    };

    let filtered: Vec<&database::ClipboardItem> = if query.is_empty() {
        items.iter().collect()
    } else {
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
    };

    let entries: Vec<ClipboardEntry> = filtered
        .iter()
        .map(|item| {
            let display_text = match &item.value_text {
                Some(text) if text.len() > 100 => format!("{}…", &text[..100]),
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
