mod database;
mod clipboard;

use database::Database;
use log::{info, error};
use std::sync::Arc;
use slint::{ModelRc, VecModel, SharedString};
use std::rc::Rc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

slint::include_modules!();

fn main() {
    env_logger::init();
    info!("Starting maccy-kde...");

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
        // Touch the item to update last_used_at
        if let Ok(history) = db_paste.get_history() {
            if let Some(item) = history.iter().find(|i| i.id == id as i64) {
                let _ = db_paste.add_item(&item.value_text);
            }
        }
        // Close window after paste
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
        ui.hide().unwrap();
    });

    // Run the Slint event loop
    ui.run().unwrap();
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
                matcher.fuzzy_match(&item.value_text, query).map(|score| (item, score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(item, _)| item).collect()
    };

    let entries: Vec<ClipboardEntry> = filtered
        .iter()
        .map(|item| ClipboardEntry {
            id: item.id as i32,
            text: SharedString::from(
                if item.value_text.len() > 100 {
                    format!("{}…", &item.value_text[..100])
                } else {
                    item.value_text.clone()
                }
            ),
            is_pinned: item.is_pinned,
            shortcut_index: 0, // assigned by Slint via index
        })
        .collect();

    let model = Rc::new(VecModel::from(entries));
    ui.set_items(ModelRc::from(model));
    ui.set_current_index(0);
}
