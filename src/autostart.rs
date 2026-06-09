use std::fs;
use std::path::PathBuf;
use log::{info, error};

const DESKTOP_FILE_CONTENT: &str = r#"[Desktop Entry]
Type=Application
Name=maccy-kde
Comment=Clipboard manager for KDE Plasma
Exec=maccy-kde --daemon
Icon=edit-paste
Terminal=false
X-KDE-autostart-after=plasma-workspace.target
"#;

fn get_autostart_dir() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".config")
        });
    config_dir.join("autostart")
}

pub fn install_autostart() -> Result<(), Box<dyn std::error::Error>> {
    let autostart_dir = get_autostart_dir();
    fs::create_dir_all(&autostart_dir)?;

    let desktop_file_path = autostart_dir.join("maccy-kde.desktop");
    fs::write(&desktop_file_path, DESKTOP_FILE_CONTENT)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&desktop_file_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&desktop_file_path, permissions)?;
    }

    info!("Автостарт установлен: {:?}", desktop_file_path);
    Ok(())
}

pub fn remove_autostart() -> Result<(), Box<dyn std::error::Error>> {
    let desktop_file_path = get_autostart_dir().join("maccy-kde.desktop");
    if desktop_file_path.exists() {
        fs::remove_file(&desktop_file_path)?;
        info!("Автостарт удален: {:?}", desktop_file_path);
    } else {
        info!("Автостарт не найден, удаление не требуется");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::env;

    #[test]
    fn test_autostart() {
        // Сохраняем оригинальное значение XDG_CONFIG_HOME
        let original_xdg = env::var("XDG_CONFIG_HOME").ok();
        
        // Создаем временную директорию
        let temp_dir = tempdir().unwrap();
        env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        
        // Устанавливаем автостарт
        install_autostart().unwrap();
        
        // Проверяем, что файл был создан
        let desktop_file_path = get_autostart_dir().join("maccy-kde.desktop");
        assert!(desktop_file_path.exists());
        
        // Проверяем содержимое файла
        let content = fs::read_to_string(&desktop_file_path).unwrap();
        assert_eq!(content, DESKTOP_FILE_CONTENT);
        
        // Удаляем автостарт
        remove_autostart().unwrap();
        
        // Проверяем, что файл был удален
        assert!(!desktop_file_path.exists());
        
        // Восстанавливаем оригинальное значение XDG_CONFIG_HOME
        if let Some(val) = original_xdg {
            env::set_var("XDG_CONFIG_HOME", val);
        } else {
            env::remove_var("XDG_CONFIG_HOME");
        }
    }
}
