use log::{info, error};

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

    if let Err(e) = enigo.key(modifier, Direction::Press) {
        error!("Failed to press modifier key: {:?}", e);
        return;
    }
    if let Err(e) = enigo.key(Key::Unicode('v'), Direction::Click) {
        error!("Failed to press 'v' key: {:?}", e);
        // Try to release modifier even if 'v' failed
        let _ = enigo.key(modifier, Direction::Release);
        return;
    }
    if let Err(e) = enigo.key(modifier, Direction::Release) {
        error!("Failed to release modifier key: {:?}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests are marked as ignored because they perform actual system operations
    // (clipboard manipulation and keyboard simulation) which is not suitable for automated testing.
    // Manual testing is required for paste functionality.

    #[test]
    #[ignore]
    fn test_paste_text_empty_string() {
        // This test verifies that paste_text handles empty strings without panicking
        paste_text("");
    }

    #[test]
    #[ignore]
    fn test_paste_text_normal_string() {
        // This test verifies that paste_text handles normal strings without panicking
        paste_text("Test text");
    }

    #[test]
    #[ignore]
    fn test_paste_text_special_characters() {
        // This test verifies that paste_text handles special characters without panicking
        paste_text("Test with émojis 🎉 and spëcial çhars");
    }

    #[test]
    #[ignore]
    fn test_paste_text_long_string() {
        // This test verifies that paste_text handles long strings without panicking
        let long_text = "A".repeat(10000);
        paste_text(&long_text);
    }
}
