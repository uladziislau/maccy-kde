use maccy_kde::database::{Database, ClipboardItem, DataType};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[test]
fn test_fuzzy_search_logic() {
    let items = vec![
        create_test_item(1, "Apple"),
        create_test_item(2, "Banana"),
        create_test_item(3, "Apricot"),
    ];

    let matcher = SkimMatcherV2::default();
    let query = "Ap";

    let mut scored: Vec<_> = items
        .into_iter()
        .filter_map(|item| {
            matcher.fuzzy_match(item.value_text.as_ref().unwrap(), query).map(|score| (item, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    assert_eq!(scored.len(), 2);
    assert_eq!(scored[0].0.value_text.as_ref().unwrap(), "Apple");
    assert_eq!(scored[1].0.value_text.as_ref().unwrap(), "Apricot");
}

fn create_test_item(id: i64, text: &str) -> ClipboardItem {
    ClipboardItem {
        id,
        value_text: Some(text.to_string()),
        image_path: None,
        data_type: DataType::Text,
        raw_mime_type: "text/plain".to_string(),
        is_pinned: false,
        pin_order: 0,
        last_used_at: 0,
    }
}
