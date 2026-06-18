%global commit %(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
%global shortcommit %(git rev-parse --short=7 HEAD 2>/dev/null || echo "unknown")

Name:           firmium
Version:        6.4.1
Release:        1
Summary:        OpenSubsonic music streaming desktop client

License:        GPL-3.0
URL:            https://github.com/fossisawesome/firmium
Source0:        %{url}/archive/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rustc
BuildRequires:  nodejs
BuildRequires:  npm
BuildRequires:  openssl-devel
BuildRequires:  libxdo-devel
BuildRequires:  libxcb-devel
BuildRequires:  libxcb-render-devel
BuildRequires:  libxcb-shape-devel
BuildRequires:  libxcb-xfixes-devel
BuildRequires:  libsecret-devel
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  make
BuildRequires:  git

%description
Firmium is a lightweight, low-latency OpenSubsonic music streaming client for the desktop.
Built with Tauri, Svelte, and Rust for fast, native performance across Linux, macOS, and Windows.

%prep
%setup -q -n %{name}-%{version}

%build
# Install dependencies
npm install

# Build release
npm run release

%install
# Extract and install the RPM that Tauri built
cd src-tauri/target/release/bundle/rpm
rpm2cpio firmium-*.rpm | cpio -idmv
cp -r usr/* %{buildroot}/usr/ 2>/dev/null || true

%files
%license LICENSE
/usr/bin/firmium
/usr/share/applications/firmium.desktop
/usr/share/icons/hicolor/*/apps/firmium.png
/usr/share/icons/hicolor/*/apps/firmium.svg

%changelog
* Thu Jun 18 2026 fossisawesome <fossisawesome AT github DOT com> - 6.4.1-1
- Initial packaging for COPR
