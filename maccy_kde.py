#!/usr/bin/env python3
"""maccy-kde: keyboard-first clipboard manager for KDE Plasma 6."""

import json
import os
import re
import socket
import subprocess
import sys
import time
import threading
from pathlib import Path

from PyQt6.QtCore import QObject, QUrl, QTimer, pyqtSignal, pyqtSlot, pyqtProperty
from PyQt6.QtDBus import QDBusConnection, QDBusInterface
from PyQt6.QtQml import QQmlApplicationEngine
from PyQt6.QtWidgets import QApplication

# --- Constants ---

DATA_DIR = Path.home() / ".local" / "share" / "maccy-kde"
PINS_FILE = DATA_DIR / "pins.json"
HIDDEN_FILE = DATA_DIR / "hidden.json"

DBUS_SERVICE = "org.kde.klipper"
DBUS_PATH = "/klipper"
DBUS_INTERFACE = "org.kde.klipper.klipper"

SOCKET_PATH = os.environ.get("XDG_RUNTIME_DIR", "/tmp") + "/maccy-kde.sock"

QML_FILE = Path(__file__).resolve().parent / "maccy_kde.qml"
if not QML_FILE.exists():
    QML_FILE = Path("/usr/share/maccy-kde/maccy_kde.qml")

# --- Data model ---

from dataclasses import dataclass

@dataclass
class Clip:
    """One clipboard entry."""
    text: str
    category: str = ""
    pinned: bool = False
    shortcut: str = ""

    def __post_init__(self):
        if not self.category:
            self.category = detect_category(self.text)

    def matches(self, query: str) -> bool:
        """Fuzzy match — True if all query chars appear in order."""
        if not query:
            return True
        query = query.lower()
        text = self.text.lower()
        qi = ti = 0
        while qi < len(query) and ti < len(text):
            if query[qi] == text[ti]:
                qi += 1
            ti += 1
        return qi == len(query)

    def to_dict(self) -> dict:
        return {"text": self.text, "category": self.category,
                "pinned": self.pinned, "shortcut": self.shortcut,
                "section": "Pinned" if self.pinned else "History"}


# --- JSON persistence ---

class ClipStore:
    """Load/save pins and hidden items."""

    def __init__(self, data_dir: Path):
        self._pins_file = data_dir / "pins.json"
        self._hidden_file = data_dir / "hidden.json"
        self._pins: set[str] = set()
        self._hidden: set[str] = set()
        self._load()

    def _load(self):
        self._pins = set(self._read(self._pins_file))
        self._hidden = set(self._read(self._hidden_file))

    def _save(self):
        self._write(self._pins_file, list(self._pins))
        self._write(self._hidden_file, list(self._hidden))

    @staticmethod
    def _read(path: Path) -> list:
        try:
            with open(path) as f:
                return json.load(f)
        except (FileNotFoundError, json.JSONDecodeError):
            return []

    @staticmethod
    def _write(path: Path, data: list):
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_suffix(".tmp")
        with open(tmp, "w") as f:
            json.dump(data, f)
        tmp.replace(path)

    @property
    def pins(self) -> set[str]:
        return self._pins

    @property
    def hidden(self) -> set[str]:
        return self._hidden

    def toggle_pin(self, text: str):
        if text in self._pins:
            self._pins.discard(text)
        else:
            self._pins.add(text)
        self._save()

    def clear_unpinned(self):
        self._pins.clear()
        self._save()


# Suppress harmless Wayland virtual keyboard warning
os.environ.setdefault("QT_IM_MODULE", "none")

CATEGORIES = ["All", "URLs", "Emails", "Paths", "Codes", "Numbers"]
CATEGORY_RE = {
    "URLs": re.compile(r'^https?://\S+$', re.I),
    "Emails": re.compile(r'^[\w.+-]+@[\w-]+\.[\w.]+$'),
    "Paths": re.compile(r'^/[\w/.\-~]+$'),
    "Codes": re.compile(r'^[a-zA-Z0-9+/=]{8,}$'),
    "Numbers": re.compile(r'^\d+(\.\d+)?$'),
}

AUTOSTART_DESKTOP = """\
[Desktop Entry]
Type=Application
Name=maccy-kde daemon
Exec={executable}
Terminal=false
NoDisplay=true
X-KDE-autostart-phase=2
"""

HOTKEY_DESKTOP = """\
[Desktop Entry]
Type=Application
Name=maccy-kde toggle
Exec={executable} --toggle
Terminal=false
NoDisplay=true
X-KDE-Shortcuts=Meta+Shift+C
"""

# --- Category detection ---

def detect_category(text):
    for cat, pattern in CATEGORY_RE.items():
        if pattern.match(text.strip()):
            return cat
    return "Other"

# --- Paste helper ---

