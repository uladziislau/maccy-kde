use super::{ItemId, Content, Category, Timestamp, MimeType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: ItemId,
    pub content: Content,
    pub mime_type: MimeType,
    pub category: Option<Category>,
    pub is_pinned: bool,
    pub pin_order: i64,
    pub last_used_at: Timestamp,
}

impl ClipboardItem {
    pub fn new(
        id: ItemId,
        content: Content,
        mime_type: MimeType,
        category: Option<Category>,
    ) -> Self {
        Self {
            id,
            content,
            mime_type,
            category,
            is_pinned: false,
            pin_order: 0,
            last_used_at: Timestamp::now(),
        }
    }

    pub fn text(text: String, id: ItemId) -> Self {
        let category = Some(Category::from_text(&text));
        Self::new(
            id,
            Content::Text(text),
            MimeType::text_plain(),
            category,
        )
    }

    pub fn image(image_path: std::path::PathBuf, mime_type: MimeType, id: ItemId) -> Self {
        Self::new(
            id,
            Content::Image(image_path),
            mime_type,
            Some(Category::Picture),
        )
    }

    pub fn pinned(&self) -> bool {
        self.is_pinned
    }

    pub fn pin(&mut self, order: i64) {
        self.is_pinned = true;
        self.pin_order = order;
    }

    pub fn unpin(&mut self) {
        self.is_pinned = false;
        self.pin_order = 0;
    }

    pub fn update_last_used(&mut self) {
        self.last_used_at = Timestamp::now();
    }

    pub fn display_text(&self) -> String {
        match &self.content {
            Content::Text(text) if text.chars().count() > 100 => {
                let truncated: String = text.chars().take(100).collect();
                format!("{}…", truncated)
            },
            Content::Text(text) => text.clone(),
            Content::Image(_) => "📷 Изображение".to_string(),
        }
    }
}