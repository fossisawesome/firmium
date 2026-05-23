# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Firmium** is a desktop Subsonic music streaming client built with Tauri 2 (Rust backend + JavaScript frontend). It provides low-latency audio playback using the `rodio` audio library, OS-level credential storage via the system keyring, and integrates with Subsonic/Navidrome servers to stream music.

### Tech Stack
- **Frontend**: Vanilla JavaScript (no framework), HTML/CSS
- **Backend**: Rust 2021 edition, Tauri 2.11+
- **Audio**: `rodio` 0.22 for native OS audio engine integration
- **HTTP**: `reqwest` 0.13 for async Subsonic API calls
- **Credentials**: OS keyring (libsecret on Linux) via `keyring` crate
- **UI Framework**: Tauri window management + system-native audio device support
- **Packaging**: Linux (deb, rpm), with Arch Linux (makepkg) support

## Architecture

### Rust Backend (src-tauri/src/)

The backend exposes Tauri commands that the frontend invokes via the bridge in `audio-bridge.js`. Key modules:

- **main.rs**: Entry point. Defines Tauri commands for:
  - Subsonic auth: `generate_auth_params()` — MD5 token hashing on Rust side (keeps plaintext credentials out of JS)
  - Keyring ops: `save_password()`, `get_password()`, `delete_password()` — OS credential storage
  - Audio control: `play_stream()`, `pause_playback()`, `resume_playback()`, `stop_playback()`, `seek_position()`, `set_volume()`, `get_volume()`
  - Audio state: `get_playback_state()`, `is_playback_finished()`, `get_track_duration()`, `get_current_position()`
  - Diagnostics: `get_machine_info()` (CPU, GPU, distro), `list_audio_devices()`
  - File ops: `cache_cover()` — stores cover art to disk
  
- **audio.rs**: Audio playback module (~300 lines). Core design:
  - `StreamingReader`: Implements Read+Seek over HTTP response body. Bytes buffered locally to keep Subsonic "Now Playing" status during playback.
  - `AudioPlayer`: Manages session lifecycle (loading → playing → paused/stopped). Uses `rodio::MixerDeviceSink` for per-device volume control. Thread-safe via `parking_lot::Mutex`.
  - Session state: `PlaybackState` enum (Loading, Playing, Paused, Stopped)
  - Sessions stored in `Arc<RwLock<HashMap>>` — playback events fire via Tauri `emit()` to frontend
  - Multiple concurrent sessions supported (one per audio device)

- **lib.rs**: Boilerplate Tauri setup (plugins, handlers). Currently minimal; new commands must be registered here.

### JavaScript Frontend (src/)

Single-page app with no JS frameworks. Architecture:

- **index.html**: Basic page structure, loads CSS and JS in order
- **audio-bridge.js**: Tauri invocation helpers + event listeners. Bridges `tauriInvoke()` calls from app.js to Rust commands.
- **app.js** (~1500 lines): Main application state and UI logic:
  - **Api**: Subsonic API client (fetch with auth params from Rust)
  - **Store**: Singleton state manager (auth, server info, playlist, now playing, etc.)
  - **WikiApi**: Wikipedia biography + thumbnail fetches for artists
  - **SafeStorage**: Wrapper around localStorage with error handling
  - **Keyring**: Credential management via Rust backend
  - UI modules: AlbumBrowser, NowPlaying, Sidebar, SearchResults, VolumeControl, etc.
  - Event flow: Click handlers → API calls → Store updates → DOM re-render

- **style.css**: Light/dark mode support, responsive layout for 1200×800 default window

### Data Flow

```
Frontend (app.js)
    ↓ (tauriInvoke)
Rust Commands (main.rs)
    ├─ Subsonic API calls (reqwest) → reqwest::blocking::Response
    ├─ MD5 auth token generation
    └─ Audio playback (audio.rs)
         └─ StreamingReader (HTTP→rodio)
              └─ OS audio device (rodio)
    ↓ (tauri::emit)
Frontend event listeners (audio-bridge.js, app.js)
```

### Key Design Decisions

1. **Credentials in Keyring, Not localStorage**: The system keyring (libsecret on Linux) stores credentials securely. plaintext passwords never leak to JS.

2. **HTTP Streaming with Local Buffering**: `StreamingReader` keeps the HTTP connection open during playback so Subsonic/Navidrome sees "Now Playing" status for the full track duration, not just the download moment.

3. **Synchronous HTTP Blocking**: `reqwest::blocking` is used instead of async to simplify integration with rodio's Decoder, which expects a synchronous Read+Seek source.

4. **UUID-Based Session Tracking**: Each audio playback gets a UUID. Multiple devices can play concurrently; each has its own session in the `AudioPlayer` map.

5. **Volume Isolation Per Device**: `MixerDeviceSink` allows independent volume control per audio output device, not just global volume.

## Build & Run

### Prerequisites
- Rust 1.70+ (for MSRV)
- Node.js 18+ (for npm)
- On Linux: `libssl-dev`, `libxdo-dev`, `libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev` for Tauri dependencies
- On Linux: `libsecret-1-dev` for keyring integration

