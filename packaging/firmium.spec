%define debug_package %{nil}

Name:           firmium
Version:        VERSION_PLACEHOLDER
Release:        1%{?dist}
Summary:        OpenSubsonic music streaming desktop client
License:        GPL-3.0-only
URL:            https://github.com/fossisawesome/firmium
BuildArch:      x86_64

# Builds from source via cargo (Tauri 2 app). The Svelte frontend is prebuilt by CI
# and shipped as dist/ inside the vendor tarball.
Source0:        %{url}/archive/vVERSION_PLACEHOLDER.tar.gz

# Copr/mock builds run without network access, so crates.io can't be reached
# during %%build. CI (.github/workflows/copr.yml) runs `cargo vendor` + `npm run build`
# and packages vendor/ and dist/ here; %%build then compiles fully offline.
Source1:        firmium-VERSION_PLACEHOLDER-vendor.tar.xz

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

Requires:       alsa-lib
Requires:       gtk3
Requires:       webkit2gtk4.1
Requires:       libsecret
Requires:       libxdo
Requires:       hicolor-icon-theme

%description
Firmium is a low-latency OpenSubsonic music streaming client for the desktop,
built with Tauri 2 (Rust backend, Svelte UI). Supports OS
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
cargo build --release --locked --offline -p firmium --features custom-protocol

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

%changelog
* CHANGELOG_DATE_PLACEHOLDER GitHub Actions <actions@github.com> - VERSION_PLACEHOLDER-1
- Automated release
