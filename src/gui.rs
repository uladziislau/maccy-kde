use std::sync::Arc;
use std::rc::Rc;
use slint::{ModelRc, VecModel, SharedString, Weak, ComponentHandle};
use fuzzy_matcher::FuzzyMatcher;
use log::info;
use crate::database::{Database, ClipboardItem, DataType};
use crate::paster;

slint::include_modules!();

pub struct GuiManager {
    ui: Weak<MaccyMenu>,
    db: Arc<Database>,
}

impl GuiManager {
    pub fn new(ui: MaccyMenu, db: Arc<Database>) -> Self {
        Self { ui: ui.as_weak(), db }
    }

    pub fn setup_callbacks(&self) {
        let ui_weak = self.ui.clone();
        let db_clone = self.db.clone();

        let ui = ui_weak.upgrade().unwrap();
        // Initial refresh
        self.refresh("");

        // Search
        let ui_search = ui_weak.clone();
        let db_search = db_clone.clone();
        ui.on_search_changed(move |text| {
            if let Some(ui) = ui_search.upgrade() {
                Self::refresh_ui(&ui, &db_search, &text);
            }
        });

        // Paste
        let ui_paste = ui_weak.clone();
        let db_paste = db_clone.clone();
        ui.on_paste_item(move |id| {
            info!("Pasting item id={}", id);
            if let Ok(history) = db_paste.get_history() {
                if let Some(item) = history.iter().find(|i| i.id == id as i64) {
                    if let Some(ui) = ui_paste.upgrade() {
                        let _ = ui.hide();
                    }
                    match item.data_type {
                        DataType::Text => {
                            if let Some(text) = &item.value_text {
                                let _ = db_paste.add_text_item(text);
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                paster::paste_text(text);
                            }
                        }
                        DataType::Image => {
                            if let Some(path) = &item.image_path {
                                paster::paste_image(path);
                            }
                        }
                    }
                }
            }
        });

        // Delete
        let ui_del = ui_weak.clone();
        let db_del = db_clone.clone();
        ui.on_delete_item(move |id| {
            let _ = db_del.delete_item(id as i64);
            if let Some(ui) = ui_del.upgrade() {
                Self::refresh_ui(&ui, &db_del, &ui.get_search_text());
            }
        });

        // Pin
        let ui_pin = ui_weak.clone();
        let db_pin = db_clone.clone();
        ui.on_toggle_pin(move |id| {
            let _ = db_pin.toggle_pin(id as i64);
            if let Some(ui) = ui_pin.upgrade() {
                Self::refresh_ui(&ui, &db_pin, &ui.get_search_text());
            }
        });

        // Clear Unpinned
        let ui_clear = ui_weak.clone();
        let db_clear = db_clone.clone();
        ui.on_clear_unpinned(move || {
            let _ = db_clear.clear_unpinned();
            if let Some(ui) = ui_clear.upgrade() {
                Self::refresh_ui(&ui, &db_clear, &ui.get_search_text());
            }
        });

        // Close
        let ui_close = ui_weak.clone();
        ui.on_request_close(move || {
            if let Some(ui) = ui_close.upgrade() {
                let _ = ui.hide();
            }
        });

        let ui_win_close = ui_weak.clone();
        ui.window().on_close_requested(move || {
            if let Some(ui) = ui_win_close.upgrade() {
                let _ = ui.hide();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }

    pub fn show(&self) {
        if let Some(ui) = self.ui.upgrade() {
            self.refresh("");
            ui.show().expect("Failed to show UI");
        }
    }

    pub fn hide(&self) {
        if let Some(ui) = self.ui.upgrade() {
            let _ = ui.hide();
        }
    }

    pub fn run(&self) {
        // Run is called on the strong handle usually
    }

    pub fn get_weak(&self) -> Weak<MaccyMenu> {
        self.ui.clone()
    }

    fn refresh(&self, query: &str) {
        if let Some(ui) = self.ui.upgrade() {
            Self::refresh_ui(&ui, &self.db, query);
        }
    }

    pub fn refresh_data(&self) {
        self.refresh("");
    }

    pub fn refresh_ui(ui: &MaccyMenu, db: &Arc<Database>, query: &str) {
        let items = db.get_history().unwrap_or_default();
        let entries = Self::filter_and_map_items(items, query);
        let model = Rc::new(VecModel::from(entries));
        ui.set_items(ModelRc::from(model));
        ui.set_current_index(0);
    }

    pub fn refresh_ui_from_history(ui: &MaccyMenu, items: Vec<ClipboardItem>, query: &str) {
        let entries = Self::filter_and_map_items(items, query);
        let model = Rc::new(VecModel::from(entries));
        ui.set_items(ModelRc::from(model));
        ui.set_current_index(0);
    }

    fn format_relative_time(millis: i64) -> String {
        let now = chrono::Utc::now().timestamp_millis();
        let diff = (now - millis).abs() / 1000; // diff in seconds

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

    fn filter_and_map_items(items: Vec<ClipboardItem>, query: &str) -> Vec<ClipboardEntry> {
        let filtered: Vec<ClipboardItem> = if query.is_empty() {
            items
        } else {
            let matcher = crate::get_matcher();
            let mut scored: Vec<_> = items
                .into_iter()
                .filter_map(|item| {
                    let search_text = item.value_text.as_deref().unwrap_or("📷 Изображение");
                    matcher.fuzzy_match(search_text, query).map(|score| (item, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            scored.into_iter().map(|(item, _)| item).collect()
        };

        filtered.into_iter().enumerate().map(|(i, item)| {
            let display_text = match &item.value_text {
                Some(text) if text.len() > 100 => format!("{}…", &text[..100]),
                Some(text) => text.clone(),
                None => "📷 Изображение".to_string(),
            };
            let is_code = item.value_text.as_ref().map(|t| Self::is_likely_code(t)).unwrap_or(false);
            ClipboardEntry {
                id: item.id as i32,
                text: SharedString::from(display_text),
                timestamp: SharedString::from(Self::format_relative_time(item.last_used_at)),
                is_pinned: item.is_pinned,
                is_image: item.data_type == DataType::Image,
                is_code,
                shortcut_index: if (i as i32) < 9 { (i as i32) + 1 } else { 0 },
            }
        }).collect()
    }
}