### Commands

```bash
# Install dependencies
npm install

# Develop: Build Rust backend + serve frontend in Tauri dev window
npm run tauri dev
# (Rebuilds Rust on changes; hot reload for frontend JS/CSS not available)

# Release build
npm run build:arch
# Builds .deb + .rpm, then runs makepkg in src-tauri/target/release/bundle/arch/
```

### First-Time Setup

1. Clone repo and `npm install` in the root
2. Ensure Rust is installed: `rustup default stable`
3. On Linux, install system dependencies (exact names vary by distro; Tauri docs list them)
4. Run `npm run tauri dev` to start the dev window
5. In-app: enter a Subsonic/Navidrome server URL, username, and password
6. Credentials are saved to the OS keyring; server address is stored in localStorage

## Development Notes

### Modifying Rust Commands
- Add new `#[tauri::command]` functions in `main.rs`
- Register them in `lib.rs` via `tauri::generate_handler![]` macro
- Update `capabilities/default.json` to add the command to the allowed list
- Restart dev server: `npm run tauri dev`

### Adding Audio Playback Features
- Playback logic lives in `audio.rs`. New playback methods (e.g., crossfade, equalizer) belong there.
- All changes must maintain thread-safety (Arc, Mutex, RwLock).
- Sessions are identified by UUID; use `AudioPlayer::get_state(session_id)` to query state.

### Frontend State Management
- All mutable state in `Store` singleton (auth, server info, playlist, now-playing, search results, etc.).
- DOM updates via direct manipulation (no virtual DOM). After changing `Store`, manually update affected DOM nodes.
- API calls use the `Api` helper; responses are type-checked manually (no TypeScript).

### Debugging Rust Backend
- `eprintln!()` prints to dev server console
- Use `RUST_BACKTRACE=1 npm run tauri dev` for panic backtraces
- `sysinfo` crate queries hardware; check `get_machine_info()` for diagnostics output

### Debugging Frontend
- Dev window has DevTools: press F12 or `Ctrl+Shift+I`
- Console logs visible in DevTools + stderr from dev server
- Network tab shows Subsonic API requests (Content-Security-Policy allows http://* for local servers)

## Testing

Currently no automated tests. Manual testing workflow:
1. Start dev server: `npm run tauri dev`
2. Log into a local Subsonic/Navidrome instance
3. Test playback, seeking, pause/resume, volume control
4. Test cover art caching (should be cached on second view)
5. Test search and artist Wikipedia bio fetches

## Packaging & Distribution

- `tauri.conf.json` defines the build, bundles (deb, rpm), and updater endpoints
- Updater signature in `tauri.conf.json` points to GitHub releases; update the pubkey if rotating signing keys
- Linux .desktop file for app launcher: `firmium.desktop` (bundled by Tauri)
- Icon files in `src-tauri/icons/` (32x32, 128x128, 128x128@2x, icon.icns, icon.ico)

## Key Files

- `src-tauri/src/main.rs` — Tauri command definitions & auth
- `src-tauri/src/audio.rs` — Audio playback engine
- `src/app.js` — Application state + UI logic
- `src/audio-bridge.js` — Tauri IPC bridge
- `src-tauri/tauri.conf.json` — App metadata, bundler config, updater settings
- `src-tauri/capabilities/default.json` — Tauri permissions (security scoping)
- `package.json` — npm scripts for build/dev

## Subsonic API Integration

The app uses Subsonic REST API (versions 1.12–1.16+). Requests include:
- `u` (username), `t` (MD5-hashed token), `s` (random salt), `v` (API version), `c` (client name)
- MD5 hashing done on Rust side; plaintext password sent to Rust, never leaves frontend
- OpenSubsonic extension detection for forward compatibility

Common endpoints used: `getArtists`, `getAlbum`, `search3`, `stream`, `getCoverArt`, `scrobble`, etc.

## Versioning

Always use semantic verisoning
- Automatically change files to reflect the new version
- List of files is found in /info/claude/files.txt

## Changelogs

- Always output change-logs to extra/changelogs
- Use the .md format
- File names should follow this `RELEASE_VerisonNumber`
- To generate a change log - compare the local files to the Git repo

## Performance Considerations

- **Cover Art Caching**: Blob URLs cached in memory (limit: 150 entries); oldest entries evicted when limit exceeded
- **Album Fetching**: Paginated with `maxItems=500` (Subsonic API limit)
- **Search**: Limited to 40 albums, 100 songs per query (configurable in app.js constants)
- **Playback Concurrency**: Only one audio stream per device active at a time; multiple devices can play different streams concurrently
- **CPU**: Release build has `opt-level = 2` + LTO enabled; `strip = false` keeps debug symbols for crash reporting

## Future Considerations

- Automated test suite (unit tests for audio module, integration tests for API)
- Hot reload for frontend during dev (would require custom Tauri dev server)
- Equalizer or DSP effects (extend audio.rs)
- Playlist persistence beyond session
- Scrobbling to ListenBrainz (API already supports it; UI not yet wired)
