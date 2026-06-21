"""Tests for maccy-kde (run: pytest test_all.py -v)"""

import os
import sys
import json
import tempfile
import time
from pathlib import Path

# Must be before any PyQt6 import
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ.pop("WAYLAND_DISPLAY", None)

# Now safe to import PyQt6
from PyQt6.QtCore import QObject, QUrl
from PyQt6.QtQml import QQmlApplicationEngine
from PyQt6.QtWidgets import QApplication

# Ensure we're in project root
PROJECT = Path(__file__).resolve().parent
sys.path.insert(0, str(PROJECT))

from maccy_kde import (
    Clip,
    ClipStore,
    detect_category,
    Backend,
    CATEGORIES,
    CATEGORY_RE,
    DATA_DIR,
    paste_text,
)

# Keep QApplication alive across all tests
_app = QApplication.instance() or QApplication(sys.argv[:1])
# Keep backends alive to prevent GC of wrapped C++ objects
_backends = []


# --- Category Detection ---

def test_detect_url():
    assert detect_category("https://example.com") == "URLs"
    assert detect_category("http://foo.bar/baz?q=1") == "URLs"
    assert detect_category("HTTP://SHOUT.ING") == "URLs"


def test_detect_email():
    assert detect_category("user@example.com") == "Emails"
    assert detect_category("first.last@company.co.uk") == "Emails"


def test_detect_path():
    assert detect_category("/home/user/file.txt") == "Paths"
    assert detect_category("/usr/local/bin/python3") == "Paths"


def test_detect_code():
    assert detect_category("aGVsbG8gd29ybGQ=") == "Codes"


def test_detect_number():
    assert detect_category("42") == "Numbers"
    assert detect_category("3.14159") == "Numbers"


def test_detect_other():
    assert detect_category("Hello World") == "Other"
    assert detect_category("some random text") == "Other"


# --- Clip model ---

def test_clip_category_auto():
    c = Clip("https://x.com")
    assert c.category == "URLs"


def test_clip_matches_fuzzy():
    c = Clip("hello world")
    assert c.matches("hello")
    assert c.matches("hlo")
    assert c.matches("hld")
    assert not c.matches("xyz")


def test_clip_matches_empty():
    c = Clip("anything")
    assert c.matches("")


def test_clip_to_dict():
    c = Clip("text", "URLs", True, "Q")
    d = c.to_dict()
    assert d["text"] == "text"
    assert d["category"] == "URLs"
    assert d["pinned"] == True
    assert d["shortcut"] == "Q"
    assert d["section"] == "Pinned"


def test_clip_to_dict_unpinned():
    c = Clip("some text")
    assert c.to_dict()["section"] == "History"


# --- Pins Persistence ---

def test_clipstore_init():
    store = ClipStore(DATA_DIR)
    assert isinstance(store.pins, set)
    assert isinstance(store.hidden, set)


def test_clipstore_toggle_pin():
    store = ClipStore(DATA_DIR)
    store.toggle_pin("item1")
    assert "item1" in store.pins
    store.toggle_pin("item1")
    assert "item1" not in store.pins


def test_clipstore_persists(tmp_path):
    store = ClipStore(tmp_path)
    store.toggle_pin("persistent")
    # Create new store pointing at same dir — data should persist
    store2 = ClipStore(tmp_path)
    assert "persistent" in store2.pins


# --- Backend Item Filtering ---

def _make_backend():
    b = Backend()
    _backends.append(b)  # prevent GC
    b._raw_items = [
        "https://github.com",
        "user@example.com",
        "/home/user/docs/report.pdf",
        "Hello World",
    ]
    b._rebuild()
    return b


def test_backend_items_all():
    b = _make_backend()
    items = b.itemsModel
    assert len(items) == 4


def test_backend_filter_by_category():
    b = _make_backend()
    items = b.itemsModel
    urls = [it for it in items if it["category"] == "URLs"]
    assert len(urls) == 1
    assert urls[0]["text"] == "https://github.com"


def test_backend_filter_query():
    b = _make_backend()
    b._query = "github"
    b._rebuild()
    items = b.itemsModel
    assert len(items) == 1
    assert "github" in items[0]["text"]


def test_backend_filter_query_no_match():
    b = _make_backend()
    b._query = "zzz_nonexistent"
    b._rebuild()
    assert len(b.itemsModel) == 0


def test_backend_pin_toggle():
    b = _make_backend()
    text = "https://github.com"
    assert text not in b._pinned

    b.togglePin(text)
    assert text in b._pinned

    b.togglePin(text)
    assert text not in b._pinned


def test_backend_pinned_items_first():
    b = _make_backend()
    b._pinned.add("Hello World")
    b._rebuild()
    items = b.itemsModel
    assert items[0]["text"] == "Hello World"
    assert items[0]["pinned"] == True


def test_backend_hide():
    b = _make_backend()
    text = "Hello World"
    b._hidden.add(text)
    b._rebuild()
    assert text not in [it["text"] for it in b.itemsModel]


