# 01. Архитектура системы и модель взаимодействия

Этот документ описывает общую структуру `maccy-kde` и взаимодействие между процессами.

## 1. Концепция единого бинарника
Проект компилируется в один исполняемый файл `maccy-kde`. Поведение определяется флагами командной строки:
- `maccy-kde --daemon` — запускает долгоживущий фоновый процесс (ядро).
- `maccy-kde --popup` — запускает мгновенное графическое окно интерфейса.

## 2. Схема межпроцессного взаимодействия (IPC)
Связь между `--daemon` и `--popup` осуществляется через Tokio Unix Domain Sockets (UDS).
- **Путь к сокету в Linux (Flatpak/Host):** `/run/user/{UID}/maccy-kde.socket`
- **Путь к сокету в macOS (Для локальной разработки):** `/tmp/maccy-kde.socket`

### Протокол обмена (структуры JSON через сокет):
Клиент (`--popup`) отправляет команды, Демон (`--daemon`) присылает ответы.

```rust
// Примеры структур для сериализации/десериализации через serde
enum IpcCommand {
    GetHistory,                  // Запрос списка элементов
    SelectItem { id: i64 },      // Запрос на вставку элемента в активное окно
    TogglePin { id: i64 },       // Переключить статус закрепления
    DeleteItem { id: i64 },      // Удалить элемент из истории
}

enum IpcResponse {
    History(Vec<ClipboardItem>),
    Success,
    Error(String),
}
```

## 3. Кроссплатформенность путей (Разработка на macOS -> Деплой на Linux)
ИИ-агент должен использовать условную компиляцию (`#[cfg(target_os = "linux")]`) или рантайм-проверку для определения путей:

| Ресурс | Путь на Linux (Целевой) | Путь на macOS (Разработка) |
| :--- | :--- | :--- |
| **База Данных** | `~/.local/share/maccy-kde/history.db` | `~/Library/Application Support/maccy-kde/history.db` |
| **IPC Сокет** | `/run/user/{uid}/maccy-kde.socket` | `/tmp/maccy-kde.socket` |
| **Кэш картинок**| `~/.cache/maccy-kde/images/` | `~/Library/Caches/maccy-kde/images/` |
