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
# Create the expected build subdirectory and extract the Tauri RPM into it.
# rpmbuild runs %%install in %%{_builddir}/%%{name}-%%{version}, so we must
# extract there rather than in %%{_builddir} directly.
mkdir -p %{name}-%{version}
cd %{name}-%{version}
rpm2cpio %{SOURCE0} | cpio -idmv --no-absolute-filenames

%build
# Pre-compiled binary — nothing to build.

%install
# Copy extracted usr/ tree to buildroot.
cp -a usr %{buildroot}/

%files
%{_bindir}/firmium-desktop
%{_datadir}/applications/Firmium.desktop
%{_datadir}/icons/hicolor/32x32/apps/firmium-desktop.png
%{_datadir}/icons/hicolor/128x128/apps/firmium-desktop.png
%{_datadir}/icons/hicolor/256x256@2/apps/firmium-desktop.png
/usr/lib/Firmium/

%changelog
* CHANGELOG_DATE_PLACEHOLDER GitHub Actions <actions@github.com> - VERSION_PLACEHOLDER-1
- Automated release