def test_backend_clear_unpinned():
    b = _make_backend()
    b._pinned.add("Hello World")
    b.clearUnpinned()
    assert len(b._pinned) == 0


# --- QML Loading ---

def test_qml_loads():
    qml = PROJECT / "maccy_kde.qml"
    assert qml.exists()

    engine = QQmlApplicationEngine()
    b = Backend()
    engine.rootContext().setContextProperty("backend", b)

    top = None

    def on_load(obj, url):
        nonlocal top
        top = obj

    engine.objectCreated.connect(on_load)
    engine.load(QUrl.fromLocalFile(str(qml)))

    _app.processEvents()
    assert top is not None, "QML root object should not be None"
    # QML Window inherits QWindow (through QQuickWindow)
    assert "QWindow" in str(type(top))


# --- Paste ---

def test_paste_text_calls_subprocess(monkeypatch):
    calls = []
    def fake_run(cmd, **kw):
        calls.append(cmd)
        class Result:
            returncode = 0
        return Result()

    monkeypatch.setattr("subprocess.run", fake_run)
    monkeypatch.setattr("sys.exit", lambda x: None)

    text = "test clipboard"
    paste_text(text)
    assert len(calls) > 0, "paste should attempt at least one paste command"
    # First attempt should be wtype
    assert "wtype" in calls[0]


# --- Integration: Mock Klipper D-Bus ---

def _mock_klipper(backend, items):
    """Replace Klipper D-Bus interface with a mock."""
    from unittest.mock import MagicMock
    mock_iface = MagicMock()
    def mock_call(method, *args):
        if method == "getClipboardHistoryMenu":
            return _mock_msg(items)
        return _mock_msg([])
    mock_iface.call = mock_call
    backend._klipper = mock_iface

def _mock_msg(items):
    """Simulate QDBusMessage with a list argument."""
    from unittest.mock import MagicMock
    msg = MagicMock()
    def arguments():
        return [items]
    msg.arguments = arguments
    return msg


def test_backend_fetch_from_klipper():
    b = _make_backend()
    klipper_items = ["first item", "second item", "https://example.com"]
    _mock_klipper(b, klipper_items)
    b.fetch()
    items = b.itemsModel
    assert len(items) == 3
    assert items[0]["text"] == "first item"


def test_backend_fetch_dedup():
    b = _make_backend()
    klipper_items = ["item", "item", "different"]  # duplicates
    _mock_klipper(b, klipper_items)
    b.fetch()
    items = b.itemsModel
    assert len(items) == 2  # deduped


def test_backend_fetch_empty():
    b = _make_backend()
    _mock_klipper(b, [])
    b.fetch()
    assert len(b.itemsModel) == 0


def test_backend_paste_sets_clipboard(monkeypatch):
    """Verify paste calls setClipboardContents before wtype."""
    from unittest.mock import MagicMock
    dbus_calls = []

    class FakeDBusInterface:
        def __init__(self, *a, **kw):
            pass
        def call(self, method, *args):
            dbus_calls.append((method, args))

    monkeypatch.setattr("maccy_kde.QDBusInterface", FakeDBusInterface)
    subprocess_calls = []
    def fake_run(cmd, **kw):
        subprocess_calls.append(cmd)
        class Result:
            returncode = 0
        return Result()

    monkeypatch.setattr("subprocess.run", fake_run)

    paste_text("test text")
    assert len(dbus_calls) == 1
    assert dbus_calls[0][0] == "setClipboardContents"
    assert dbus_calls[0][1] == ("test text",)
    assert len(subprocess_calls) >= 1


# --- Integration: Full App Lifecycle ---

def test_app_start_and_toggle():
    """Start app, send toggle, verify window shows."""
    app = _app

    # Import and call main's internals
    from maccy_kde import start_toggle_listener, Backend
    from PyQt6.QtQml import QQmlApplicationEngine

    backend = Backend()
    _backends.append(backend)
    _mock_klipper(backend, ["clip 1", "clip 2"])

    engine = QQmlApplicationEngine()
    engine.rootContext().setContextProperty("backend", backend)

    top = None
    def on_load(obj, url):
        nonlocal top
        top = obj
    engine.objectCreated.connect(on_load)

    qml_file = PROJECT / "maccy_kde.qml"
    engine.load(QUrl.fromLocalFile(str(qml_file)))
    app.processEvents()
    assert top is not None

    toggled = []
    def fake_toggle():
        toggled.append(1)

    start_toggle_listener(backend, fake_toggle)

    # Send toggle via socket
    import socket
    import time
    sock_path = os.environ.get("XDG_RUNTIME_DIR", "/tmp") + "/maccy-kde.sock"
    # Wait for socket
    for _ in range(50):
        if os.path.exists(sock_path):
            break
        time.sleep(0.01)

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(2)
    s.connect(sock_path)
    s.sendall(b"toggle\n")
    s.close()
    app.processEvents()
    app.processEvents()

    assert len(toggled) == 1, "toggle handler should have been called once"

    # Clean up
    os.unlink(sock_path)
