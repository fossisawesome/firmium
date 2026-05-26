<h1 align="center">Firmium</h1>
<p align="center"><i>Smooth, fast, simple. Forever.</i></p>

<p align="center">
  <a href="https://github.com/fossisawesome/firmium/releases/latest"><img src="https://img.shields.io/github/v/release/fossisawesome/firmium?label=version&color=blue" alt="Latest release"></a>
  <a href="https://aur.archlinux.org/packages/firmium-desktop-bin"><img src="https://img.shields.io/aur/version/firmium-desktop-bin?label=AUR" alt="AUR version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green" alt="License: GPL-3.0"></a>
</p>

---

Firmium is a desktop [OpenSubsonic](https://opensubsonic.netlify.app/) music streaming client built with Tauri 2. It connects to any OpenSubsonic-compatible server — such as [Navidrome](https://www.navidrome.org/) — and provides lightweight, low-latency audio playback using the native OS audio engine.

## Features

- Synced and unsynced lyrics
- Wikipedia artist biographies
- Lightweight native audio backend via Rodio
- Pretty UI with 8 color themes
- Cover art caching
- Crossfade between tracks
- Per-device volume control
- Credentials stored securely in the OS keyring (GNOME Keyring / KWallet)
- Full OpenSubsonic API support (scrobbling, search, playlists, and more)

## Gallery

<img src="readme/homepage.png?raw=true" alt="Homepage of Firmium">
<img src="readme/artist.png?raw=true" alt="Artist page of Firmium">
<img src="readme/search.png?raw=true" alt="Searchpage of Firmium">
<img src="readme/settings.png?raw=true" alt="Settings page of Firmium">

## Getting Started

1. Install Firmium for your distribution (see [Install](#install) below)
2. Open the app and enter your OpenSubsonic server URL (e.g. `http://your-navidrome-server:4533`)
3. Enter your username and password — credentials are saved securely to your OS keyring

That's it. Your library will load automatically.

## Install

### System Dependencies

Before running Firmium, install the required system libraries for your distribution.

**Debian / Ubuntu**
```bash
sudo apt update && sudo apt install -y libwebkit2gtk-4.1-0 libasound2 libssl3
```

**Fedora**
```bash
sudo dnf install -y webkit2gtk4.1 alsa-lib openssl-libs
```

**Arch Linux**
```bash
sudo pacman -S --needed webkit2gtk-4.1 alsa-lib openssl
```

Firmium also requires a Secret Service provider (GNOME Keyring or KWallet) for credential storage. This is included in most desktop environments by default.

### Installing the App

Download the latest release from the [releases page](https://github.com/fossisawesome/firmium/releases/latest).

**Arch Linux**
```bash
yay -S firmium-desktop-bin # or paru -S firmium-desktop-bin
```

**Fedora**
```bash
# Download the .rpm from the releases page, then:
sudo dnf install ./firmium_*.rpm
```

**Debian / Ubuntu**
```bash
# Download the .deb from the releases page, then:
sudo dpkg -i ./firmium_*.deb
```

### Compatibility

Firmium is tested on Hyprland (Wayland). Other desktop environments should work but are not officially tested.

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

## License

[GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html) — see [LICENSE](LICENSE) for the full text.
