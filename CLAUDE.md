# CLAUDE.md

**Version**: 5.0.0

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Firmium** is an OpenSubsonic music streaming client. The desktop app (Linux + Windows) is built with Tauri 2 (Rust backend + Svelte frontend), providing low-latency audio playback, OS-level credential storage, and integration with OpenSubsonic-compatible servers (e.g. Navidrome). A separate native Android app lives in `android/`, built with Kotlin + Jetpack Compose.

### Tech Stack

**Desktop (Linux, Windows)**
- **Frontend**: Svelte 5 + TypeScript, bundled via Vite
- **Backend**: Rust 2021 edition, Tauri 2.11+
- **Audio**: `rodio` 0.22 for native OS audio engine integration
- **HTTP**: `reqwest` 0.13 for async OpenSubsonic API calls
- **Credentials**: OS keyring via `keyring` crate (libsecret on Linux, Windows Credential Manager on Windows)
- **Packaging**: Linux (deb, rpm, Arch makepkg), Windows (NSIS installer)

**Android**

See [android/CLAUDE.md](android/CLAUDE.md) for the Android tech stack and architecture.

## Architecture

### Rust Backend (src-tauri/src/)

The backend exposes Tauri commands that the frontend invokes via `src/lib/audio-bridge.ts` and `src/lib/tauri.ts`. Key modules:

- **lib.rs**: Tauri app entry point. Defines `run()`, sets up the app, and registers all commands via `tauri::generate_handler![]`. Command implementations live in `commands/`.

