use maccy_kde::database::{ClipboardItem, DataType};
use maccy_kde::gui::GuiManager;
use slint::ComponentHandle;

#[test]
fn test_ui_item_mapping() {
    // Мы не можем легко инициализировать MaccyMenu без графического бэкенда в тестах,
    // но мы можем протестировать внутреннюю логику маппинга, если вынесем её.
    // Пока проверим, что структура ClipboardItem корректно определена.

    let item = ClipboardItem {
        id: 1,
        value_text: Some("Test".to_string()),
        image_path: None,
        data_type: DataType::Text,
        raw_mime_type: "text/plain".to_string(),
        is_pinned: false,
        pin_order: 0,
        last_used_at: 12345,
    };

    assert_eq!(item.id, 1);
    assert_eq!(item.value_text.unwrap(), "Test");
}
