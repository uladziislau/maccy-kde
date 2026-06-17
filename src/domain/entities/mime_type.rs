use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MimeType(String);

impl MimeType {
    pub fn new(mime_type: String) -> Self {
        Self(mime_type)
    }

    pub fn text_plain() -> Self {
        Self("text/plain".to_string())
    }

    pub fn image_png() -> Self {
        Self("image/png".to_string())
    }

    pub fn image_jpeg() -> Self {
        Self("image/jpeg".to_string())
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn is_text(&self) -> bool {
        self.0.starts_with("text/")
    }

    pub fn is_image(&self) -> bool {
        self.0.starts_with("image/")
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MimeType {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}