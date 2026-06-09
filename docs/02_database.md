# 02. Слой хранения данных (SQLite Database Core)

Этот документ описывает структуру базы данных и правила управления историей буфера обмена.

## 1. Схема базы данных (SQLite + rusqlite)
При старте демона проверяется наличие файла БД. Если файла нет, он создается, и выполняется следующая миграция:

```sql
CREATE TABLE IF NOT EXISTS clipboard_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    value_text TEXT,                    -- Текстовое содержимое (NULL, если это только картинка)
    image_path TEXT,                    -- Путь к файлу картинки в кэше (NULL, если это текст)
    data_type TEXT NOT NULL,            -- 'text' или 'image'
    raw_mime_type TEXT NOT NULL,         -- Например, 'text/plain' или 'image/png'
    created_at INTEGER NOT NULL,        -- Unix timestamp
    last_used_at INTEGER NOT NULL,      -- Unix timestamp (нужен для сортировки)
    usage_count INTEGER DEFAULT 1,      -- Количество вызовов элемента
    is_pinned INTEGER DEFAULT 0         -- 1 = закреплен (защищен от автоудаления), 0 = нет
);

-- Индекс для мгновенного поиска по времени использования
CREATE INDEX IF NOT EXISTS idx_items_last_used ON clipboard_items(last_used_at DESC);
```

## 2. Логика ротации и очистки истории (Правила для ИИ)
Максимальный размер истории — **200 элементов**. 
При добавлении нового элемента (`add_item`), если общее количество записей превышает 200, ИИ-агент должен выполнить транзакцию удаления:

```sql
DELETE FROM clipboard_items 
WHERE is_pinned = 0 
  AND id NOT IN (
      SELECT id FROM clipboard_items 
      ORDER BY is_pinned DESC, last_used_at DESC 
      LIMIT 200
  );
```

## 3. Оптимизация хранения изображений
Категорически запрещено хранить тяжелые бинарные BLOB картинок внутри SQLite, чтобы избежать фризов UI.
- Если перехвачена картинка, демон генерирует уникальное имя (UUID или хэш), сохраняет её как файл `.png` в папку кэша (см. `01_architecture.md`).
- В таблицу `clipboard_items` в поле `image_path` записывается относительный или абсолютный путь к этому файлу.
- При удалении записи из БД, соответствующий файл картинки должен физически удаляться с диска.
