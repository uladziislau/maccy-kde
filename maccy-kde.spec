Name:           maccy-kde
Version:        0.1.0
Release:        1%{?dist}
Summary:        A lightweight, keyboard-first clipboard manager for KDE Plasma 6 on Wayland

License:        MIT
URL:            https://github.com/your-username/maccy-kde
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust cargo
BuildRequires:  cmake
BuildRequires:  gtk3-devel
BuildRequires:  dbus-devel
BuildRequires:  sqlite-devel

%description
A lightweight, keyboard-first clipboard manager for KDE Plasma 6 on Wayland, built in Rust.

%prep
%setup -q

%build
cargo build --release

%install
install -D -m 755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

# Desktop file for autostart is handled by the app itself via --install-autostart

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}

%changelog
* Tue Jun 09 2026 Uladzislau Darazhei <uladzislau.darazhei@gmail.com> - 0.1.0-1
- Initial package
