use crate::cli::commands::{DaemonCommand, PopupCommand, ItemCommand};
use crate::domain::repositories::ClipboardRepository;
use crate::shared::Result;
use std::sync::Arc;

/// CLI parser and executor
pub struct CliParser {
    repository: Arc<dyn ClipboardRepository>,
    daemon_command: Option<DaemonCommand>,
    popup_command: Option<PopupCommand>,
    item_command: ItemCommand,
}

impl CliParser {
    pub fn new(repository: Arc<dyn ClipboardRepository>) -> Self {
        Self {
            repository: repository.clone(),
            daemon_command: None,
            popup_command: None,
            item_command: ItemCommand::new(repository),
        }
    }

    /// Execute CLI command from arguments
    pub fn execute(&mut self, args: &[String]) -> Result<CliResult> {
        if args.is_empty() {
            return Ok(CliResult::Help(self.get_help()));
        }

        match args[0].as_str() {
            "daemon" => self.execute_daemon(&args[1..]),
            "popup" => self.execute_popup(&args[1..]),
            "item" => self.execute_item(&args[1..]),
            "help" | "--help" | "-h" => Ok(CliResult::Help(self.get_help())),
            _ => Ok(CliResult::Error(format!("Unknown command: {}", args[0]))),
        }
    }

    fn execute_daemon(&mut self, args: &[String]) -> Result<CliResult> {
        if self.daemon_command.is_none() {
            self.daemon_command = Some(DaemonCommand::new(self.repository.clone()));
        }
        
        let command = self.daemon_command.as_mut().unwrap();
        
        if args.is_empty() {
            let status = command.status()?;
            return Ok(CliResult::DaemonStatus(status));
        }

        match args[0].as_str() {
            "start" => {
                command.start()?;
                Ok(CliResult::Success("Daemon started".to_string()))
            }
            "stop" => {
                command.stop()?;
                Ok(CliResult::Success("Daemon stopped".to_string()))
            }
            "restart" => {
                command.restart()?;
                Ok(CliResult::Success("Daemon restarted".to_string()))
            }
            "status" => {
                let status = command.status()?;
                Ok(CliResult::DaemonStatus(status))
            }
            _ => Ok(CliResult::Error(format!("Unknown daemon command: {}", args[0]))),
        }
    }

    fn execute_popup(&mut self, args: &[String]) -> Result<CliResult> {
        if self.popup_command.is_none() {
            self.popup_command = Some(PopupCommand::new(self.repository.clone(), 100));
        }
        
        let command = self.popup_command.as_mut().unwrap();
        
        if args.is_empty() {
            let result = command.show()?;
            return Ok(CliResult::PopupShown(result.item_count));
        }

        match args[0].as_str() {
            "show" => {
                let result = command.show()?;
                Ok(CliResult::PopupShown(result.item_count))
            }
            "hide" => {
                command.hide()?;
                Ok(CliResult::Success("Popup hidden".to_string()))
            }
            "search" => {
                if args.len() < 2 {
                    return Ok(CliResult::Error("Search query required".to_string()));
                }
                let result = command.search(args[1].clone())?;
                Ok(CliResult::PopupShown(result.item_count))
            }
            "up" => {
                command.move_selection(crate::cli::commands::SelectionDirection::Up)?;
                Ok(CliResult::Success("Selection moved up".to_string()))
            }
            "down" => {
                command.move_selection(crate::cli::commands::SelectionDirection::Down)?;
                Ok(CliResult::Success("Selection moved down".to_string()))
            }
            "paste" => {
                let content = command.select_and_paste()?;
                match content {
                    Some(text) => Ok(CliResult::Pasted(text)),
                    None => Ok(CliResult::Error("No selection".to_string())),
                }
            }
            _ => Ok(CliResult::Error(format!("Unknown popup command: {}", args[0]))),
        }
    }

    fn execute_item(&mut self, args: &[String]) -> Result<CliResult> {
        let command = &self.item_command;
        
        if args.is_empty() {
            let items = command.list_recent(10)?;
            return Ok(CliResult::ItemList(items));
        }

        match args[0].as_str() {
            "list" => {
                let limit = if args.len() > 1 {
                    args[1].parse::<usize>().unwrap_or(10)
                } else {
                    10
                };
                let items = command.list_recent(limit)?;
                Ok(CliResult::ItemList(items))
            }
            "pinned" => {
                let items = command.list_pinned()?;
                Ok(CliResult::ItemList(items))
            }
            "delete" => {
                if args.len() < 2 {
                    return Ok(CliResult::Error("Item ID required".to_string()));
                }
                command.delete(&args[1])?;
                Ok(CliResult::Success(format!("Item {} deleted", args[1])))
            }
            "pin" => {
                if args.len() < 2 {
                    return Ok(CliResult::Error("Item ID required".to_string()));
                }
                command.toggle_pin(&args[1])?;
                Ok(CliResult::Success(format!("Item {} pinned/unpinned", args[1])))
            }
            "clear" => {
                let deleted = command.clear_unpinned()?;
                Ok(CliResult::Success(format!("Cleared {} unpinned items", deleted)))
            }
            "count" => {
                let count = command.count()?;
                Ok(CliResult::Count(count))
            }
            _ => Ok(CliResult::Error(format!("Unknown item command: {}", args[0]))),
        }
    }

