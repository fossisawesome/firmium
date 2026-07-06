Name:           firmium
Version:        8.0.0
Release:        1
Summary:        OpenSubsonic music streaming desktop client

License:        GPL-3.0-only
URL:            https://github.com/fossisawesome/firmium
Source0:        %{url}/archive/v%{version}.tar.gz

# Build deps for the native iced/Rust app.
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  alsa-lib-devel
BuildRequires:  gtk3-devel
BuildRequires:  libsecret-devel

# cpal->alsa, rfd->gtk3, keyring->libsecret, wgpu->vulkan loader, text->fontconfig.
Requires:       alsa-lib
Requires:       gtk3
Requires:       libsecret
Requires:       vulkan-loader
Requires:       fontconfig
Requires:       hicolor-icon-theme

%description
Firmium is a lightweight, low-latency OpenSubsonic music streaming client for the
desktop. Built as a native iced (Rust) application for fast, GPU-accelerated UI,
OS keyring credential storage, and OpenSubsonic server integration (e.g. Navidrome).

%prep
%setup -q -n %{name}-%{version}

%build
cargo build --release --locked

%install
install -Dm755 target/release/firmium %{buildroot}%{_bindir}/firmium
install -Dm644 packaging/firmium.desktop %{buildroot}%{_datadir}/applications/firmium.desktop
install -Dm644 assets/app-icons/128x128.png %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/firmium.png
install -Dm644 assets/app-icons/32x32.png %{buildroot}%{_datadir}/icons/hicolor/32x32/apps/firmium.png

%files
%license LICENSE
%{_bindir}/firmium
%{_datadir}/applications/firmium.desktop
%{_datadir}/icons/hicolor/*/apps/firmium.png