def paste_text(text):
    # 1. Set Klipper clipboard via D-Bus
    klipper = QDBusInterface(DBUS_SERVICE, DBUS_PATH, DBUS_INTERFACE)
    klipper.call("setClipboardContents", text)

    # 2. Simulate Ctrl+V to paste
    # 2. Simulate Ctrl+V via wtype/ydotool/wl-copy+xdotool
    for cmd in [
        ["wtype", "-M", "ctrl", "v", "-m", "ctrl"],
        ["ydotool", "key", "29:1", "25:1", "25:0", "29:0"],
    ]:
        try:
            subprocess.run(cmd, timeout=3)
            return
        except Exception:
            continue

    # 3. Last resort: try Qt's own paste via clipboard event (works in some apps)
    try:
        subprocess.run(["wtype", "-M", "shift", "insert", "-m", "shift"], timeout=2)
    except Exception:
        pass


def _resolve_executable():
    exe = os.path.abspath(sys.argv[0])
    if exe.endswith("maccy_kde.py"):
        # When run as python3 maccy_kde.py, use the wrapper
        exe = str(Path(exe).parent / "maccy-kde")
    return exe

def _autostart_path():
    return Path.home() / ".config" / "autostart" / "maccy-kde.desktop"

def _hotkey_path():
    return Path.home() / ".local" / "share" / "applications" / "maccy-kde.desktop"


# --- Socket helpers ---

def send_toggle():
    if not os.path.exists(SOCKET_PATH):
        print("maccy-kde is not running. Start it first.", file=sys.stderr)
        sys.exit(1)
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
            s.settimeout(3)
            s.connect(SOCKET_PATH)
            s.sendall(b"toggle\n")
    except (ConnectionRefusedError, ConnectionError):
        print("maccy-kde is not running. Start it first.", file=sys.stderr)
        sys.exit(1)


# --- Backend (QML bridge) ---

class Backend(QObject):
    itemsChanged = pyqtSignal()

    def __init__(self):
        super().__init__()
        self._raw_items: list[str] = []
        self._filtered: list[Clip] = []
        self._query = ""
        self._category = "All"
        self._store = ClipStore(DATA_DIR)
        self._klipper = None
        self._setup_klipper()

    @property
    def _pinned(self):
        return self._store.pins

    @property
    def _hidden(self):
        return self._store.hidden

    # --- Klipper D-Bus ---

    def _setup_klipper(self):
        try:
            bus = QDBusConnection.sessionBus()
            if not bus.isConnected():
                return
            self._klipper = QDBusInterface(DBUS_SERVICE, DBUS_PATH, DBUS_INTERFACE, bus)

            bus.connect(
                DBUS_SERVICE, DBUS_PATH, DBUS_INTERFACE,
                "clipboardHistoryUpdated",
                self._on_history_changed,
            )
        except Exception:
            pass

    def _on_history_changed(self, *args):
        QTimer.singleShot(500, self.fetch)

    @pyqtSlot()
    def fetch(self):
        if not self._klipper:
            return
        try:
            msg = self._klipper.call("getClipboardHistoryMenu")
            items = msg.arguments()[0] if msg.arguments() else []
        except Exception:
            items = []
        self._raw_items = list(dict.fromkeys(items))
        self._rebuild()

    # --- Items ---

    PIN_KEYS = ["Q","A","Z","W","S","X","E","D","C","R","F","V","T","G","B"]

    def _rebuild(self):
        clips: list[Clip] = []
        for text in self._raw_items:
            if text in self._hidden:
                continue
            clip = Clip(text)
            if self._category != "All" and clip.category != self._category:
                continue
            if self._query and not clip.matches(self._query):
                continue
            clip.pinned = text in self._pinned
            clips.append(clip)

        pinned = [c for c in clips if c.pinned]
        recent = [c for c in clips if not c.pinned]

        for i, c in enumerate(pinned):
            if i < len(self.PIN_KEYS):
                c.shortcut = self.PIN_KEYS[i]

        for i, c in enumerate(recent):
            n = i + 1
            if n <= 9:
                c.shortcut = str(n)

        self._filtered = pinned + recent
        self.itemsChanged.emit()

    # --- Slots for QML ---

    @pyqtSlot(str)
    def filter(self, query):
        self._query = query.strip()
        self._rebuild()

    @pyqtSlot(str)
    def filterByCategory(self, category):
        if category == "All":
            self._category = "All"
        else:
            self._category = category
        self._rebuild()

    @pyqtSlot(str)
    def paste(self, text):
        paste_text(text)

    @pyqtSlot(str)
    def togglePin(self, text):
        self._store.toggle_pin(text)
        self._rebuild()

    @pyqtSlot()
    def clearUnpinned(self):
        self._store.clear_unpinned()
        self._rebuild()

    # --- Properties ---

    @pyqtProperty(list, notify=itemsChanged)
    def itemsModel(self):
        return [c.to_dict() for c in self._filtered]

    @pyqtProperty(list, constant=True)
    def categories(self):
        return CATEGORIES

    @pyqtProperty(str, notify=itemsChanged)
    def activeCategory(self):
        return self._category


