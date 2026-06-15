use std::sync::Arc;
use slint::ComponentHandle;
use log::{info, error};
use crate::database::Database;
use crate::gui::{GuiManager, MaccyMenu, Keeper};
use crate::{clipboard, ipc, GlobalState};

/// Для разработки: запускаем всё в одном процессе
pub fn run_all_in_one() {
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
    let _keeper = Keeper::new().unwrap(); // Keep event loop alive
    let gui = GuiManager::new(ui.clone_strong(), db.clone());
    gui.setup_callbacks();
    ui.show().unwrap();

    // Run the Slint event loop
    ui.run().unwrap();
}

/// Запустить демон
pub fn run_daemon() {
    let db = match Database::new() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    let ui = MaccyMenu::new().unwrap();
    let gui = GuiManager::new(ui.clone_strong(), db.clone());
    gui.setup_callbacks();

    {
        let mut state_lock = crate::get_state().lock().unwrap();
        *state_lock = Some(GlobalState {
            gui: Some(gui),
            db: db.clone(),
        });
    }

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

            tokio::spawn(async move {
                if let Err(e) = start_dbus_server().await {
                    error!("Failed to start DBus server: {}", e);
                }
            });

            if let Err(e) = ipc::start_ipc_server(db).await {
                error!("Failed to start IPC server: {}", e);
            }
        });
    });

    info!("Daemon running with UI prepared.");
    // Don't show window by default when running daemon mode
    let _keeper = Keeper::new().unwrap(); // Keep event loop alive
    ui.run().unwrap();
}

/// Запустить popup
pub fn run_popup() {
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
    let _keeper = Keeper::new().unwrap(); // Keep event loop alive

    // Загрузить начальный список
    let initial_history = match rt.block_on(ipc::send_command(ipc::IpcCommand::GetHistory { query: None })) {
        Ok(ipc::IpcResponse::History(items)) => items,
        _ => vec![]
    };
    GuiManager::refresh_ui_from_history(&ui, initial_history, "");

    // --- Callback: search changed ---
    let ui_weak = ui.as_weak();
    ui.on_search_changed(move |text: slint::SharedString| {
        let ui_for_task = ui_weak.clone();
        let query = text.to_string();

        tokio::spawn(async move {
            if let Ok(ipc::IpcResponse::History(items)) = ipc::send_command(ipc::IpcCommand::GetHistory { query: Some(query) }).await {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_for_task.upgrade() {
                        GuiManager::refresh_ui_from_history(&ui, items, "");
                    }
                });
            }
        });
    });

    // --- Callback: paste item ---
    let ui_weak = ui.as_weak();
    ui.on_paste_item(move |id| {
        info!("Paste item id={}", id);
        let ui = ui_weak.unwrap();
        let _ = ui.hide();
        tokio::spawn(async move {
            let _ = ipc::send_command(ipc::IpcCommand::SelectItem { id: id as i64 }).await;
        });
    });

    // --- Callback: delete item ---
    let ui_weak = ui.as_weak();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let ui_for_task = ui_weak.clone();
        tokio::spawn(async move {
            if let Ok(ipc::IpcResponse::History(items)) = ipc::send_command(ipc::IpcCommand::DeleteItem { id: id as i64 }).await {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_for_task.upgrade() {
                        GuiManager::refresh_ui_from_history(&ui, items, &ui.get_search_text());
                    }
                });
            }
        });
    });

    // --- Callback: toggle pin ---
    let ui_weak = ui.as_weak();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let ui_for_task = ui_weak.clone();
        tokio::spawn(async move {
            if let Ok(ipc::IpcResponse::History(items)) = ipc::send_command(ipc::IpcCommand::TogglePin { id: id as i64 }).await {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_for_task.upgrade() {
                        GuiManager::refresh_ui_from_history(&ui, items, &ui.get_search_text());
                    }
                });
            }
        });
    });

    // --- Callback: clear unpinned ---
    let ui_weak = ui.as_weak();
    ui.on_clear_unpinned(move || {
        info!("Clear unpinned");
        let ui_for_task = ui_weak.clone();
        tokio::spawn(async move {
            if let Ok(ipc::IpcResponse::History(items)) = ipc::send_command(ipc::IpcCommand::ClearUnpinned).await {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_for_task.upgrade() {
                        GuiManager::refresh_ui_from_history(&ui, items, &ui.get_search_text());
                    }
                });
            }
        });
    });

    // --- Callback: close ---
    let ui_weak = ui.as_weak();
    ui.on_request_close(move || {
        let ui = ui_weak.unwrap();
        let _ = ui.hide();
    });

    ui.show().unwrap();
    ui.run().unwrap();
}

struct DaemonIpc;

#[zbus::dbus_interface(name = "org.maccy_kde.Daemon")]
impl DaemonIpc {
    async fn show(&self) {
        slint::invoke_from_event_loop(|| {
            crate::show_ui();
        }).unwrap();
    }
}

async fn start_dbus_server() -> Result<(), Box<dyn std::error::Error>> {
    use zbus::ConnectionBuilder;
    let _conn = ConnectionBuilder::session()?
        .name("org.maccy_kde.Daemon")?
        .serve_at("/org/maccy_kde/Daemon", DaemonIpc)?
        .build()
        .await?;

    info!("DBus server started at org.maccy_kde.Daemon");
    // Keep connection alive
    std::future::pending::<()>().await;
    Ok(())
}
