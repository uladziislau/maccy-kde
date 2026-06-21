Name:           maccy-kde
Version:        0.1.0
Release:        1%{?dist}
Summary:        Keyboard-first clipboard manager for KDE Plasma 6

License:        MIT
URL:            https://github.com/your-username/maccy-kde
Source0:        %{name}-%{version}.tar.gz

BuildArch:      noarch
Requires:       python3-pyqt6
Requires:       python3-pyqt6-qml
Requires:       python3-pyqt6-dbus
Requires:       wtype
Requires:       wl-clipboard

%description
A keyboard-first clipboard manager for KDE Plasma 6, inspired by Maccy.
Integrates with Klipper via D-Bus, provides fuzzy search, categories,
pinned items, and quick paste.

%prep
%setup -q

%build
# no build step — pure Python

%install
install -D -m 755 maccy-kde %{buildroot}%{_bindir}/%{name}
install -D -m 644 maccy_kde.qml %{buildroot}%{_datadir}/maccy-kde/maccy_kde.qml
install -D -m 644 maccy-kde.desktop %{buildroot}%{_datadir}/applications/%{name}.desktop

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_datadir}/maccy-kde/maccy_kde.qml
%{_datadir}/applications/%{name}.desktop

%post
%{_bindir}/%{name} --install-autostart 2>/dev/null || :

%preun
%{_bindir}/%{name} --remove-autostart 2>/dev/null || :

%changelog
* Sun Jun 21 2026 Uladzislau Darazhei <uladzislau.darazhei@gmail.com> - 0.1.0-1
- Initial Python/QML release
- Klipper D-Bus integration, fuzzy search, pins, categories, paste
