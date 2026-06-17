use crate::domain::entities::{ClipboardItem, Category};

#[allow(dead_code)] // Used in PopupService
pub struct CategoryFilterService;

#[allow(dead_code)] // Methods used in PopupService
impl CategoryFilterService {
    /// Filter items by category
    pub fn filter_by_category<'a>(items: &'a [ClipboardItem], category: &Category) -> Vec<&'a ClipboardItem> {
        items.iter()
            .filter(|item| item.category.as_ref() == Some(category))
            .collect()
    }

    /// Filter items by multiple categories
    pub fn filter_by_categories<'a>(items: &'a [ClipboardItem], categories: &[Category]) -> Vec<&'a ClipboardItem> {
        items.iter()
            .filter(|item| {
                item.category.as_ref()
                    .map(|cat| categories.contains(cat))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get items without category
    pub fn filter_without_category(items: &[ClipboardItem]) -> Vec<&ClipboardItem> {
        items.iter()
            .filter(|item| item.category.is_none())
            .collect()
    }

    /// Count items by category
    pub fn count_by_category(items: &[ClipboardItem], category: &Category) -> usize {
        items.iter()
            .filter(|item| item.category.as_ref() == Some(category))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Content, ItemId, MimeType};

    fn create_item(id: i64, text: &str, category: Option<Category>) -> ClipboardItem {
        ClipboardItem::new(
            ItemId(id),
            Content::Text(text.to_string()),
            MimeType::text_plain(),
            category,
        )
    }

    #[test]
    fn test_filter_by_category_url() {
        let items = vec![
            create_item(1, "https://example.com", Some(Category::Url)),
            create_item(2, "user@example.com", Some(Category::Email)),
            create_item(3, "plain text", Some(Category::Other)),
        ];

        let result = CategoryFilterService::filter_by_category(&items, &Category::Url);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(1));
    }

    #[test]
    fn test_filter_by_multiple_categories() {
        let items = vec![
            create_item(1, "https://example.com", Some(Category::Url)),
            create_item(2, "https://github.com", Some(Category::Url)),
            create_item(3, "user@example.com", Some(Category::Email)),
            create_item(4, "plain text", Some(Category::Other)),
        ];

        let categories = vec![Category::Url, Category::Email];
        let result = CategoryFilterService::filter_by_categories(&items, &categories);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_filter_without_category() {
        let items = vec![
            create_item(1, "https://example.com", Some(Category::Url)),
            create_item(2, "uncategorized text", None),
            create_item(3, "user@example.com", Some(Category::Email)),
        ];

        let result = CategoryFilterService::filter_without_category(&items);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, ItemId(2));
    }

    #[test]
    fn test_count_by_category() {
        let items = vec![
            create_item(1, "https://example.com", Some(Category::Url)),
            create_item(2, "https://github.com", Some(Category::Url)),
            create_item(3, "user@example.com", Some(Category::Email)),
        ];

        let count = CategoryFilterService::count_by_category(&items, &Category::Url);
        assert_eq!(count, 2);

        let email_count = CategoryFilterService::count_by_category(&items, &Category::Email);
        assert_eq!(email_count, 1);
    }

    #[test]
    fn test_filter_empty_items() {
        let items: Vec<ClipboardItem> = vec![];
        let result = CategoryFilterService::filter_by_category(&items, &Category::Url);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_no_match() {
        let items = vec![
            create_item(1, "https://example.com", Some(Category::Url)),
            create_item(2, "user@example.com", Some(Category::Email)),
        ];

        let result = CategoryFilterService::filter_by_category(&items, &Category::Account);
        assert_eq!(result.len(), 0);
    }
}