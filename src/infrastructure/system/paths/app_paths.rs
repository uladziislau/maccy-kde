use std::path::PathBuf;

pub struct AppPaths;

impl AppPaths {
    pub fn data_dir() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            let data_home = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| std::env::temp_dir().display().to_string());
                    PathBuf::from(home).join(".local").join("share")
                });
            data_home.join("maccy-kde")
        }

        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/tmp")
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            std::env::temp_dir().join("maccy-kde")
        }
    }

    pub fn cache_dir() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            let cache_home = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| std::env::temp_dir().display().to_string());
                    PathBuf::from(home).join(".cache")
                });
            cache_home.join("maccy-kde")
        }

        #[cfg(target_os = "macos")]
        {
            std::env::temp_dir().join("maccy-kde-cache")
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            std::env::temp_dir().join("maccy-kde-cache")
        }
    }

    pub fn database_path() -> PathBuf {
        Self::data_dir().join("history.db")
    }

    pub fn images_cache_path() -> PathBuf {
        Self::cache_dir().join("images")
    }

    pub fn socket_path() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
                return PathBuf::from(runtime_dir).join("maccy-kde.sock");
            }
        }

        std::env::temp_dir().join("maccy-kde.sock")
    }

    pub fn autostart_dir() -> PathBuf {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".config")
            });
        config_dir.join("autostart")
    }

    pub fn ensure_directories() -> std::io::Result<()> {
        std::fs::create_dir_all(Self::data_dir())?;
        std::fs::create_dir_all(Self::cache_dir())?;
        std::fs::create_dir_all(Self::images_cache_path())?;
        std::fs::create_dir_all(Self::autostart_dir())?;
        Ok(())
    }
}