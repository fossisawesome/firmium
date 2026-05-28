# CLAUDE.md

**Version**: 3.1.2

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Firmium** is a cross-platform OpenSubsonic music streaming client built with Tauri 2 (Rust backend + JavaScript frontend). It targets Linux desktop and Android. It provides low-latency audio playback, OS-level credential storage, and integrates with OpenSubsonic-compatible servers (e.g. Navidrome).

### Tech Stack
- **Frontend**: Svelte 5, bundled via Vite
- **Backend**: Rust 2021 edition, Tauri 2.11+
- **Audio (desktop)**: `rodio` 0.22 for native OS audio engine integration
- **Audio (Android)**: ExoPlayer via Kotlin `AudioPlugin`
- **HTTP**: `reqwest` 0.13 for async OpenSubsonic API calls
- **Credentials (desktop)**: OS keyring (libsecret on Linux) via `keyring` crate
- **Credentials (Android)**: Android Keystore-backed `EncryptedSharedPreferences` via Kotlin `SecureStoragePlugin`
- **Packaging**: Linux (deb, rpm, Arch makepkg), Android APK via GitHub Actions

## Architecture

### Rust Backend (src-tauri/src/)

The backend exposes Tauri commands that the frontend invokes via `src/lib/audio-bridge.js` and `src/lib/tauri.js`. Key modules:

- **lib.rs**: Main command file and Tauri app entry point. Contains all `#[tauri::command]` functions and the `run()` function. Registers Android plugins. Key commands:
  - Themes: `list_themes()` — reads `.toml` theme files; on Android uses compile-time embedded themes
  - Data mappers: `map_albums()`, `map_artists()`, `map_songs()` — Rust-side mapping of raw Subsonic JSON to typed structs (including `infer_release_type()`)
  - Auth: `generate_auth_params()` — MD5 token hashing
  - Credentials: `save_password()`, `get_password()`, `delete_password()` — OS keyring on desktop, `SecureStoragePlugin` on Android
  - Audio: `play_stream()`, `preload_stream()`, `pause_playback()`, `resume_playback()`, `stop_playback()`, `seek_position()`, `set_volume()`, `get_volume()`, `crossfade_to()` — delegates to rodio `AudioPlayer` on desktop, `AudioPlugin` (ExoPlayer) on Android
  - Audio state: `get_playback_state()`, `is_playback_finished()`, `get_track_duration()`, `get_current_position()`
  - Now Playing (Android only): `update_now_playing()`, `update_playback_state()`, `clear_now_playing()` — delegates to `NowPlayingPlugin` (MediaSession + notification)
  - Diagnostics: `get_machine_info()`, `list_audio_devices()`
  - Logging: `write_log()`, `delete_logs()`, `get_log_path()`, `is_debug_mode()`, `get_app_version()`

- **audio.rs**: Desktop-only audio playback module. Core design:
  - `StreamingReader`: Implements Read+Seek over HTTP response body. Bytes buffered locally to keep Subsonic "Now Playing" status during playback.
  - `AudioPlayer`: Manages session lifecycle (loading → playing → paused/stopped). Uses `rodio::MixerDeviceSink` for per-device volume control. Thread-safe via `parking_lot::Mutex`.
  - Session state: `PlaybackState` enum (Loading, Playing, Paused, Stopped) — also defined in lib.rs for cross-platform use
  - Sessions stored in `Arc<RwLock<HashMap>>` — playback events fire via Tauri `emit()` to frontend
  - Supports `preload_stream()` and `crossfade_to()` for gapless playback
  - Not compiled on Android (`#[cfg(not(target_os = "android"))]`)

- **main.rs**: Thin entry point that calls `lib::run()`. No commands defined here.

### Android Kotlin Plugins (src-tauri/gen/android/)

Three Kotlin plugins bridge Tauri commands to Android APIs:
- **AudioPlugin**: ExoPlayer-based audio playback. Mirrors the rodio AudioPlayer API (play, pause, seek, crossfade, preload).
- **SecureStoragePlugin**: `EncryptedSharedPreferences` for credential storage (replaces OS keyring).
- **NowPlayingPlugin**: Android MediaSession + persistent notification with prev/play/next controls. JS uses `src/lib/nowPlaying.js` to drive it.

### Svelte Frontend (src/)

Single-page Svelte 5 app bundled by Vite. Hot reload works for all frontend changes during dev.

