<p align="center">
  <img src="readme/favicon.svg" alt="Firmium logo" width="96">
</p>

<h1 align="center">Firmium</h1>
<p align="center"><i>Smooth, fast, simple. Forever.</i></p>

<p align="center">
  <a href="https://github.com/fossisawesome/firmium/releases/latest"><img src="https://img.shields.io/github/v/release/fossisawesome/firmium?label=version&color=blue" alt="Latest release"></a>
  <a href="https://aur.archlinux.org/packages/firmium-desktop-bin"><img src="https://img.shields.io/aur/version/firmium-desktop-bin?label=AUR" alt="AUR version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green" alt="License: GPL-3.0"></a>
  <a href="https://github.com/sponsors/fossisawesome"><img src="https://img.shields.io/badge/sponsor-%E2%9D%A4-db61a2?logo=githubsponsors" alt="Sponsor on GitHub"></a>
  <a href="https://discord.gg/bfZ3rpYJXk"><img src="https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
</p>

---

<p align="center">
  <img src="readme/homepage.png?raw=true" alt="Firmium homepage" width="800">
</p>

---

Firmium is a cross-platform [OpenSubsonic](https://opensubsonic.netlify.app/) music streaming client built with Tauri 2, targeting **Linux desktop, Windows and Android**. It connects to any OpenSubsonic-compatible server — such as [Navidrome](https://www.navidrome.org/) — and provides lightweight, low-latency audio playback using the native OS audio engine.

> **Note:** Firmium is a *client only* — you need a self-hosted OpenSubsonic-compatible server to use it. [Navidrome](https://www.navidrome.org/) is the most popular choice and is free and open source.

## Features

| Feature | Desktop | Android |
| --- | :---: | :---: |
| Native audio engine (no Electron/Chromium audio stack) | Rodio | ExoPlayer |
| Crossfade between tracks with configurable overlap | ✅ | ✅ |
| Credentials stored in OS keyring / Keystore — never plaintext on disk | ✅ | ✅ |
| Lock screen controls & persistent playback notification | — | ✅ |
| Synced and unsynced lyrics | ✅ | ✅ |
| 18 built-in color themes + custom user themes | ✅ | ✅ |
| Cover art caching | ✅ | ✅ |
| Per-device volume control | ✅ | ✅ |
| Full OpenSubsonic API support (scrobbling, search, playlists, more) | ✅ | ✅ |

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

<a href="https://apps.obtainium.imranr.dev/redirect?r=obtainium://app/%7B%22id%22%3A%22com.fossisawesome.firmium%22%2C%22url%22%3A%22https%3A%2F%2Fgithub.com%2Ffossisawesome%2Ffirmium%22%2C%22author%22%3A%22fossisawesome%22%2C%22name%22%3A%22Firmium%22%2C%22preferredApkIndex%22%3A0%2C%22additionalSettings%22%3A%22%7B%5C%22includePrereleases%5C%22%3Afalse%2C%5C%22fallbackToOlderReleases%5C%22%3Atrue%2C%5C%22autoApkFilterByArch%5C%22%3Atrue%2C%5C%22versionDetection%5C%22%3Atrue%2C%5C%22sortMethodChoice%5C%22%3A%5C%22date%5C%22%7D%22%2C%22overrideSource%22%3Anull%7D"><img src="https://raw.githubusercontent.com/ImranR98/Obtainium/main/assets/graphics/badge_obtainium.png" alt="Get it on Obtainium" height="54"></a>

Or download the latest `.apk` from the [releases page](https://github.com/fossisawesome/firmium/releases/latest) and install it manually:

```bash
# Via ADB (sideloading):
adb install firmium_*.apk
```

Or transfer the APK to your device and open it with a file manager. You may need to enable **Install from unknown sources** in your device settings.

> **Note:** Firmium for Android requires Android 8.0 (API 26) or later.

#### Using Android Auto

Firmium supports Android Auto, so you can browse your library and play music from your car's
display. Installing the APK alone is not enough, though: because Firmium is sideloaded rather than
installed from the Google Play Store, Android Auto hides it by default. To use it in your car,
allow it once:

1. Install Firmium on your phone (above).
2. Open the **Android Auto** settings on your phone, scroll to the bottom and tap **Version** about
   ten times to unlock **Developer settings**, then turn on **Unknown sources**.
3. Connect your phone to the car and choose **Firmium** from the car's app launcher.

The "Unknown sources" step is required only because Firmium is not distributed through the Play
Store; it would not be needed for a Play Store release approved for Android Auto. See the
[Android Auto guide](https://docs.firmium.app/android-auto) for browsing, voice control, and more.

### System Dependencies

Before running Firmium, install the required system libraries for your distribution.

<details>
<summary><b>Debian / Ubuntu</b></summary>

```bash
sudo apt update && sudo apt install -y libwebkit2gtk-4.1-0 libasound2 libssl3 libsecret-1-0 libxdo3 libxcb1
```
</details>

<details>
<summary><b>Fedora</b></summary>

```bash
sudo dnf install -y webkit2gtk4.1 alsa-lib openssl-libs libsecret libxdo libxcb
```
</details>

<details>
<summary><b>Arch Linux</b></summary>

```bash
sudo pacman -S --needed webkit2gtk-4.1 alsa-lib openssl libsecret xdotool libxcb
```
</details>

Firmium also requires:
- A **Secret Service provider** (GNOME Keyring or KWallet) for credential storage — included in most desktop environments. Without it, passwords won't be saved and you'll need to log in every launch.
- **PipeWire or PulseAudio** — on modern distros ALSA routes through one of these. Run `aplay -l` to verify audio devices are visible.

### Installing the App

<table>
  <tr>
    <td valign="top"><b>Arch Linux</b><br>

```bash
yay -S firmium-desktop-bin # or paru -S firmium-desktop-bin
```
</td>
    <td valign="top"><b>Fedora (COPR)</b><br>

```bash
sudo dnf copr enable fossisawesome/Firmium
sudo dnf install firmium
```
</td>
  </tr>
  <tr>
    <td valign="top" colspan="2"><b>Debian / Ubuntu / other</b><br>

Download the `.deb` from the [releases page](https://github.com/fossisawesome/firmium/releases/latest), then:

```bash
sudo dpkg -i ./firmium_*.deb
```
</td>
  </tr>
</table>

<details>
<summary><h2 style="display:inline">Building from Source</h2></summary>

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

The Android app is a native Kotlin + Jetpack Compose app in the `android/` directory, built with Gradle independently of the desktop Tauri project.

#### Additional Prerequisites

- JDK 17 or later
- Android SDK with build tools 35 (install via Android Studio or `sdkmanager`)
- `ANDROID_HOME` environment variable set

#### Steps

```bash
cd android

# Development build
./gradlew assembleDebug

# Install debug build on connected device / emulator
./gradlew installDebug

# Release APK (requires signing env vars — see below)
./gradlew assembleRelease
```

The release APK is output to `android/app/build/outputs/apk/release/`.

To sign the release build, set these environment variables before running `assembleRelease`:

```bash
export ANDROID_SIGNING_KEY_PATH=/path/to/your.keystore
export ANDROID_SIGNING_KEY_ALIAS=your-key-alias
export ANDROID_SIGNING_STORE_PASSWORD=store-password
export ANDROID_SIGNING_KEY_PASSWORD=key-password
```

If these are not set, Gradle will build an unsigned APK.

</details>

<details>
<summary><h2 style="display:inline">Troubleshooting (Android)</h2></summary>

**App installed but won't open**
Ensure your device runs Android 8.0 (API 26) or later. If you sideloaded the APK, confirm you have allowed installs from unknown sources.

**Playback notification doesn't appear**
Grant Firmium the **Notifications** permission in your device's app settings. On Android 13+, this permission must be granted explicitly.

**Credentials lost after reinstall**
Credential storage is tied to the app's Android Keystore entry. A full uninstall clears the keys — you'll need to log in again after reinstalling.

**No audio through Bluetooth / certain output devices**
ExoPlayer routes audio through the Android audio system. If a specific output device isn't working, check that it is selected as the active output in your system's audio settings.

</details>

<details>
<summary><h2 style="display:inline">Troubleshooting (Linux)</h2></summary>

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

**Bit-perfect Audio doesn't seem to change the output rate**
On PipeWire systems, the app stream can open at the track's native sample rate while the ALSA sink itself stays locked at a fixed rate (commonly 48000Hz), so PipeWire resamples before the signal reaches the DAC. Check with `pw-top` during playback — if the `alsa_output` sink row doesn't match your track's sample rate, add a `default.clock.allowed-rates` list (e.g. `[ 44100 48000 88200 96000 176400 192000 ]`) to `~/.config/pipewire/pipewire.conf.d/`, then restart PipeWire (`systemctl --user restart pipewire pipewire-pulse wireplumber`).

</details>

## Contributing

Bug reports, feature requests, and pull requests are welcome - open an issue or PR on [GitHub](https://github.com/fossisawesome/firmium/issues).

## License

[GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html) - see [LICENSE](LICENSE) for the full text.
