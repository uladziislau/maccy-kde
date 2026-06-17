use crate::domain::entities::{ClipboardItem, Category};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct FuzzySearchService {
    matcher: SkimMatcherV2,
}

impl FuzzySearchService {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Filter clipboard items using fuzzy search with category support
    pub fn search<'a>(
        &'a self,
        items: &'a [ClipboardItem],
        query: &str,
    ) -> Vec<&'a ClipboardItem> {
        if query.is_empty() {
            return items.iter().collect();
        }

        // Check for category filter syntax (@url, @email, @account, @picture, @other)
        let (category_filter, search_query) = if query.starts_with('@') {
            let parts: Vec<&str> = query.splitn(2, ' ').collect();
            let category_str = parts[0].to_lowercase();
            let remaining_query = if parts.len() > 1 { parts[1].trim() } else { "" };
            
            let category_filter = match category_str.as_str() {
                "@url" => Some(Category::Url),
                "@email" => Some(Category::Email),
                "@account" => Some(Category::Account),
                "@picture" => Some(Category::Picture),
                "@other" => Some(Category::Other),
                _ => None,
            };
            
            (category_filter, remaining_query)
        } else {
            (None, query)
        };

        let mut scored: Vec<_> = items
            .iter()
            .filter_map(|item| {
                // Apply category filter if specified
                if let Some(ref cat) = category_filter {
                    if item.category != Some(cat.clone()) {
                        return None;
                    }
                }
                
                // Apply text search if query is not empty
                if !search_query.is_empty() {
                    let search_text = Self::get_search_text(item);
                    self.matcher.fuzzy_match(search_text, search_query)
                        .map(|score| (item, score))
                } else {
                    Some((item, 100)) // Default score for category-only filter
                }
            })
            .collect();
        
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(item, _)| item).collect()
    }

    fn get_search_text<'a>(item: &'a ClipboardItem) -> &'a str {
        match &item.content {
            crate::domain::entities::Content::Text(text) => text,
            crate::domain::entities::Content::Image(_) => "Изображение",
        }
    }
}

impl Default for FuzzySearchService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Content, ItemId, MimeType, Timestamp};

    fn create_test_item(id: i64, text: &str) -> ClipboardItem {
        ClipboardItem::new(
            ItemId(id),
            Content::Text(text.to_string()),
            MimeType::text_plain(),
            None,
        )
    }

    fn create_image_item(id: i64) -> ClipboardItem {
        ClipboardItem::new(
            ItemId(id),
            Content::Image(std::path::PathBuf::from("/test/image.png")),
            MimeType::image_png(),
            Some(Category::Picture),
        )
    }

    #[test]
    fn test_search_empty_query() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Test Item"),
        ];
        
        let result = service.search(&items, "");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_search_exact_match() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Test Item"),
        ];
        
        let result = service.search(&items, "Hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(1));
    }

    #[test]
    fn test_search_partial_match() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Hello There"),
            create_test_item(3, "Test Item"),
        ];
        
        let result = service.search(&items, "He");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_search_no_match() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "Test Item"),
        ];
        
        let result = service.search(&items, "xyz");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_search_cyrillic() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_test_item(1, "Привет мир"),
            create_test_item(2, "Тестовый элемент"),
        ];
        
        let result = service.search(&items, "Прив");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(1));
    }

    #[test]
    fn test_search_emoji() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_test_item(1, "Hello 🌍"),
            create_test_item(2, "Test 🚀"),
        ];
        
        let result = service.search(&items, "🌍");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(1));
    }

    #[test]
    fn test_search_image_item() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_image_item(1),
            create_test_item(2, "Test Item"),
        ];
        
        let result = service.search(&items, "Изображение");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(1));
    }

    #[test]
    fn test_search_category_url() {
        let service = FuzzySearchService::new();
        let items = vec![
            ClipboardItem::new(
                ItemId(1),
                Content::Text("https://example.com".to_string()),
                MimeType::text_plain(),
                Some(Category::Url),
            ),
            ClipboardItem::new(
                ItemId(2),
                Content::Text("user@example.com".to_string()),
                MimeType::text_plain(),
                Some(Category::Email),
            ),
        ];

        let result = service.search(&items, "@url");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(1));
    }

    #[test]
    fn test_search_category_email() {
        let service = FuzzySearchService::new();
        let items = vec![
            ClipboardItem::new(
                ItemId(1),
                Content::Text("https://example.com".to_string()),
                MimeType::text_plain(),
                Some(Category::Url),
            ),
            ClipboardItem::new(
                ItemId(2),
                Content::Text("user@example.com".to_string()),
                MimeType::text_plain(),
                Some(Category::Email),
            ),
        ];

        let result = service.search(&items, "@email");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(2));
    }

    #[test]
    fn test_search_category_with_text() {
        let service = FuzzySearchService::new();
        let items = vec![
            ClipboardItem::new(
                ItemId(1),
                Content::Text("https://github.com".to_string()),
                MimeType::text_plain(),
                Some(Category::Url),
            ),
            ClipboardItem::new(
                ItemId(2),
                Content::Text("https://example.com".to_string()),
                MimeType::text_plain(),
                Some(Category::Url),
            ),
        ];

        let result = service.search(&items, "@url github");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(1));
    }

    #[test]
    fn test_search_case_insensitive() {
        let service = FuzzySearchService::new();
        let items = vec![
            create_test_item(1, "Hello World"),
            create_test_item(2, "HELLO THERE"),
        ];
        
        let result = service.search(&items, "hello");
        assert!(result.len() >= 1);
    }
}