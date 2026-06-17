/// Popup module - popup UI management
/// Uses new Clean Architecture with PresentationService and RepositoryBridge

use log::{info, error};
use std::sync::{Arc, Mutex};
use crate::presentation::PresentationService;
use crate::integration::RepositoryBridge;
use crate::database::Database;
use slint::{ModelRc, VecModel, SharedString};
use std::rc::Rc;

slint::include_modules!();

pub fn run() {
    info!("Starting popup with new Clean Architecture...");
    
    // Initialize the database
    let db = match Database::new() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };
    
    // Create repository bridge (new architecture)
    let repository = Arc::new(RepositoryBridge::new(db.clone()));
    
    // Create presentation service wrapped in Arc
    let presentation = Arc::new(Mutex::new(PresentationService::new(repository.clone(), 100)));
    
    // Refresh initial state
    if let Ok(mut pres) = presentation.lock() {
        if let Err(e) = pres.refresh() {
            error!("Failed to refresh presentation state: {}", e);
            return;
        }
    }
    
    // Create the Slint UI
    let ui = MaccyMenu::new().expect("Failed to create Slint UI");
    
    // Load initial items from presentation service
    let entries: Vec<ClipboardEntry> = {
        let pres = presentation.lock().unwrap();
        let state = pres.get_state();
        state.items
            .iter()
            .map(|item| {
                ClipboardEntry {
                    id: item.id.parse::<i32>().unwrap_or(0),
                    text: SharedString::from(&item.display_text),
                    data_type: SharedString::from(&item.mime_type),
                    category: SharedString::from(item.category_badge.as_deref().unwrap_or("")),
                    image_path: SharedString::from(""),
                    is_pinned: item.is_pinned,
                    shortcut_index: 0,
                }
            })
            .collect()
    };
    
    ui.set_items(ModelRc::new(Rc::new(VecModel::from(entries))));
    
    // --- Callback: search changed ---
    let ui_weak = ui.as_weak();
    let presentation_search = presentation.clone();
    ui.on_search_changed(move |text| {
        let _ui = ui_weak.upgrade().expect("UI was dropped");
        // Update search query in presentation service
        if let Ok(mut pres) = presentation_search.lock() {
            let _ = pres.set_search_query(text.to_string());
        }
    });
    
    // --- Callback: paste item ---
    let ui_weak = ui.as_weak();
    ui.on_paste_item(move |id| {
        info!("Paste item id={}", id);
        let _ui = ui_weak.upgrade().expect("UI was dropped");
        // TODO: Implement paste through presentation service
    });
    
    // --- Callback: delete item ---
    let ui_weak = ui.as_weak();
    ui.on_delete_item(move |id| {
        info!("Delete item id={}", id);
        let _ui = ui_weak.upgrade().expect("UI was dropped");
        // TODO: Implement delete through presentation service
    });
    
    // --- Callback: toggle pin ---
    let ui_weak = ui.as_weak();
    ui.on_toggle_pin(move |id| {
        info!("Toggle pin id={}", id);
        let _ui = ui_weak.upgrade().expect("UI was dropped");
        // TODO: Implement toggle pin through presentation service
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