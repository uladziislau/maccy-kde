pub mod database;
pub mod clipboard;
pub mod paster;
pub mod ipc;
pub mod autostart;
pub mod gui;
pub mod app;

use std::sync::{Arc, Mutex, OnceLock};
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::database::Database;
use crate::gui::GuiManager;

pub struct GlobalState {
    pub gui: Option<GuiManager>,
    pub db: Arc<Database>,
}

pub static STATE: OnceLock<Mutex<Option<GlobalState>>> = OnceLock::new();
pub static MATCHER: OnceLock<SkimMatcherV2> = OnceLock::new();

pub fn get_matcher() -> &'static SkimMatcherV2 {
    MATCHER.get_or_init(SkimMatcherV2::default)
}

pub fn get_state() -> &'static Mutex<Option<GlobalState>> {
    STATE.get_or_init(|| Mutex::new(None))
}

pub fn show_ui() {
    let mut state_lock = get_state().lock().unwrap();
    if let Some(state) = state_lock.as_mut() {
        if let Some(gui) = &state.gui {
            gui.show();
        }
    }
}

pub fn refresh_ui() {
    let mut state_lock = get_state().lock().unwrap();
    if let Some(state) = state_lock.as_mut() {
        if let Some(gui) = &state.gui {
            gui.refresh_data();
        }
    }
}
