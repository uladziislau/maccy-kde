PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share/maccy-kde
APPS_DIR ?= $(PREFIX)/share/applications

.PHONY: install uninstall autostart remove-autostart deps-fedora test clean

install:
	install -D -m 755 maccy-kde $(DESTDIR)$(BINDIR)/maccy-kde
	install -D -m 644 maccy_kde.qml $(DESTDIR)$(DATADIR)/maccy_kde.qml

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/maccy-kde
	rm -rf $(DESTDIR)$(DATADIR)

autostart: install
	maccy-kde --install-autostart

remove-autostart:
	maccy-kde --remove-autostart

deps-fedora:
	sudo dnf install -y python3-pyqt6 python3-pyqt6-qml python3-pyqt6-dbus wtype

test:
	python3 -m pytest test_all.py -v

clean:
	rm -rf target/ .pytest_cache/ __pycache__/ */__pycache__/
