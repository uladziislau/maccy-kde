use crate::database::{ClipboardItem, Database, DataType};
use log::{info, error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

#[derive(Serialize, Deserialize, Debug)]
pub enum IpcCommand {
    GetHistory,
    SelectItem { id: i64 },
    TogglePin { id: i64 },
    DeleteItem { id: i64 },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum IpcResponse {
    History(Vec<ClipboardItem>),
    Success,
    Error(String),
}

pub fn get_socket_path() -> std::path::PathBuf {
    // Сначала проверяем переменную окружения для тестов
    if let Ok(path_str) = std::env::var("MACCY_KDE_SOCKET_PATH") {
        return std::path::PathBuf::from(path_str);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            return std::path::PathBuf::from(runtime_dir).join("maccy-kde.sock");
        }
    }
    std::env::temp_dir().join("maccy-kde.sock")
}

pub async fn start_ipc_server(db: Arc<Database>) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = get_socket_path();

    // Удаляем старый сокет, если он есть
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    info!("IPC server listening...");

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let db = db.clone();
                tokio::spawn(async move {
                    handle_client(stream, db).await;
                });
            }
            Err(e) => error!("Failed to accept connection: {}", e),
        }
    }
}

async fn handle_client(stream: UnixStream, db: Arc<Database>) {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    if let Ok(Some(line)) = lines.next_line().await {
        match serde_json::from_str::<IpcCommand>(&line) {
            Ok(cmd) => {
                let response = handle_command(cmd, db).await;
                let resp_json = match serde_json::to_string(&response) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize response: {}", e);
                        serde_json::to_string(&IpcResponse::Error("Serialization failed".into())).unwrap()
                    }
                };
                // Get the writer
                let mut stream = lines.into_inner().into_inner();
                if let Err(e) = stream.write_all(resp_json.as_bytes()).await {
                    error!("Failed to write response: {}", e);
                }
                let _ = stream.write_all(b"\n").await;
            }
            Err(e) => error!("Failed to parse command: {}", e),
        }
    }
}

async fn handle_command(cmd: IpcCommand, db: Arc<Database>) -> IpcResponse {
    match cmd {
        IpcCommand::GetHistory => match db.get_history() {
            Ok(items) => IpcResponse::History(items),
            Err(e) => IpcResponse::Error(format!("Failed to get history: {}", e)),
        },
        IpcCommand::SelectItem { id } => {
            if let Ok(history) = db.get_history() {
                if let Some(item) = history.iter().find(|i| i.id == id) {
                    match &item.data_type {
                        DataType::Text => {
                            if let Some(text) = &item.value_text {
                                let _ = db.add_text_item(text);
                                // Активировать буфер обмена и вставить
                                crate::paster::paste_text(text);
                            }
                        }
                        DataType::Image => {
                            // TODO: Активировать буфер обмена и вставить изображение
                        }
                    }
                    // Возвращаем обновленную историю
                    if let Ok(new_history) = db.get_history() {
                        return IpcResponse::History(new_history);
                    }
                }
            }
            IpcResponse::Error("Item not found".into())
        }
        IpcCommand::TogglePin { id } => match db.toggle_pin(id) {
            Ok(_) => match db.get_history() {
                Ok(items) => IpcResponse::History(items),
                Err(e) => IpcResponse::Error(format!("Failed to get history after toggle: {}", e)),
            },
            Err(e) => IpcResponse::Error(format!("Failed to toggle pin: {}", e)),
        },
        IpcCommand::DeleteItem { id } => match db.delete_item(id) {
            Ok(_) => match db.get_history() {
                Ok(items) => IpcResponse::History(items),
                Err(e) => IpcResponse::Error(format!("Failed to get history after delete: {}", e)),
            },
            Err(e) => IpcResponse::Error(format!("Failed to delete item: {}", e)),
        },
    }
}

pub async fn send_command(cmd: IpcCommand) -> Result<IpcResponse, Box<dyn std::error::Error>> {
    let socket_path = get_socket_path();
    let mut stream = UnixStream::connect(socket_path).await?;

    let cmd_json = serde_json::to_string(&cmd)?;
    stream.write_all(cmd_json.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    if let Ok(Some(line)) = lines.next_line().await {
        let resp = serde_json::from_str(&line)?;
        Ok(resp)
    } else {
        Err("No response from daemon".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ipc_get_history() {
        // Создаем временную базу данных
        let db = Arc::new(Database::in_memory().unwrap());

        // Добавляем тестовый элемент
        db.add_text_item("Hello IPC test").unwrap();

        // Создаем временную директорию для сокета
        let temp_dir = tempdir().unwrap();
        let temp_socket_path = temp_dir.path().join("test.sock");
        std::env::set_var("MACCY_KDE_SOCKET_PATH", temp_socket_path.to_str().unwrap());

        // Запускаем сервер в отдельной задаче
        let db_clone = db.clone();
        let server_handle = tokio::spawn(async move {
            let _ = start_ipc_server(db_clone).await;
        });

        // Дожидаемся запуска сервера
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Тестируем отправку команды
        let result = send_command(IpcCommand::GetHistory).await;
        assert!(result.is_ok());

        if let Ok(IpcResponse::History(items)) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].value_text, Some("Hello IPC test".to_string()));
        }

        // Завершаем сервер
        server_handle.abort();

        // Удаляем переменную окружения
        std::env::remove_var("MACCY_KDE_SOCKET_PATH");
    }
}
