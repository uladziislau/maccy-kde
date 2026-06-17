use std::io;

pub type Result<T> = std::result::Result<T, MaccyError>;

#[derive(Debug)]
pub enum MaccyError {
    Io(io::Error),
    Database(String),
    Clipboard(String),
    Ipc(String),
    Paste(String),
    Autostart(String),
    Serialization(String),
    Validation(String),
}

impl std::fmt::Display for MaccyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaccyError::Io(err) => write!(f, "IO error: {}", err),
            MaccyError::Database(msg) => write!(f, "Database error: {}", msg),
            MaccyError::Clipboard(msg) => write!(f, "Clipboard error: {}", msg),
            MaccyError::Ipc(msg) => write!(f, "IPC error: {}", msg),
            MaccyError::Paste(msg) => write!(f, "Paste error: {}", msg),
            MaccyError::Autostart(msg) => write!(f, "Autostart error: {}", msg),
            MaccyError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            MaccyError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for MaccyError {}

impl From<io::Error> for MaccyError {
    fn from(err: io::Error) -> Self {
        MaccyError::Io(err)
    }
}

impl From<rusqlite::Error> for MaccyError {
    fn from(err: rusqlite::Error) -> Self {
        MaccyError::Database(err.to_string())
    }
}