    fn get_help(&self) -> String {
        format!(
            "maccy-kde CLI v{}\n\n\
            Commands:\n\
            daemon [start|stop|restart|status] - Manage daemon\n\
            popup [show|hide|search <query>|up|down|paste] - Manage popup\n\
            item [list <n>|pinned|delete <id>|pin <id>|clear|count] - Manage items\n\
            help - Show this help",
            env!("CARGO_PKG_VERSION")
        )
    }
}

#[derive(Debug, Clone)]
pub enum CliResult {
    Help(String),
    Success(String),
    Error(String),
    DaemonStatus(crate::cli::commands::DaemonStatus),
    PopupShown(usize),
    Pasted(String),
    ItemList(Vec<crate::cli::commands::ItemSummary>),
    Count(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ClipboardItem, Content, ItemId, MimeType};

    struct MockRepo {
        items: std::sync::Mutex<Vec<ClipboardItem>>,
    }

    impl MockRepo {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                items: std::sync::Mutex::new(Vec::new()),
            })
        }

        fn add_item(&self, id: i64, text: &str) {
            let mut items = self.items.lock().unwrap();
            items.push(ClipboardItem::new(
                ItemId(id),
                Content::Text(text.to_string()),
                MimeType::text_plain(),
                None,
            ));
        }
    }

    impl ClipboardRepository for MockRepo {
        fn save(&self, _item: &ClipboardItem) -> Result<()> {
            Ok(())
        }

        fn find_by_id(&self, _id: ItemId) -> Result<Option<ClipboardItem>> {
            Ok(None)
        }

        fn find_all(&self) -> Result<Vec<ClipboardItem>> {
            let items = self.items.lock().unwrap();
            Ok(items.clone())
        }

        fn find_recent(&self, _limit: usize) -> Result<Vec<ClipboardItem>> {
            Ok(Vec::new())
        }

        fn find_pinned(&self) -> Result<Vec<ClipboardItem>> {
            Ok(Vec::new())
        }

        fn delete(&self, _id: ItemId) -> Result<()> {
            Ok(())
        }

        fn update_pin(&self, _id: ItemId, _pinned: bool, _order: i64) -> Result<()> {
            Ok(())
        }

        fn toggle_pin(&self, _id: ItemId) -> Result<()> {
            Ok(())
        }

        fn count(&self) -> Result<usize> {
            let items = self.items.lock().unwrap();
            Ok(items.len())
        }

        fn rotate_history(&self, _max_items: usize) -> Result<()> {
            Ok(())
        }

        fn find_by_content(&self, _content: &str) -> Result<Option<ClipboardItem>> {
            Ok(None)
        }

        fn update_last_used(&self, _id: ItemId) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_cli_parser_creation() {
        let repo = MockRepo::new();
        let mut parser = CliParser::new(repo);
        
        // Test empty args returns help
        let result = parser.execute(&[]).unwrap();
        assert!(matches!(result, CliResult::Help(_)));
    }

    #[test]
    fn test_execute_help() {
        let repo = MockRepo::new();
        let mut parser = CliParser::new(repo);
        
        let result = parser.execute(&[String::from("help")]).unwrap();
        assert!(matches!(result, CliResult::Help(_)));
    }

    #[test]
    fn test_execute_unknown_command() {
        let repo = MockRepo::new();
        let mut parser = CliParser::new(repo);
        
        let result = parser.execute(&[String::from("unknown")]).unwrap();
        assert!(matches!(result, CliResult::Error(_)));
    }

    #[test]
    fn test_execute_daemon_start() {
        let repo = MockRepo::new();
        let mut parser = CliParser::new(repo);
        
        let args = vec![
            String::from("daemon"),
            String::from("start"),
        ];
        let result = parser.execute(&args).unwrap();
        
        assert!(matches!(result, CliResult::Success(_)));
    }

    #[test]
    fn test_execute_item_count() {
        let repo = MockRepo::new();
        repo.add_item(1, "item");
        
        let mut parser = CliParser::new(repo);
        let args = vec![
            String::from("item"),
            String::from("count"),
        ];
        let result = parser.execute(&args).unwrap();
        
        assert!(matches!(result, CliResult::Count(_)));
    }

    #[test]
    fn test_execute_item_list() {
        let repo = MockRepo::new();
        repo.add_item(1, "item1");
        repo.add_item(2, "item2");
        
        let mut parser = CliParser::new(repo);
        let args = vec![
            String::from("item"),
            String::from("list"),
        ];
        let result = parser.execute(&args).unwrap();
        
        assert!(matches!(result, CliResult::ItemList(_)));
    }
}