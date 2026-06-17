use crate::domain::entities::{ClipboardItem, Category};

/// View model for displaying clipboard items in UI
#[derive(Debug, Clone, PartialEq)]
pub struct ItemViewModel {
    pub id: String,
    pub display_text: String,
    pub category_badge: Option<String>,
    pub is_pinned: bool,
    pub pin_order: i64,
    pub timestamp: String,
    pub mime_type: String,
}

impl ItemViewModel {
    /// Convert from domain entity to view model
    pub fn from_domain(item: &ClipboardItem) -> Self {
        let category_badge = item.category.as_ref().map(|cat| match cat {
            Category::Url => "🔗",
            Category::Email => "📧",
            Category::Account => "👤",
            Category::Picture => "🖼️",
            Category::Other => "📝",
        }.to_string());

        let display_text = item.display_text();
        let timestamp = format_timestamp(item.last_used_at.value());
        
        Self {
            id: item.id.to_string(),
            display_text: Self::truncate_text(&display_text, 100),
            category_badge,
            is_pinned: item.is_pinned,
            pin_order: item.pin_order,
            timestamp,
            mime_type: item.mime_type.value().to_string(),
        }
    }

    /// Truncate text to max length
    fn truncate_text(text: &str, max_length: usize) -> String {
        if text.len() > max_length {
            format!("{}...", &text[..max_length])
        } else {
            text.to_string()
        }
    }

    /// Get styled display text with HTML tags
    pub fn get_styled_text(&self) -> String {
        if let Some(badge) = &self.category_badge {
            format!("{} {}", badge, self.display_text)
        } else {
            self.display_text.clone()
        }
    }

    /// Get icon based on mime type
    pub fn get_icon(&self) -> &str {
        if self.mime_type.contains("image") {
            "image"
        } else if self.mime_type.contains("text") {
            "text"
        } else {
            "file"
        }
    }

    /// Check if item is image
    pub fn is_image(&self) -> bool {
        self.mime_type.contains("image")
    }

    /// Check if item is text
    pub fn is_text(&self) -> bool {
        self.mime_type.contains("text") || self.mime_type.contains("plain")
    }
}

/// Format timestamp for display
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};
    
    let dt = DateTime::<Utc>::from_timestamp(timestamp, 0);
    match dt {
        Some(dt) => {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(dt);
            
            if duration.num_seconds() < 60 {
                format!("{}s ago", duration.num_seconds())
            } else if duration.num_minutes() < 60 {
                format!("{}m ago", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("{}h ago", duration.num_hours())
            } else {
                dt.format("%Y-%m-%d").to_string()
            }
        }
        None => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Content, ItemId, MimeType, Timestamp};

    fn create_test_item(id: i64, text: &str, category: Option<Category>) -> ClipboardItem {
        ClipboardItem::new(
            ItemId(id),
            Content::Text(text.to_string()),
            MimeType::text_plain(),
            category,
        )
    }

    #[test]
    fn test_from_domain_text_item() {
        let item = create_test_item(1, "test text", None);
        let view_model = ItemViewModel::from_domain(&item);
        
        assert_eq!(view_model.id, "1");
        assert!(view_model.display_text.contains("test text"));
        assert!(!view_model.is_pinned);
    }

    #[test]
    fn test_from_domain_url_item() {
        let item = create_test_item(1, "https://example.com", Some(Category::Url));
        let view_model = ItemViewModel::from_domain(&item);
        
        assert_eq!(view_model.category_badge, Some("🔗".to_string()));
    }

    #[test]
    fn test_from_domain_email_item() {
        let item = create_test_item(1, "user@example.com", Some(Category::Email));
        let view_model = ItemViewModel::from_domain(&item);
        
        assert_eq!(view_model.category_badge, Some("📧".to_string()));
    }

    #[test]
    fn test_truncate_text_short() {
        let item = create_test_item(1, "short", None);
        let view_model = ItemViewModel::from_domain(&item);
        
        assert_eq!(view_model.display_text, "short");
    }

    #[test]
    fn test_truncate_text_long() {
        let long_text = "a".repeat(200);
        let item = create_test_item(1, &long_text, None);
        let view_model = ItemViewModel::from_domain(&item);
        
        assert!(view_model.display_text.len() <= 103); // 100 + "..."
        assert!(view_model.display_text.ends_with("..."));
    }

    #[test]
    fn test_get_styled_text_with_badge() {
        let item = create_test_item(1, "https://example.com", Some(Category::Url));
        let view_model = ItemViewModel::from_domain(&item);
        let styled = view_model.get_styled_text();
        
        assert!(styled.contains("🔗"));
    }

    #[test]
    fn test_get_styled_text_without_badge() {
        let item = create_test_item(1, "plain text", None);
        let view_model = ItemViewModel::from_domain(&item);
        let styled = view_model.get_styled_text();
        
        assert_eq!(styled, "plain text");
    }

    #[test]
    fn test_get_icon_text() {
        let item = create_test_item(1, "text", None);
        let view_model = ItemViewModel::from_domain(&item);
        
        assert_eq!(view_model.get_icon(), "text");
    }

    #[test]
    fn test_is_text() {
        let item = create_test_item(1, "text", None);
        let view_model = ItemViewModel::from_domain(&item);
        
        assert!(view_model.is_text());
        assert!(!view_model.is_image());
    }

    #[test]
    fn test_equality() {
        let item1 = create_test_item(1, "same", None);
        let item2 = create_test_item(1, "same", None);
        
        let vm1 = ItemViewModel::from_domain(&item1);
        let vm2 = ItemViewModel::from_domain(&item2);
        
        assert_eq!(vm1, vm2);
    }

    #[test]
    fn test_clone() {
        let item = create_test_item(1, "test", None);
        let view_model = ItemViewModel::from_domain(&item);
        let cloned = view_model.clone();
        
        assert_eq!(view_model, cloned);
    }
}