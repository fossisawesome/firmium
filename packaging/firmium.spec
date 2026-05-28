%define debug_package %{nil}

# Disable auto-dependency detection — this is a pre-built binary repackage.
%global __requires_exclude_from ^.*$
%global __provides_exclude_from ^.*$
AutoReqProv:    no

Name:           firmium
Version:        VERSION_PLACEHOLDER
Release:        1%{?dist}
Summary:        Cross-platform OpenSubsonic music streaming client
License:        MIT
URL:            https://github.com/fossisawesome/firmium-desktop
BuildArch:      x86_64
ExclusiveArch:  x86_64

# Downloads the pre-built Tauri RPM from GitHub Releases instead of recompiling.
Source0:        https://github.com/fossisawesome/firmium-desktop/releases/download/vVERSION_PLACEHOLDER/RPM_FILENAME_PLACEHOLDER

BuildRequires:  cpio

Requires:       webkit2gtk4.1
Requires:       openssl-libs
Requires:       libsecret
Requires:       libxdo

%description
Firmium is a cross-platform OpenSubsonic music streaming client built with
Tauri 2. Supports low-latency audio playback, OS keyring credential storage,
and OpenSubsonic server integration (e.g. Navidrome).

%prep
# Extract the pre-built Tauri RPM; files land in ./usr/...
rpm2cpio %{SOURCE0} | cpio -idmv --no-absolute-filenames

%build
# Pre-compiled binary — nothing to build.

%install
# Copy extracted usr/ tree to buildroot
if [ -d usr ]; then
  cp -a usr %{buildroot}/
fi

%files
%{_bindir}/firmium
%{_datadir}/applications/firmium.desktop
%{_datadir}/icons/hicolor/*/apps/firmium.png
/usr/lib/firmium/

%changelog
* CHANGELOG_DATE_PLACEHOLDER GitHub Actions <actions@github.com> - VERSION_PLACEHOLDER-1
- Automated release
