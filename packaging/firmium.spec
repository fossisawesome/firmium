%define debug_package %{nil}

Name:           firmium
Version:        VERSION_PLACEHOLDER
Release:        1%{?dist}
Summary:        OpenSubsonic music streaming desktop client
License:        GPL-3.0-only
URL:            https://github.com/fossisawesome/firmium
BuildArch:      x86_64

# Builds from source via cargo (the app is now a native iced/Rust binary; the
# old prebuilt-Tauri-RPM repackage flow is retired).
Source0:        %{url}/archive/vVERSION_PLACEHOLDER.tar.gz

# Copr/mock builds run without network access, so crates.io can't be reached
# during %%build. CI (.github/workflows/copr.yml) runs `cargo vendor` and
# packages the result here; %%build then compiles fully offline against it.
Source1:        firmium-VERSION_PLACEHOLDER-vendor.tar.xz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  cmake
BuildRequires:  alsa-lib-devel
BuildRequires:  gtk3-devel
BuildRequires:  libsecret-devel

Requires:       alsa-lib
Requires:       gtk3
Requires:       libsecret
Requires:       vulkan-loader
Requires:       fontconfig
Requires:       hicolor-icon-theme

%description
Firmium is a low-latency OpenSubsonic music streaming client for the desktop,
built as a native iced (Rust) application. Supports GPU-accelerated UI, OS
keyring credential storage, and OpenSubsonic server integration (e.g. Navidrome).

%prep
%setup -q -n firmium-VERSION_PLACEHOLDER
tar xf %{SOURCE1}
mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
cargo build --release --locked --offline

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

%changelog
* CHANGELOG_DATE_PLACEHOLDER GitHub Actions <actions@github.com> - VERSION_PLACEHOLDER-1
- Automated release