- **App.svelte**: Root component. Handles auth check on mount, theme/decorations, view routing, and global overlay components (LyricsPanel, PlaylistMenu).
- **components/**: Shared UI components
  - `PlayerBar.svelte` — persistent bottom player with controls, seek bar, volume (desktop)
  - `MobilePlayer.svelte` — full-screen now-playing player for Android
  - `QueueSheet.svelte` — bottom-sheet queue view for Android
  - `Sidebar.svelte` — navigation sidebar
  - `LyricsPanel.svelte` — synced/unsynced lyrics overlay
  - `PlaylistMenu.svelte` — context menu for adding tracks to playlists
  - `Setup.svelte` — initial server login screen
- **views/**: Full-page view components (one per route)
  - `HomeView.svelte`, `AlbumList.svelte`, `AlbumDetail.svelte`, `ArtistList.svelte`, `ArtistDetail.svelte`
  - `SearchView.svelte`, `PlaylistsView.svelte`, `PlaylistDetail.svelte`, `Settings.svelte`
- **lib/**: Logic modules (no UI)
  - `stores.js` — all Svelte writable/derived stores (auth, queue, playback state, lyrics, playlists, etc.)
  - `playback.js` — `playAt()`, `crossfadeToNext()`, position tracking, lyrics sync, bridge event wiring
  - `audio-bridge.js` — `AudioBridge` class: wraps Tauri IPC calls for play/pause/seek/volume, status polling loop
  - `api.js` — `Api` (OpenSubsonic REST client), `OpenSubsonicRouter` (URL builder), `Keyring`, `WikiApi`
  - `nowPlaying.js` — Android MediaSession notification helpers (`initNowPlaying`, `updateNowPlaying`, `clearNowPlaying`)
  - `platform.js` — `isMobile` flag (detects Android vs desktop); gates mobile-only code
  - `playerControls.js` — shared player control logic (used by both PlayerBar and MobilePlayer)
  - `icons.js` — SVG icon helpers
  - `coverCache.js` — in-memory blob URL cache (max 150 entries, LRU eviction)
  - `utils.js` — `SafeStorage` (localStorage wrapper), misc helpers
  - `tauri.js` — thin `tauriInvoke()` wrapper
  - `lazyLoad.js` — IntersectionObserver-based lazy image loading
  - `lyrics.js` — lyrics fetch + parse logic
  - `playlistMenu.js` — playlist context menu state helpers
- **style.css**: Light/dark mode support, responsive layout; includes mobile-specific styles

### Data Flow

```
Svelte components / lib/playback.js
    ↓ (AudioBridge → tauriInvoke)
Rust Commands (main.rs)
    ├─ OpenSubsonic API calls (reqwest) → reqwest::blocking::Response
    ├─ MD5 auth token generation
    └─ Audio playback (audio.rs)
         └─ StreamingReader (HTTP→rodio)
              └─ OS audio device (rodio)
    ↓ (status polling every 750ms via AudioBridge)
Svelte stores (playbackState, currentPosition, …) → reactive UI
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

# Develop: Build Rust backend + serve frontend via Vite in Tauri dev window
npm run dev:app
# Rust recompiles on .rs changes; Svelte/CSS/JS changes hot-reload instantly via Vite.

# Release build
npm run release
# Builds .deb + .rpm, then runs makepkg in src-tauri/target/release/bundle/arch/
```

### First-Time Setup

1. Clone repo and `npm install` in the root
2. Ensure Rust is installed: `rustup default stable`
3. On Linux, install system dependencies (exact names vary by distro; Tauri docs list them)
4. Run `npm run dev:app` to start the dev window
5. In-app: enter a Subsonic/Navidrome server URL, username, and password
6. Credentials are saved to the OS keyring; server address is stored in localStorage

## Development Notes

### Modifying Rust Commands
- Add new `#[tauri::command]` functions in `lib.rs`
- Register them in the `tauri::generate_handler![]` macro inside `run()` in `lib.rs`
- For Android-specific commands, use `#[cfg(target_os = "android")]` / `#[cfg(not(target_os = "android"))]`
- Update `capabilities/default.json` to add the command to the allowed list
- Restart dev server: `npm run dev:app`

### Adding Audio Playback Features
- Playback logic lives in `audio.rs`. New playback methods (e.g., equalizer) belong there.
- All changes must maintain thread-safety (Arc, Mutex, RwLock).
- Sessions are identified by UUID; use `AudioPlayer::get_state(session_id)` to query state.
- Crossfade is implemented entirely in the JS layer (`AudioBridge.startCrossfadeIn` in `lib/audio-bridge.js`).

### Frontend State Management
- All mutable app state lives in Svelte stores (`src/lib/stores.js`).
- Components subscribe reactively — update the store, the UI updates automatically.
- Playback orchestration (play, crossfade, position tracking, lyrics sync) is in `src/lib/playback.js`.
- API calls use `Api` from `src/lib/api.js`; responses are type-checked manually (no TypeScript).

### Debugging Rust Backend
- `eprintln!()` prints to dev server console
- Use `RUST_BACKTRACE=1 npm run dev:app` for panic backtraces
- `sysinfo` crate queries hardware; check `get_machine_info()` for diagnostics output

### Debugging Frontend
- Dev window has DevTools: press F12 or `Ctrl+Shift+I`
- Console logs visible in DevTools + the Vite dev server terminal output
- Network tab shows Subsonic API requests (Content-Security-Policy allows http://* for local servers)
- Svelte component state is inspectable via the Svelte DevTools browser extension

## Testing

Currently no automated tests. Manual testing workflow:
1. Start dev server: `npm run dev:app`
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

- `src-tauri/src/lib.rs` — All Tauri command definitions, Android plugin registration, app entry point
- `src-tauri/src/main.rs` — Thin entry point that calls `lib::run()`
- `src-tauri/src/audio.rs` — Desktop-only audio playback engine (rodio)
- `src/App.svelte` — Root component, auth bootstrap, view routing
- `src/lib/stores.js` — All Svelte stores (single source of truth for app state)
- `src/lib/playback.js` — Playback orchestration, position tracking, lyrics sync
- `src/lib/audio-bridge.js` — Tauri IPC bridge (`AudioBridge` class)
- `src/lib/api.js` — OpenSubsonic API client, URL builder, keyring, WikiApi
- `src/lib/nowPlaying.js` — Android MediaSession notification (mobile only)
- `src/lib/platform.js` — `isMobile` platform detection
- `src-tauri/tauri.conf.json` — App metadata, bundler config, updater settings
- `src-tauri/capabilities/default.json` — Tauri permissions (security scoping)
- `themes/` — TOML theme files (embedded at compile time for Android via `build.rs`)
- `vite.config.js` — Vite + Svelte plugin config
- `package.json` — npm scripts for build/dev

## OpenSubsonic API Integration

The app targets the OpenSubsonic REST API (v1.16.1). Legacy Subsonic servers are tolerated but unsupported. Requests include:
- `u` (username), `t` (MD5-hashed token), `s` (random salt), `v=1.16.1`, `c=firmium`, `f=json`
- MD5 hashing done on Rust side; plaintext password sent to Rust, never leaves frontend
- `openSubsonicExtensions` detected on every response and stored in the `openSubsonicExtensions` Svelte store
- OpenSubsonic fields used as primary: `displayArtist`, `releaseTypes[]`, `replayGain`, `bpm`, `genres[]`, `isCompilation`
- Settings page shows a server badge ("OpenSubsonic" or "Subsonic") based on detected capabilities

Common endpoints used: `getArtists`, `getAlbum`, `search3`, `stream`, `getCoverArt`, `scrobble`, etc.

## Versioning

- Always use semantic verisoning

## Comments

- Whenever creating something new - add a comment above it explaining what it does
- Use previous comments to get a better understanding of the code

## Changelogs

- Always output change-logs to extra/changelogs
- Use the .md format
- File names should follow this `RELEASE_v(verison-number)`
- To generate a change log - compare the local files to the Git repo
- Use semantic versioning (Look at #versioning(Versioning))

## Performance Considerations

- **Cover Art Caching**: Blob URLs cached in memory (limit: 150 entries); oldest entries evicted when limit exceeded
- **Album Fetching**: Paginated with `maxItems=500` (Subsonic API limit)
- **Search**: Limited to 40 albums, 100 songs per query (configurable in `src/lib/api.js` constants)
- **Playback Concurrency**: Only one audio stream per device active at a time; multiple devices can play different streams concurrently
- **CPU**: Release build has `opt-level = 2` + LTO enabled; `strip = false` keeps debug symbols for crash reporting

## Future Considerations

- Automated test suite (unit tests for audio module, integration tests for API)
- Equalizer or DSP effects (extend audio.rs)
- Playlist persistence beyond session
- Scrobbling to ListenBrainz (API already supports it; UI not yet wired)
