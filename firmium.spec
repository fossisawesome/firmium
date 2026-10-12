Name:           firmium
Version:        9.0.0
Release:        1
Summary:        OpenSubsonic music streaming desktop client

License:        GPL-3.0-only
URL:            https://github.com/fossisawesome/firmium
Source0:        %{url}/archive/v%{version}.tar.gz

# Build deps for the Tauri 2 app (Rust backend + Svelte frontend).
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  cmake
BuildRequires:  alsa-lib-devel
BuildRequires:  gtk3-devel
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  openssl-devel
BuildRequires:  libsecret-devel
BuildRequires:  libxdo-devel
BuildRequires:  librsvg2-devel
BuildRequires:  dbus-devel
BuildRequires:  nodejs
BuildRequires:  npm

# cpal->alsa, tauri->webkit2gtk/gtk3, keyring->libsecret, tauri->libxdo.
Requires:       alsa-lib
Requires:       gtk3
Requires:       webkit2gtk4.1
Requires:       libsecret
Requires:       libxdo
Requires:       hicolor-icon-theme

%description
Firmium is a lightweight, low-latency OpenSubsonic music streaming client for the
desktop. Built with Tauri 2 (Rust backend, Svelte UI) for a fast, lightweight app,
OS keyring credential storage, and OpenSubsonic server integration (e.g. Navidrome).

%prep
%setup -q -n %{name}-%{version}

%build
npm ci
npm run build
cargo build --release --locked -p firmium --features custom-protocol

%install
install -Dm755 target/release/firmium %{buildroot}%{_bindir}/firmium
install -Dm644 packaging/firmium.desktop %{buildroot}%{_datadir}/applications/firmium.desktop
install -Dm644 src-tauri/icons/128x128.png %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/firmium.png
install -Dm644 src-tauri/icons/32x32.png %{buildroot}%{_datadir}/icons/hicolor/32x32/apps/firmium.png

%files
%license LICENSE
%{_bindir}/firmium
%{_datadir}/applications/firmium.desktop
%{_datadir}/icons/hicolor/*/apps/firmium.png
