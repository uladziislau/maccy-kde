use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Content {
    Text(String),
    Image(PathBuf),
}

impl Content {
    pub fn is_text(&self) -> bool {
        matches!(self, Content::Text(_))
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Content::Image(_))
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Text(text) => Some(text),
            Content::Image(_) => None,
        }
    }

    pub fn as_image_path(&self) -> Option<&PathBuf> {
        match self {
            Content::Text(_) => None,
            Content::Image(path) => Some(path),
        }
    }
}