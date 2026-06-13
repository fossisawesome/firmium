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
URL:            https://github.com/fossisawesome/firmium
BuildArch:      x86_64
ExclusiveArch:  x86_64

# Downloads the pre-built Tauri RPM from GitHub Releases instead of recompiling.
Source0:        https://github.com/fossisawesome/firmium/releases/download/vVERSION_PLACEHOLDER/RPM_FILENAME_PLACEHOLDER

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
# %prep always runs in %{_builddir}; extract there so we have a stable
# absolute path to reference in %install.
rpm2cpio %{SOURCE0} | cpio -idmv --no-absolute-filenames

%build
# Pre-compiled binary — nothing to build.

%install
# Reference %{_builddir} explicitly — don't rely on cwd being the same
# as in %prep.
cp -a %{_builddir}/usr %{buildroot}/

%files
%{_bindir}/firmium
%{_datadir}/applications/Firmium.desktop
%{_datadir}/icons/hicolor/32x32/apps/firmium.png
%{_datadir}/icons/hicolor/128x128/apps/firmium.png
%{_datadir}/icons/hicolor/256x256@2/apps/firmium.png
/usr/lib/Firmium/

%changelog
* CHANGELOG_DATE_PLACEHOLDER GitHub Actions <actions@github.com> - VERSION_PLACEHOLDER-1
- Automated release
