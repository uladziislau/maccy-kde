use std::io::Write;
use std::path::Path;
use log::{info, error};

/// Set the system clipboard to `text` and simulate Ctrl+V (Linux) or Cmd+V (macOS)
pub fn paste_text(text: &str) {
    let clipboard_result = set_clipboard(text);
    if let Err(e) = clipboard_result {
        error!("Failed to set clipboard: {}", e);
        return;
    }

    std::thread::sleep(std::time::Duration::from_millis(80));

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

    #[cfg(target_os = "macos")]
    {
        error!("paste_image not implemented on macOS");
    }
}

#[cfg(target_os = "macos")]
fn set_clipboard(text: &str) -> Result<(), String> {
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("pbcopy spawn: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())
            .map_err(|e| format!("pbcopy write: {}", e))?;
        drop(stdin);
    }
    child.wait()
        .map_err(|e| format!("pbcopy wait: {}", e))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_clipboard(text: &str) -> Result<(), String> {
    let mut child = std::process::Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("wl-copy spawn: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())
            .map_err(|e| format!("wl-copy write: {}", e))?;
        drop(stdin);
    }
    child.wait()
        .map_err(|e| format!("wl-copy wait: {}", e))?;
    Ok(())
}

fn simulate_paste() {
    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \"v\" using command down")
            .status();
        match result {
            Ok(status) if status.success() => return,
            Ok(status) => error!("osascript exited with: {}", status),
            Err(e) => error!("Failed to run osascript: {}", e),
        }
        fallback_paste_enigo();
    }

    #[cfg(target_os = "linux")]
    {
        use enigo::{Enigo, Keyboard, Settings, Key, Direction};
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to init enigo: {:?}", e);
                return;
            }
        };
        let modifier = Key::Control;
        let _ = enigo.key(modifier, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(modifier, Direction::Release);
    }
}

#[cfg(target_os = "macos")]
fn fallback_paste_enigo() {
    use enigo::{Enigo, Keyboard, Settings, Key, Direction};
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            error!("Fallback enigo init failed: {:?}", e);
            return;
        }
    };
    let _ = enigo.key(Key::Meta, Direction::Press);
    let _ = enigo.key(Key::Unicode('v'), Direction::Click);
    let _ = enigo.key(Key::Meta, Direction::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_paste_text_empty_string() {
        paste_text("");
    }

    #[test]
    #[ignore]
    fn test_paste_text_normal_string() {
        paste_text("Test text");
    }

    #[test]
    #[ignore]
    fn test_paste_text_special_characters() {
        paste_text("Test with émojis 🎉 and spëcial çhars");
    }

    #[test]
    #[ignore]
    fn test_paste_text_long_string() {
        let long_text = "A".repeat(10000);
        paste_text(&long_text);
    }
}