# --- Toggle socket listener ---

_notifiers = []  # keep refs to prevent GC

def start_toggle_listener(parent, show_fn):
    try:
        os.unlink(SOCKET_PATH)
    except FileNotFoundError:
        pass

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.bind(SOCKET_PATH)
    sock.listen(1)
    sock.setblocking(False)

    from PyQt6.QtCore import QSocketNotifier
    notifier = QSocketNotifier(sock.fileno(), QSocketNotifier.Type.Read, parent)
    _notifiers.append(notifier)

    def on_ready():
        try:
            conn, _ = sock.accept()
            data = conn.recv(1024)
            if data.strip() == b"toggle":
                show_fn()
            conn.close()
        except BlockingIOError:
            pass
        except Exception:
            pass

    notifier.activated.connect(on_ready)


# --- Watch mode (auto-restart on code changes) ---

def _start_watcher():
    import shutil
    watched = {}
    for f in [Path(__file__), QML_FILE]:
        if f.exists():
            watched[f] = f.stat().st_mtime

    def check():
        while True:
            time.sleep(0.5)
            for path, old_mtime in list(watched.items()):
                try:
                    if path.exists() and path.stat().st_mtime != old_mtime:
                        print(f"\n*** maccy-kde: {path.name} changed, restarting...", flush=True)
                        watched[path] = path.stat().st_mtime
                        cache = path.parent / "__pycache__"
                        if cache.exists():
                            shutil.rmtree(cache)
                        qml_cache = Path.home() / ".cache" / "maccy-kde" / "qmlcache"
                        if qml_cache.exists():
                            shutil.rmtree(qml_cache)
                        os.execvpe(sys.argv[0], sys.argv, os.environ)
                except Exception as e:
                    print(f"*** maccy-kde: restart failed ({e}), retrying...", flush=True)

    print("maccy-kde: watcher started, watching .py/.qml for changes", flush=True)
    t = threading.Thread(target=check, daemon=True)
    t.start()


# --- Main ---

def main():
    if len(sys.argv) > 1:
        arg = sys.argv[1]
        if arg == "--toggle":
            send_toggle()
            return
        elif arg == "--install-autostart":
            desktop = _autostart_path()
            desktop.parent.mkdir(parents=True, exist_ok=True)
            executable = _resolve_executable()
            desktop.write_text(AUTOSTART_DESKTOP.format(executable=executable))
            print(f"Autostart installed: {desktop}")
            return
        elif arg == "--remove-autostart":
            desktop = _autostart_path()
            if desktop.exists():
                desktop.unlink()
                print("Autostart removed")
            return
        elif arg == "--install-hotkey":
            desktop = _hotkey_path()
            desktop.parent.mkdir(parents=True, exist_ok=True)
            executable = _resolve_executable()
            desktop.write_text(HOTKEY_DESKTOP.format(executable=executable))
            print(f"Hotkey installed: Meta+Shift+C → {desktop}")
            print("KDE may need a moment to pick it up, or re-login.")
            return
        elif arg == "--remove-hotkey":
            desktop = _hotkey_path()
            if desktop.exists():
                desktop.unlink()
                print("Hotkey removed")
            return

    _start_watcher()

    app = QApplication(sys.argv)
    app.setApplicationName("maccy-kde")

    engine = QQmlApplicationEngine()

    backend = Backend()

    # Show/hide logic (called from QSocketNotifier — main thread)
    def toggle_window():
        if not qml_item:
            return
        if qml_item.isVisible():
            qml_item.hide()
        else:
            backend.fetch()
            from PyQt6.QtGui import QCursor
            cursor = QCursor.pos()
            qml_item.setX(cursor.x())
            qml_item.setY(cursor.y() - qml_item.height() // 2)
            qml_item.show()
            qml_item.requestActivate()

    qml_item = None

    def on_object_created(obj, url):
        nonlocal qml_item
        if not obj:
            print("Failed to load QML UI", flush=True)
            sys.exit(1)
        qml_item = obj
        backend.fetch()

    engine.rootContext().setContextProperty("backend", backend)
    engine.objectCreated.connect(on_object_created)
    engine.load(QUrl.fromLocalFile(str(QML_FILE)))

    # Global shortcut via toggle socket (no threads — QSocketNotifier)
    start_toggle_listener(backend, toggle_window)

    sys.exit(app.exec())


if __name__ == "__main__":
    main()