- **commands/**: Command modules, re-exported via `commands/mod.rs`:
  - `themes.rs`: `list_themes()` — reads `.toml` theme files
  - `mappers.rs`: `map_albums()`, `map_artists()`, `map_songs()` — Rust-side mapping of raw Subsonic JSON to typed structs (including `infer_release_type()`)
  - `auth.rs`: `generate_auth_params()` — MD5 token hashing
  - `credentials.rs`: `save_password()`, `get_password()`, `delete_password()` — OS keyring
  - `playback.rs`: `play_stream()`, `preload_stream()`, `pause_playback()`, `resume_playback()`, `stop_playback()`, `seek_position()`, `set_volume()`, `get_volume()`, `crossfade_to()`, `get_playback_state()`, `is_playback_finished()`, `get_track_duration()`, `get_current_position()`, `list_audio_devices()` — delegate to rodio `AudioPlayer`
  - `logging.rs`: `write_log()`, `delete_logs()`, `get_log_path()`, `is_debug_mode()`, `get_app_version()`

- **audio.rs**: Desktop-only audio playback module. Core design:
  - `StreamingReader`: Implements Read+Seek over HTTP response body. Bytes buffered locally to keep Subsonic "Now Playing" status during playback.
  - `AudioPlayer`: Manages session lifecycle (loading → playing → paused/stopped). Uses `rodio::MixerDeviceSink` for per-device volume control. Thread-safe via `parking_lot::Mutex`.
  - Session state: `PlaybackState` enum (Loading, Playing, Paused, Stopped)
  - Sessions stored in `Arc<RwLock<HashMap>>` — playback events fire via Tauri `emit()` to frontend
  - Supports `preload_stream()` and `crossfade_to()` for gapless playback

- **main.rs**: Thin entry point that calls `lib::run()`. No commands defined here.

### Svelte Frontend (src/)

Single-page Svelte 5 app bundled by Vite. Hot reload works for all frontend changes during dev.

- **App.svelte**: Root component. Handles auth check on mount, theme/decorations, view routing, and global overlay components (LyricsPanel, PlaylistMenu).
- **components/**: Shared UI components
  - `PlayerBar.svelte` — persistent bottom player with controls, seek bar, volume
  - `Sidebar.svelte` — navigation sidebar
  - `LyricsPanel.svelte` — synced/unsynced lyrics overlay
  - `PlaylistMenu.svelte` — context menu for adding tracks to playlists
  - `Setup.svelte` — initial server login screen
- **views/**: Full-page view components (one per route)
  - `HomeView.svelte`, `AlbumList.svelte`, `AlbumDetail.svelte`, `ArtistList.svelte`, `ArtistDetail.svelte`
  - `SearchView.svelte`, `PlaylistsView.svelte`, `PlaylistDetail.svelte`, `Settings.svelte`
- **lib/**: Logic modules (no UI)
  - `stores.ts` — all Svelte writable/derived stores (auth, queue, playback state, lyrics, playlists, etc.)
  - `playback.ts` — `playAt()`, `crossfadeToNext()`, position tracking, lyrics sync, bridge event wiring
  - `audio-bridge.ts` — `AudioBridge` class: wraps Tauri IPC calls for play/pause/seek/volume, status polling loop
  - `api.ts` — `Api` (OpenSubsonic REST client), `OpenSubsonicRouter` (URL builder), `Keyring`, `WikiApi`
  - `playerControls.ts` — shared player control logic
  - `icons.ts` — SVG icon helpers
  - `coverCache.ts` — in-memory blob URL cache (50MB byte budget, LRU eviction)
  - `utils.ts` — `SafeStorage` (localStorage wrapper), misc helpers
  - `tauri.ts` — thin `tauriInvoke()` wrapper
  - `lazyLoad.ts` — IntersectionObserver-based lazy image loading
  - `lyrics.ts` — lyrics fetch + parse logic
  - `playlistMenu.ts` — playlist context menu state helpers
- **style.css**: Light/dark mode support, responsive layout; includes mobile-specific styles

### Data Flow

```
Svelte components / lib/playback.ts
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

### Android App

Native Kotlin/Compose app in `android/`, independent of the Tauri build, sharing the OpenSubsonic API contract with the desktop app. See [android/CLAUDE.md](android/CLAUDE.md) for its architecture, build commands, and conventions.

### Key Design Decisions

1. **Credentials in Keyring, Not localStorage**: The system keyring (libsecret on Linux) stores credentials securely. plaintext passwords never leak to JS.

2. **HTTP Streaming with Local Buffering**: `StreamingReader` keeps the HTTP connection open during playback so Subsonic/Navidrome sees "Now Playing" status for the full track duration, not just the download moment.

3. **Synchronous HTTP Blocking**: `reqwest::blocking` is used instead of async to simplify integration with rodio's Decoder, which expects a synchronous Read+Seek source.

4. **UUID-Based Session Tracking**: Each audio playback gets a UUID. Multiple devices can play concurrently; each has its own session in the `AudioPlayer` map.

5. **Volume Isolation Per Device**: `MixerDeviceSink` allows independent volume control per audio output device, not just global volume.

### Known Cross-Platform Divergences

Desktop (Tauri/Rust/Svelte) and Android (Kotlin/Compose) implement the same
features independently and have drifted in some areas. These are intentional
or at least currently-accepted differences — don't "fix" one to match the
other without checking with the user first:

1. **Release type inference**: `commands/mappers.rs::infer_release_type()`
   (desktop) returns lowercase `"single"/"ep"/"album"` with title-text and
   songCount fallback heuristics. `ApiClient.kt::inferReleaseType()` (Android)
   returns Title Case `"Single"/"EP"/"Album"/"Compilation"/"Live"/"Remix"`,
   checks `isCompilation` first, and has no title/songCount fallback at that
   layer — `AlbumListScreen.kt::effectiveType()` does separate songCount-based
   reclassification on the Android side.

2. **Crossfade gain handling**: Desktop's `AudioPlayer::crossfade_to` (Rust,
   `audio.rs`) does not apply ReplayGain during the fade ramp. Android's
   `AudioPlayer.crossfadeTo` multiplies by `gain` during the ramp.

3. **Queue/playback model**: Android runs a single ExoPlayer instance with the
   full playlist loaded (native gapless/queue management). Desktop runs
   per-track `rodio` sessions with manual preload-and-promote to the next
   session for gapless/crossfade transitions.

## Build & Run

### Prerequisites
- Rust 1.70+ (for MSRV)
- Node.js 18+ (for npm)
- On Linux: `libssl-dev`, `libxdo-dev`, `libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libsecret-1-dev` for Tauri + keyring
- On Windows: no extra system dependencies needed (rustls handles TLS, Windows Credential Manager is built-in)

### Commands

```bash
# Install dependencies
npm install

# Develop: Build Rust backend + serve frontend via Vite in Tauri dev window
npm run dev:app
# Rust recompiles on .rs changes; Svelte/CSS/JS changes hot-reload instantly via Vite.

# Release build (Linux only)
npm run release
# Builds .deb + .rpm, then runs makepkg in src-tauri/target/release/bundle/arch/

# Android (separate native app in android/)
npm run android:build   # assembleRelease via Gradle
npm run android:debug   # assembleDebug via Gradle
npm run android:install # installDebug via adb
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
- Update `capabilities/default.json` to add the command to the allowed list
- Restart dev server: `npm run dev:app`

### Adding Audio Playback Features
- Playback logic lives in `audio.rs`. New playback methods (e.g., equalizer) belong there.
- All changes must maintain thread-safety (Arc, Mutex, RwLock).
- Sessions are identified by UUID; use `AudioPlayer::get_state(session_id)` to query state.
- Crossfade is implemented in Rust: `AudioPlayer::crossfade_to()` in `audio.rs` ramps volume between the outgoing and incoming sessions. The frontend (`src/lib/playback.ts`) decides *when* to trigger it and calls into Rust via `AudioBridge`; it does not perform the fade itself.

### Frontend State Management
- All mutable app state lives in Svelte stores (`src/lib/stores.ts`).
- Components subscribe reactively — update the store, the UI updates automatically.
- Playback orchestration (play, crossfade, position tracking, lyrics sync) is in `src/lib/playback.ts`.
- API calls use `Api` from `src/lib/api.ts`; the frontend is written in TypeScript, with response types defined in `src/lib/types/`.

### Debugging Rust Backend
- `eprintln!()` prints to dev server console
- Use `RUST_BACKTRACE=1 npm run dev:app` for panic backtraces

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
5. Test search and artist bio fetches

## Packaging & Distribution

- `tauri.conf.json` defines the build, bundles (deb, rpm, nsis), and the in-app updater config
- `bundle.createUpdaterArtifacts: true` makes `tauri-action` (in `release.yml`) generate `.sig`
  files and a `latest.json` manifest for each tagged release
- The in-app updater (`@tauri-apps/plugin-updater` + `src/lib/updater.ts`, surfaced under
  Settings > Debug > Software Update) only covers **nsis (Windows)** and **AppImage (Linux)**
  bundles — the updater protocol can't self-update `.deb`/`.rpm` packages (no privilege
  escalation), so those users continue to update via their package manager / COPR. The
  current `bundle.targets` (`deb`, `rpm`, `nsis`) means the in-app updater is effectively a
  Windows-only feature today; adding an `appimage` target would extend it to Linux
  AppImage users.
- `plugins.updater.endpoints` in `tauri.conf.json` points at
  `https://github.com/fossisawesome/firmium/releases/latest/download/latest.json`;
  `plugins.updater.pubkey` must match the public half of the keypair whose private key is
  stored in the `TAURI_SIGNING_PRIVATE_KEY`/`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` GitHub
  secrets used by `release.yml`. Rotate both together with `npm run tauri signer generate`.
- Android: no in-app updater (native Kotlin app, not part of the Tauri build) — updates via
  Play Store or manual APK install, which is the standard/expected Android update path
- Linux .desktop file for app launcher: `firmium.desktop` (bundled by Tauri)
- Icon files in `src-tauri/icons/` (32x32, 128x128, 128x128@2x, icon.icns, icon.ico)

## Documentation

- End-user documentation (installing, building from source, usage, custom themes, and a settings reference) lives in the `firmium-docs` repo, in `src/content/*.md`, built with Vite + Svelte and deployed via GitHub Pages
- For which changes require updating which docs page, see agents.md "Keep Docs in Sync"

## Key Files

- `src-tauri/src/lib.rs` — All Tauri command definitions, app entry point
- `src-tauri/src/main.rs` — Thin entry point that calls `lib::run()`
- `src-tauri/src/audio.rs` — Audio playback engine (rodio)
- `src/App.svelte` — Root component, auth bootstrap, view routing
- `src/lib/stores.ts` — All Svelte stores (single source of truth for app state)
- `src/lib/playback.ts` — Playback orchestration, position tracking, lyrics sync
- `src/lib/audio-bridge.ts` — Tauri IPC bridge (`AudioBridge` class)
- `src/lib/api.ts` — OpenSubsonic API client, URL builder, keyring, WikiApi
- `src-tauri/tauri.conf.json` — App metadata, bundler config, updater settings
- `src-tauri/capabilities/default.json` — Tauri permissions (security scoping)
- `themes/` — TOML theme files
- `vite.config.ts` — Vite + Svelte plugin config
- `package.json` — npm scripts for build/dev
- `android/` — Separate native Kotlin/Compose Android app (not part of the Tauri build); see [android/CLAUDE.md](android/CLAUDE.md)

## OpenSubsonic API Integration

The app targets the OpenSubsonic REST API (v1.16.1). Legacy Subsonic servers are tolerated but unsupported. Requests include:
- `u` (username), `t` (MD5-hashed token), `s` (random salt), `v=1.16.1`, `c=firmium`, `f=json`
- MD5 hashing done on Rust side; plaintext password sent to Rust, never leaves frontend
- `openSubsonicExtensions` detected on every response and stored in the `openSubsonicExtensions` Svelte store
- OpenSubsonic fields used as primary: `displayArtist`, `releaseTypes[]`, `replayGain`, `bpm`, `genres[]`, `isCompilation`
- Settings page shows a server badge ("OpenSubsonic" or "Subsonic") based on detected capabilities

Common endpoints used: `getArtists`, `getAlbum`, `search3`, `stream`, `getCoverArt`, `scrobble`, etc.

## Versioning

- Always use semantic versioning

## Comments

- Add a comment above new code only when the WHY is non-obvious (a hidden constraint, a workaround, a subtle invariant). Well-named code doesn't need a comment explaining WHAT it does.
- Use existing comments to understand surrounding code.

## Performance Considerations

- **Cover Art Caching**: Blob URLs cached in memory up to a 50MB total budget (`MAX_BYTES` in `coverCache.ts`); least-recently-used entries evicted when the budget is exceeded
- **Album Fetching**: Paginated with `maxItems=500` (Subsonic API limit)
- **Search**: Limited to 40 albums, 100 songs per query (configurable in `src/lib/api.ts` constants)
- **Playback Concurrency**: Only one audio stream per device active at a time; multiple devices can play different streams concurrently
- **CPU**: Release build has `opt-level = 3` + LTO + `codegen-units = 1`; `strip = false` keeps debug symbols for crash reporting

# Foundational Thinking Principles

These principles apply to all interactions: conversations, code, debugging, planning, anything.

## 1. Think Before Acting

**Don't assume. Surface confusion. Present tradeoffs explicitly.**

Before committing to a direction:
- State your assumptions explicitly—especially about constraints: Is this for production or a one-off? Any performance targets? Does it need to integrate with existing code? Does it need to work in a specific environment?
- If uncertain, name it. Don't hide confusion behind confident-sounding recommendations.
- If multiple interpretations exist, present them—don't pick silently.
- If a simpler approach exists, mention it. Suggest it.
- If something is unclear, stop. Say what's confusing. Ask.

This applies to code, architecture, debugging, and conversations. Clarity first.

## 2. Simplicity First

**Minimum code/explanation that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it—don't delete it.

When your changes create orphans (unused imports, dead variables):
- Remove them.
- Verify pre-existing "dead" code really is dead code - and then remove it.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

If you can't write a test for it, the goal isn't clear enough. That's a forcing function—it surfaces vague requirements before you waste time coding.

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**For autonomous tool use and multi-step workflows, see `agents.md`.**

## Meta: Guidelines Are Defaults, Not Laws

If you explicitly say "I want this abstracted," "I need error handling for X," or "performance matters more than simplicity here," that overrides the guidelines above. The principles are defaults for when direction is unclear. Your judgment always wins.

## 5. Verify, Don't Assume Implementation Details

**Don't assume the user's environment, tools, or IDE capabilities.**
*
Before recommending something, consider:
- Does their IDE support X? (Ask or check, don't assume.)
- Is tool Y installed in their environment? (Verify or provide install steps.)
- Can their OS do Z? (Check constraints first—especially true for macOS, Linux kernels, terminal emulators.)
- Are they on a supported version? (Test environment assumptions.)

This catches silent failures. A recommendation that works on your machine but breaks on theirs is worse than no recommendation.

# Extra (still important)

These also apply to anything

## Dependancys

**Always research/web search dependancys before you add them.**

This helps:
- Make dependancys are up to date.
- Confirms dependancys are still safe to use - no supply chain attacks.
- Also - dont use a dependancy if you dont have to. Unless theres a real need for a dependancy - most of them can be easily be remade here. Expections apply if doing it here - woukd be geneiunlly a stupid, and unmainatble task.

