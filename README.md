# maccy-kde 📋🦀

A lightweight, keyboard-first clipboard manager for Linux, built specifically for the **KDE Plasma 6** desktop environment running on **Wayland** (e.g., Fedora 44+). 

This project is a modern spiritual successor to macOS's [Maccy](https://github.com), built from scratch in **Rust** to deliver maximum performance, safety, and a minimal resource footprint (< 15 MB RAM).

---

<details>
<summary><b>🇷🇺 Читать описание на русском языке (Click to expand)</b></summary>

# maccy-kde 📋🦀

Легковесный, управляемый исключительно с клавиатуры менеджер буфера обмена для Linux, разработанный специально для графического окружения **KDE Plasma 6** на базе **Wayland** (например, Fedora 44+).

Этот проект — духовный наследник популярного macOS-приложения [Maccy](https://github.com), переписанный с нуля на **Rust** ради максимальной скорости, безопасности памяти и минимального потребления ресурсов системы (< 15 МБ ОЗУ).

### Ключевые особенности
- **Интерфейс в стиле macOS:** Минималистичное всплывающее меню прямо под курсором мыши.
- **Keyboard-first:** Навигация стрелочками, мгновенный нечеткий (fuzzy) поиск при вводе текста, вставка элементов по горячим клавишам `Alt+1` .. `Alt+9`.
- **Создано для Wayland:** Полная поддержка нативных протоколов безопасности Wayland без костылей из эпохи X11.
- **Архитектура Демон/Клиент:** Фоновый процесс изолирован от графического интерфейса, что гарантирует работу утилиты со скоростью < 15 мс.
- **Поддержка изображений:** Сохранение изображений из буфера обмена в кэш.
- **Автостарт:** Легкая установка и удаление автостарта для KDE Plasma.

### Установка

#### Из исходных файлов (для всех дистрибутивов)
1. Установите Rust: https://www.rust-lang.org/tools/install
2. Склонируйте репозиторий:
   ```bash
   git clone <repo-url>
   cd maccy-kde
   ```
3. Соберите и установите:
   ```bash
   cargo build --release
   sudo cp target/release/maccy-kde /usr/local/bin/
   ```
4. Установите автостарт (необязательно):
   ```bash
   maccy-kde --install-autostart
   ```

#### Для Fedora 44+ (RPM-пакет, в разработке)
В будущем будет доступен RPM-пакет для установки через `dnf`.

#### Flatpak (в разработке)
Сборка Flatpak будет доступна через `org.maccy_kde.ClipboardManager.yml`.

### Использование
1. Запустите демон: `maccy-kde --daemon`
2. Создайте глобальную горячую клавишу в KDE Plasma для запуска `maccy-kde --popup`
3. Используйте всплывающее окно для поиска и вставки элементов истории

</details>

---

## ✨ Key Features
- **Mac-like Aesthetics:** A clean, borderless popup menu styled seamlessly next to your mouse cursor.
- **Keyboard-first UX:** Arrow navigation, instantaneous fuzzy searching as you type, and quick-paste via `Alt+1` to `Alt+9`.
- **Wayland Native:** Designed entirely around modern Wayland security protocols via `smithay-clipboard`.
- **Daemon/Client Architecture:** A headless background service separates core logic from the UI, ensuring popup opening speeds under 15ms.
- **Image Support:** Save clipboard images to cache.
- **Autostart:** Easy installation and removal of KDE Plasma autostart.

## Installation

### From Source (all distributions)
1. Install Rust: https://www.rust-lang.org/tools/install
2. Clone the repository:
   ```bash
   git clone <repo-url>
   cd maccy-kde
   ```
3. Build and install:
   ```bash
   cargo build --release
   sudo cp target/release/maccy-kde /usr/local/bin/
   ```
4. Optionally install autostart:
   ```bash
   maccy-kde --install-autostart
   ```

### Fedora 44+ (RPM package, coming soon)
RPM package will be available via `dnf`.

### Flatpak (coming soon)
Flatpak build via `org.maccy_kde.ClipboardManager.yml`.

## Usage
1. Start the daemon: `maccy-kde --daemon`
2. Create a global keyboard shortcut in KDE Plasma for `maccy-kde --popup`
3. Use the popup to search and paste clipboard history

## 🏗️ Architecture Blueprint (Instruction for AI Agents)

The application compiles into a single binary, switching modes via CLI flags:

1. **Background Daemon (`maccy-kde --daemon`):** 
   - Runs headlessly on system startup.
   - Monitors the Wayland clipboard for changes using `smithay-clipboard`.
   - Handles de-duplication, history pruning (keeps up to 200 items), and persistent storage in a cross-platform local SQLite database.
2. **Graphical Popup (`maccy-kde --popup`):**
   - Triggered instantly via a system-wide KDE custom shortcut.
   - Built using **Slint UI** for blazing-fast hardware-accelerated rendering.
   - Communicates with the daemon via Unix Domain Sockets (IPC) to fetch historical items and run local fuzzy matching.
   - Pastes items back to the active window utilizing KDE Plasma's virtual keyboard DBus interfaces.

## 🛠️ Development & Git Workflow Rules

All AI agents and contributors must strictly adhere to the following workflow:

1. **Branching Strategy (GitHub Flow):** Never commit directly to the `main` branch. Create modular feature branches (e.g., `feature/database-core`, `feature/ipc-layer`).
2. **Commit Formatting:** Use [Conventional Commits](https://conventionalcommits.org):
   - `feat: ...` for new capabilities.
   - `fix: ...` for bugs.
   - `chore: ...` for dependencies or configuration tweaks.
3. **Storage Fallback:** For cross-platform safety during development on macOS, resolve the database path to `~/Library/Application Support/maccy-kde/history.db`, while on Linux target `~/.local/share/maccy-kde/history.db`.

## 📦 Technical Stack
- **Language:** Rust (Edition 2021)
- **Database:** SQLite (`rusqlite` bundled)
- **GUI Framework:** Slint UI
- **Clipboard Interface:** `smithay-clipboard` (Wayland protocols)
- **IPC Protocol:** Tokio Unix Domain Sockets
- **Fuzzy Matcher:** `fuzzy-matcher` crate
