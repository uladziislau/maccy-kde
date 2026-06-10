use log::{info, error};

/// Set the system clipboard to `text` and simulate Ctrl+V (Linux) or Cmd+V (macOS)
use std::path::Path;

/// Set the system clipboard to `text` and simulate Ctrl+V (Linux) or Cmd+V (macOS)
pub fn paste_text(text: &str) {
    // Step 1: Set clipboard content
    #[cfg(target_os = "macos")]
    {
        use arboard::Clipboard;
        match Clipboard::new() {
            Ok(mut ctx) => {
                if let Err(e) = ctx.set_text(text.to_string()) {
                    error!("Failed to set clipboard: {}", e);
                    return;
                }
            }
            Err(e) => {
                error!("Failed to init clipboard: {}", e);
                return;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux Wayland, we use wl-copy via subprocess for reliability
        use std::process::Command;
        let result = Command::new("wl-copy")
            .arg(text)
            .status();
        match result {
            Ok(status) if status.success() => {}
            Ok(status) => {
                error!("wl-copy exited with: {}", status);
                return;
            }
            Err(e) => {
                error!("Failed to run wl-copy: {}", e);
                return;
            }
        }
    }

    // Step 2: Small delay for clipboard to propagate
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Step 3: Simulate Ctrl+V / Cmd+V
    info!("Simulating paste keystroke...");
    simulate_paste();
}

pub fn paste_image(path: &Path) {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let result = Command::new("wl-copy")
            .arg("--type")
            .arg("image/png")
            .stdin(std::fs::File::open(path).expect("Failed to open image file"))
            .status();
        match result {
            Ok(status) if status.success() => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                simulate_paste();
            }
            Ok(status) => error!("wl-copy image exited with: {}", status),
            Err(e) => error!("Failed to run wl-copy image: {}", e),
        }
    }
}

fn simulate_paste() {
    use enigo::{Enigo, Keyboard, Settings, Key, Direction};

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to init enigo: {:?}", e);
            return;
        }
    };

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta; // Cmd

    #[cfg(target_os = "linux")]
    let modifier = Key::Control; // Ctrl

    let _ = enigo.key(modifier, Direction::Press);
    let _ = enigo.key(Key::Unicode('v'), Direction::Click);
    let _ = enigo.key(modifier, Direction::Release);
}
