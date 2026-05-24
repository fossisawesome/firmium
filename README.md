<h1 align="center">Firmium</h1>
<p align="center"><i>Smooth, fast, simple. Forever.</i></p>

---

Firmium is a desktop Open-Subsonic music streaming client built with Tauri 2. It connects to any Open-Subsonic server and provides lightweight audio playback using native OS audio.

## Features

- Lyrics 
- Wikipedia artist biographys
- Lightweight audio backend with Rodio
- Pretty UI with 8 color themes
- Open-Subsonic
## Gallery

*Screenshots coming soon.*

## Install

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

### Installing the app

**Arch Linux**
```bash
yay -S firmium-desktop-bin # or paru -S firmium-desktop-bin
```

**Fedora**
```bash
# Download and install the .rpm - in the future this will be added to DNF
```

**Debian / Ubuntu**
```bash
# Download and install the .deb - in the future this will be added to APT
```

## Building from Source

### Prerequisites

- Rust 1.70 or later (`rustup default stable`)
- Node.js 18 or later
- System dependencies for your distribution (see [Installing Dependencies](#installing-dependencies))

### Steps

```bash
# Clone the repository
git clone https://github.com/fossisawesome/firmium.git
cd firmium

# Install Node dependencies
npm install

# Start the development build
npm run tauri dev
```

For a release build:

```bash
npm run build:arch
```

This produces `.deb` and `.rpm` packages under `src-tauri/target/release/bundle/`, and runs `makepkg` for the Arch Linux package.

## License

GPL-3.0
