<h1 align="center">Firmium</h1>
<p align="center"><i>Smooth, fast, simple. Forever.</i></p>

<p align="center">
  <a href="https://github.com/fossisawesome/firmium/releases/latest"><img src="https://img.shields.io/github/v/release/fossisawesome/firmium?label=version&color=blue" alt="Latest release"></a>
  <a href="https://aur.archlinux.org/packages/firmium-desktop-bin"><img src="https://img.shields.io/aur/version/firmium-desktop-bin?label=AUR" alt="AUR version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green" alt="License: GPL-3.0"></a>
</p>

---

<p align="center">
  <img src="readme/homepage.png?raw=true" alt="Firmium homepage" width="800">
</p>

---

Firmium is a cross-platform [OpenSubsonic](https://opensubsonic.netlify.app/) music streaming client built with Tauri 2, targeting **Linux desktop and Android**. It connects to any OpenSubsonic-compatible server — such as [Navidrome](https://www.navidrome.org/) — and provides lightweight, low-latency audio playback using the native OS audio engine.

> **Note:** Firmium is a *client only* — you need a self-hosted OpenSubsonic-compatible server to use it. [Navidrome](https://www.navidrome.org/) is the most popular choice and is free and open source.

## Features

**What makes Firmium stand out:**
- Native OS audio engine via Rodio (Linux) / ExoPlayer (Android) — no Electron, no Chromium audio stack
- Crossfade between tracks with configurable overlap
- Credentials stored securely in the OS keyring (GNOME Keyring / KWallet on Linux) or Android Keystore-backed EncryptedSharedPreferences — never written to disk in plaintext
- Android MediaSession integration: lock screen controls and persistent playback notification

**Everything else:**
- Synced and unsynced lyrics
- Wikipedia artist biographies
- Pretty UI with 8 color themes
- Cover art caching
- Per-device volume control
- Full OpenSubsonic API support (scrobbling, search, playlists, and more)

## Gallery

<table>
  <tr>
    <td><img src="readme/homepage.png?raw=true" alt="Homepage" width="400"><br><sub>Home</sub></td>
    <td><img src="readme/artist.png?raw=true" alt="Artist page" width="400"><br><sub>Artist page</sub></td>
  </tr>
  <tr>
    <td><img src="readme/search.png?raw=true" alt="Search" width="400"><br><sub>Search</sub></td>
    <td><img src="readme/settings.png?raw=true" alt="Settings" width="400"><br><sub>Settings & themes</sub></td>
  </tr>
</table>

## Getting Started

1. Install Firmium for your distribution (see [Install](#install) below)
2. Open the app and enter your OpenSubsonic server URL (e.g. `http://your-navidrome-server:4533`)
3. Enter your username and password — credentials are saved securely to your OS keyring

That's it. Your library will load automatically.

## Install

Firmium is available for **Linux desktop** and **Android**.

> **Compatibility (Linux):** Tested on Hyprland (Wayland). Other desktop environments should work but are not officially tested. X11 is untested.

### Android

[![Get it on Obtainium](https://raw.githubusercontent.com/ImranR98/Obtainium/main/assets/graphics/badge_obtainium.png)](https://apps.obtainium.imranr.dev/redirect?r=obtainium://app/https%3A%2F%2Fgithub.com%2Ffossisawesome%2Ffirmium)

Or download the latest `.apk` from the [releases page](https://github.com/fossisawesome/firmium/releases/latest) and install it manually:

```bash
# Via ADB (sideloading):
adb install firmium_*.apk
```

Or transfer the APK to your device and open it with a file manager. You may need to enable **Install from unknown sources** in your device settings.

> **Note:** Firmium for Android requires Android 8.0 (API 26) or later.

### System Dependencies

Before running Firmium, install the required system libraries for your distribution.

**Debian / Ubuntu**
```bash
sudo apt update && sudo apt install -y libwebkit2gtk-4.1-0 libasound2 libssl3 libsecret-1-0 libxdo3 libxcb1
```

**Fedora**
```bash
sudo dnf install -y webkit2gtk4.1 alsa-lib openssl-libs libsecret libxdo libxcb
```

**Arch Linux**
```bash
sudo pacman -S --needed webkit2gtk-4.1 alsa-lib openssl libsecret xdotool libxcb
```

Firmium also requires:
- A **Secret Service provider** (GNOME Keyring or KWallet) for credential storage — included in most desktop environments. Without it, passwords won't be saved and you'll need to log in every launch.
- **PipeWire or PulseAudio** — on modern distros ALSA routes through one of these. Run `aplay -l` to verify audio devices are visible.

### Installing the App

Download the latest release from the [releases page](https://github.com/fossisawesome/firmium/releases/latest). (Unless you use Arch)

**Arch Linux**
```bash
yay -S firmium-desktop-bin # or paru -S firmium-desktop-bin
```

**Fedora (COPR)**
```bash
sudo dnf copr enable fossisawesome/Firmium
sudo dnf install firmium
```

**Debian / Ubuntu**
```bash
# Download the .deb from the releases page, then:
sudo dpkg -i ./firmium_*.deb
```

## Building from Source

### Prerequisites

- Rust 1.70 or later (`rustup default stable`)
- Node.js 18 or later
- System dependencies for your distribution (see [System Dependencies](#system-dependencies) above)

### Steps

```bash
# Clone the repository
git clone https://github.com/fossisawesome/firmium.git
cd firmium

# Install Node dependencies
npm install

# Start the development build
npm run dev:app
```

For a release build:

```bash
npm run release
```

This produces `.deb` and `.rpm` packages under `src-tauri/target/release/bundle/`.

### Building for Android

#### Additional Prerequisites

- Android SDK and NDK (install via Android Studio or `sdkmanager`)
- `ANDROID_HOME` and `NDK_HOME` environment variables set
- Rust Android targets:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```

#### Steps

```bash
# Initialize Android project (first time only)
npx tauri android init

# Development build (requires connected device or emulator)
npx tauri android dev

# Release APK
npx tauri android build
```

The release APK is output to `src-tauri/gen/android/app/build/outputs/apk/`.

## Troubleshooting (Android)

**App installed but won't open**
Ensure your device runs Android 8.0 (API 26) or later. If you sideloaded the APK, confirm you have allowed installs from unknown sources.

**Playback notification doesn't appear**
Grant Firmium the **Notifications** permission in your device's app settings. On Android 13+, this permission must be granted explicitly.

**Credentials lost after reinstall**
Credential storage is tied to the app's Android Keystore entry. A full uninstall clears the keys — you'll need to log in again after reinstalling.

**No audio through Bluetooth / certain output devices**
ExoPlayer routes audio through the Android audio system. If a specific output device isn't working, check that it is selected as the active output in your system's audio settings.

## Troubleshooting (Linux)

**App launches but credentials aren't saved / login fails every restart**
Your system's Secret Service daemon isn't running or isn't unlocked. On GNOME, ensure GNOME Keyring is started. On KDE, ensure KWallet is enabled and unlocked. You can test with:
```bash
secret-tool store --label='test' key value
```
If that fails, your keyring isn't running.

**No audio output**
Check that ALSA or PipeWire/PulseWire is set up correctly. Run `aplay -l` to list audio devices.

**Blank window or app won't start (Wayland)**
Try forcing XWayland: `WAYLAND_DISPLAY= ./firmium` or set `GDK_BACKEND=x11` before launching.

**Server connection refused**
Make sure your server URL includes the port (e.g. `http://192.168.1.10:4533`) and that Firmium can reach it on your network. Check your server's logs if the URL looks correct.

## Contributing

Bug reports, feature requests, and pull requests are welcome - open an issue or PR on [GitHub](https://github.com/fossisawesome/firmium/issues).

## License

[GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html) - see [LICENSE](LICENSE) for the full text.
