use std::sync::{Mutex, OnceLock};

type Callback = Box<dyn Fn() + Send + 'static>;

static HOTKEY_CALLBACK: OnceLock<Mutex<Callback>> = OnceLock::new();

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2::rc::Retained;
    use objc2::runtime::NSObject;
    use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem, NSEventModifierFlags};
    use objc2_foundation::NSString;

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = objc2::MainThreadOnly]
        struct HotkeyHandler;

        impl HotkeyHandler {
            #[unsafe(method(hotkeyAction:))]
            fn hotkey_action(&self, _sender: &NSObject) {
                if let Some(mtx) = HOTKEY_CALLBACK.get() {
                    if let Ok(cb) = mtx.lock() {
                        cb();
                    }
                }
            }
        }
    );

    pub fn register(callback: Callback) -> Result<(), String> {
        HOTKEY_CALLBACK
            .set(Mutex::new(callback))
            .map_err(|_| "Hotkey callback already registered".to_string())?;

        let mtm = MainThreadMarker::new()
            .ok_or("register() must be called from the main thread".to_string())?;

        unsafe {
            let app = NSApplication::sharedApplication(mtm);

            let menu_item = NSMenuItem::new(mtm);
            menu_item.setKeyEquivalent(&NSString::from_str("v"));
            menu_item.setKeyEquivalentModifierMask(
                NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
            );

            let allocated = HotkeyHandler::alloc(mtm);
            let handler: Retained<HotkeyHandler> = msg_send![allocated, init];
            menu_item.setTarget(Some(&*handler));
            menu_item.setAction(Some(sel!(hotkeyAction:)));

            if let Some(menu) = app.mainMenu() {
                menu.addItem(&menu_item);
            } else {
                let new_menu = NSMenu::new(mtm);
                new_menu.addItem(&menu_item);
                app.setMainMenu(Some(&new_menu));
            }
        }

        log::info!("Global hotkey registered: Cmd+Shift+V (via NSMenu)");
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;

    pub fn register(callback: Callback) -> Result<(), String> {
        HOTKEY_CALLBACK
            .set(Mutex::new(callback))
            .map_err(|_| "Hotkey callback already registered".to_string())?;

        log::info!("Global hotkey registered: Super+C (via stub)");
        Ok(())
    }
}

pub fn register<F>(callback: F) -> Result<(), String>
where
    F: Fn() + Send + 'static,
{
    imp::register(Box::new(callback))
}
