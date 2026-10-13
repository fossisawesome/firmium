# CLAUDE.md

**Version**: 9.0.1

Guidance for Claude Code (claude.ai/code) when working in this repository.

## Project Overview

**Firmium** is an OpenSubsonic music streaming client. This repo is a Cargo workspace (`Cargo.toml` `[workspace] members = ["backend", "src-tauri", "termium"]`) with three crates: a shared `backend` (no UI), a [Tauri 2](https://tauri.app) desktop app (`src-tauri/` Rust shell + `src/` Svelte/SCSS/TS UI), and a `ratatui` terminal client (`termium`). Separate native Android app in `android/`, built with Kotlin + Jetpack Compose, sharing the same OpenSubsonic API contract but no Rust code.

### Tech Stack

**Shared backend (`backend/` crate, `firmium-backend`)** — plain Rust, no UI, consumed by both `desktop` and `termium` via `path` dependency.

**Desktop (`src-tauri/` + `src/` — Linux, Windows, macOS)**
- **UI**: Svelte 5 + SCSS + TypeScript in `src/`, rendering only. All logic is Rust: `backend/core/` (settings, accounts, library routing, playlists, radio, stats) behind `src-tauri/src/commands.rs`; the UI talks to it via `src/lib/api.ts` (invoke) and Tauri events (`playback-*`, `queue-state-changed`, `session-expired`).
- **Audio**: `symphonia` 0.5 (decoding) + `cpal` 0.17 (output device I/O), hand-rolled engine
- **HTTP**: `reqwest` 0.13 for async OpenSubsonic API calls
- **Credentials**: OS keyring via `keyring` crate (libsecret/Secret Service on Linux, Windows Credential Manager on Windows, Keychain via `apple-native` on macOS)
- **Packaging**: Linux (deb, rpm, Arch makepkg), Windows (NSIS installer), macOS (`.dmg`, unsigned)

**Termium (`termium/` crate — Linux, Windows, macOS terminal client)**
- **UI**: [ratatui](https://ratatui.rs) 0.29 + `crossterm` 0.28 (event-stream) — TUI, no GUI toolkit
- **Async**: `tokio` (rt-multi-thread) for backend calls and event bus
- **Scope**: library browsing (albums/artists/playlists/search/home), playback controls, persistent now-playing bar, configurable keybindings (`toml`-based). No lyrics/EQ/similar-tracks panels (desktop-only).
- Shares login/library/queue state with desktop and Android via the same OpenSubsonic server — logging into one, all see it.

**Android**

See [android/CLAUDE.md](android/CLAUDE.md) for Android tech stack and architecture.

## Architecture

### Backend (backend/)

The backend is its own crate (`firmium-backend`, `backend/` dir) — no UI, no IPC. `src-tauri` and `termium` depend on it via `path`; `src-tauri/src/commands.rs` forwards Tauri commands to `backend::core::Core`, termium awaits backend fns directly; the backend pushes playback/queue events back to the UI over an in-process event bus (`backend/events.rs`, a `tokio::sync::broadcast` channel). Key modules:

- **init.rs**: `Backend::new()` builds the shared handles (event bus, `AudioPlayer`, `AppState`, `QueueState`, optional `PlayHistory`) and starts the `queue_manager` background task. `core::Core` (settings, accounts, library routing, playlists, radio, stats, podcasts) wraps it and is held in Tauri state (`src-tauri/src/lib.rs`); termium holds the bare `Backend`.

- **events.rs**: `EventBus` (broadcast sender) and `BackendEvent` enum (`PlaybackStateChanged`, `PlaybackPosition`, `PlaybackFinished`, `QueueStateChanged`, `QueueExhausted`, `SessionExpired`). `src-tauri/src/events.rs` re-emits them as Tauri events.

- **state.rs**: `AppState` — holds `ConnectionState` (server URL, username, password, detected `openSubsonicExtensions`) behind `parking_lot::RwLock`, plus shared async `reqwest::Client` used by `commands/subsonic.rs` and `commands/lyrics.rs`, plus the `EventBus` handle. Set via `set_connection`.

- **commands/**: Plain async/sync fns, re-exported via `commands/mod.rs`:
  - `themes.rs`: `list_themes()` — reads `.toml` theme files
  - `mappers.rs`: `map_albums()`, `map_artists()`, `map_songs()` — Rust-side mapping of raw Subsonic JSON to typed structs (including `infer_release_type()` and `format_track_info()` for `Song.trackInfo`)
  - `auth.rs`: `generate_auth_params()` — MD5 token hashing
  - `credentials.rs`: `save_password()`, `get_password()`, `delete_password()` — OS keyring
  - `subsonic.rs`: `set_connection()`, `validate_connection()`, OpenSubsonic API — album/artist/search/genre reads, playlist CRUD, `scrobble()`, `save_play_queue()`/`get_play_queue()` (cross-device queue sync), `get_song_lyrics()` (structured → legacy → LRCLIB cascade), `get_sonic_similar_tracks()`. Internal `subsonic_request()` builds authenticated requests, emits `BackendEvent::SessionExpired` on the bus on HTTP 401 or OpenSubsonic error codes 40/41.
  - `lyrics.rs`: `parse_lrc()`, `fetch_lrclib_lyrics()` — LRC parsing and LRCLIB fallback used by `get_song_lyrics()`
  - `cover_cache.rs`: `get_cover_art()`, `clear_cover_cache()` — disk-based cover art cache (200MB budget, mtime-based LRU eviction); the UI shows cached files through `convertFileSrc`
  - `queue.rs` / `queue_manager.rs`: queue mutation (`set_queue`, `shuffle_and_play`, `play_queue_index`, …) and the background task that drains the bus for crossfade, gapless preload, track advance and scrobbling
  - `playback.rs`: thin wrappers over `AudioPlayer` (`play_stream`, `preload_stream`, pause/resume/stop, `seek_position`, `set_volume`, `crossfade_to`, `list_audio_devices`, …)
  - `downloads.rs`, `local_library.rs`, `equalizer.rs`, `stats.rs`, `cover_colors.rs`: offline downloads, `~/Music/Firmium` scan, EQ profiles, play-history aggregation (summary / export), dominant-cover-color extraction
  - `app_info.rs`: `get_app_version()`

- **audio/**: Desktop-only audio playback module (`symphonia` decode + `cpal` output). Core design:
  - `streaming_reader.rs`: `StreamingReader` implements Read+Seek over HTTP response body (bytes buffered locally to keep Subsonic "Now Playing" status during playback); `VecSource`/`FileSource` are seekable `MediaSource` wrappers for seek-rebuild and local files.
  - `decoder.rs`: `DecoderHandle` wraps `symphonia` `FormatReader`/`Decoder`, exposing `next_samples()` (interleaved f32), `seek()`, and `sample_rate`/`channels`/duration. Defaults to 48000 Hz if container doesn't report sample rate.
  - `session.rs`: `Session` holds `Mutex<VecDeque<f32>>` ring buffer plus playback state (volume, playing, position). `spawn_decode_feeder()` runs blocking decode loop (ReplayGain, EQ, 25ms fade-in, native seek via `SeekRequest`/`SeekReply`).
  - `output.rs`: cpal device negotiation (`find_compatible_config`, `open_with_config`/`open_default`) and realtime `mix_into` callback, which sums all active sessions' ring buffers (with per-session volume, channel adaptation, and linear-interpolation resampler that degenerates to passthrough when rates match).
  - `mod.rs`: `AudioPlayer` manages session lifecycle (loading → playing → paused/stopped), reopens output stream at each track's native sample rate via `reopen_stream_if_needed()` (deferred during crossfade). Thread-safe via `parking_lot::Mutex`/`RwLock`.
  - Session state: `PlaybackState` enum (Loading, Playing, Paused, Stopped)
  - Sessions stored in `Arc<RwLock<HashMap>>` — playback events fire on the `EventBus` (broadcast) consumed by `queue_manager` and the UI subscription
  - Supports `preload_stream()` and `crossfade_to()` for gapless playback

### Tauri UI (`src/`, `src-tauri/`)

- **src-tauri/src/**: `lib.rs` (builder, setup, handler list), `commands.rs` (thin `#[tauri::command]` forwarders, `CmdError {kind, message}`), `events.rs` (EventBus → Tauri events), `cli.rs` (`firmium <cmd>` control-socket client).
- **backend/core/**: all app logic: `settings.rs` (persisted prefs + side effects), `accounts.rs` (login/switch/keyring), `library.rs` (server vs offline routing, Mix, radio, auto-continue, podcasts, artist helpers), `playlists.rs` (local-first playlists + server sync), `stats.rs` (history summary, export).
- **src/lib/**: `ipc.ts` (invoke/listen), `api.ts` (one-line typed command wrappers), `types.ts`, `stores.ts` (mirrors of backend state + view routing, panels, toasts, theme application), `cover.ts` (image actions), `menu.ts`/`playlists.ts` (popup + list stores). No business logic in TS.
- **src/views, src/components**: Svelte 5 screens and widgets. **src/styles/**: SCSS partials (`main.scss` @uses them; `_spotify.scss` is the Spotify UI theme via `html[data-ui-theme]`).
- Themes: TOML under `themes/`, embedded in the backend (`commands/themes.rs`) and applied as CSS variables.

### Data Flow

```
src/ (Svelte)  user action → Api.xxx() → invoke
    ▼
src-tauri/commands.rs → backend::core::Core / backend::commands::…
    ├─ OpenSubsonic API (subsonic.rs) or offline library (local_library.rs)
    ├─ Cover art → disk cache (cover_cache.rs) → file path → convertFileSrc
    ├─ Lyrics cascade (subsonic.rs::get_song_lyrics → lyrics.rs) + word timing (core/library.rs)
    └─ Audio playback (audio/): StreamingReader → symphonia → cpal
    │  playback/queue events → EventBus → events.rs → Tauri events
    ▼
src/lib/stores.ts mirrors state → Svelte re-renders
```

Termium (`termium/src/`) follows the same backend, different front end: `app.rs` holds `App` state and the `ratatui` render/input loop, `keymap.rs` handles configurable keybindings, `ui/{home,albums,artists,playlists,search,login,player_bar}.rs` are the ratatui screen/widget renderers. No `Message`/`Task` abstraction — reads backend state directly and awaits async calls in its event loop.

### Android App

Native Kotlin/Compose app in `android/`, independent of the desktop and termium Rust binaries, sharing OpenSubsonic API contract with them. See [android/CLAUDE.md](android/CLAUDE.md) for architecture, build commands, and conventions.

### Key Design Decisions

1. **Credentials in Keyring, Not Config**: System keyring (libsecret on Linux) stores credentials securely. `config.toml` keeps only the server URL, username and saved-account list — plaintext passwords are never written to disk.

2. **HTTP Streaming with Local Buffering**: `StreamingReader` keeps HTTP connection open during playback so Subsonic/Navidrome sees "Now Playing" status for full track duration, not just download moment.

3. **Synchronous HTTP Blocking**: `reqwest::blocking` used instead of async to simplify integration with `symphonia`'s decoder, which expects synchronous Read+Seek source. Decoding runs on `spawn_blocking` task per session.

4. **UUID-Based Session Tracking**: Each audio playback gets UUID, with own `Session` (ring buffer, volume, position) in `AudioPlayer` map.

5. **Per-Session Volume**: Each `Session` has own volume, applied in shared `cpal` mixing callback (`audio/output.rs::mix_into`) — independent of global output device volume.

### Known Cross-Platform Divergences

Desktop (Tauri) and Android (Kotlin/Compose) implement same features independently and have drifted. These are intentional or accepted differences — don't "fix" one to match other without checking with user first:

1. **Release type inference**: `commands/mappers.rs::infer_release_type()` (desktop) returns lowercase `"single"/"ep"/"album"` with title-text and songCount fallback heuristics. `ApiClient.kt::inferReleaseType()` (Android) returns Title Case `"Single"/"EP"/"Album"/"Compilation"/"Live"/"Remix"`, checks `isCompilation` first, no title/songCount fallback — `AlbumListScreen.kt::effectiveType()` does separate songCount-based reclassification on Android side.

2. **Crossfade gain handling**: Desktop's `AudioPlayer::crossfade_to` (Rust, `audio/mod.rs`) doesn't apply ReplayGain during fade ramp. Android's `AudioPlayer.crossfadeTo` multiplies by `gain` during ramp.

3. **Queue/playback model**: Android runs single ExoPlayer instance with full playlist loaded (native gapless/queue management). Desktop runs per-track decode sessions (`audio/session.rs`) with manual preload-and-promote to next session for gapless/crossfade transitions.

## Build & Run

### Prerequisites
- Rust 1.80+ (`rustup default stable`)
- On Linux: ALSA (`libasound2`), `libssl`, `libsecret` (keyring), `libxkbcommon``webkit2gtk-4.1`, `xdotool`. Exact package names vary by distro — see `README.md` "System Dependencies".
- On Windows: no extra system dependencies (rustls handles TLS, Windows Credential Manager built-in)
- On macOS: Xcode Command Line Tools (`xcode-select --install`) only — CoreAudio and Keychain are built in, rustls handles TLS

### Commands

```bash
# Desktop GUI (debug build, recompiles on .rs changes)
npm run tauri dev

# Termium TUI
cargo run -p termium

# Optimized release binaries → target/release/{firmium,termium}
cargo build --release

# Android (separate native app in android/, built with Gradle)
cd android && ./gradlew assembleRelease   # release APK
cd android && ./gradlew assembleDebug     # debug APK
cd android && ./gradlew installDebug      # install on connected device
```

Desktop needs Node (Vite) and, on Linux, `webkit2gtk-4.1`. Termium is a plain Rust crate.

### First-Time Setup

1. Clone repo; ensure Rust is installed (`rustup default stable`)
2. On Linux, install system dependencies (see `README.md`)
3. Run `npm run tauri dev` to launch
4. In-app: enter Subsonic/Navidrome server URL, username, and password
5. Credentials saved to OS keyring; server URL + username stored in `config.toml`

## Development Notes

### Adding a UI Action / Backend Call (desktop)
- Put the logic in `backend/` (usually a `Core` method in `backend/core/`), add a `#[tauri::command]` forwarder in `src-tauri/src/commands.rs`, register it in `generate_handler!` in `src-tauri/src/lib.rs`, add a typed wrapper to `src/lib/api.ts` and any new shape to `src/lib/types.ts`.
- Async commands with borrowed `State` args must return `Result`. Types crossing the boundary need `Serialize` (`Deserialize` when sent from the UI), camelCase.

### Adding Audio Playback Features
- Playback logic in `audio/`. New playback methods (e.g., equalizer) belong there — `mod.rs` for public `AudioPlayer` API, `session.rs` for per-track decode/state, `output.rs` for cpal mixing callback.
- All changes must maintain thread-safety (Arc, Mutex, RwLock).
- Sessions identified by UUID; use `AudioPlayer::get_state(session_id)` to query state.
- Crossfade implemented in Rust: `AudioPlayer::crossfade_to()` in `audio/mod.rs` ramps volume between outgoing and incoming sessions. The `queue_manager` task decides *when* to trigger (reacting to `PlaybackPosition` events on the bus); the engine performs the fade.

### UI State Management (desktop)
- Real state lives in Rust (`Core`, `QueueState`, config). Svelte stores in `src/lib/stores.ts` only mirror what the backend emits/returns, plus view-only state (current view, open panel, toasts).

### Debugging
- `eprintln!()` prints to the terminal running `cargo run`.
- `RUST_BACKTRACE=1 cargo run` for panic backtraces.
- Blank/black window on Wayland or NVIDIA: try `WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri dev`.

## Testing

### Automated

Unit tests live in `#[cfg(test)] mod tests` blocks next to the code they cover (pure logic only — no network, no audio device, no keyring). Run:

```bash
cargo test --workspace   # backend + desktop + termium
cargo test -p firmium-backend   # backend only
```

Before considering any change done, also run clippy strict (treats warnings as errors) — required after every change, not just before commit:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

If clippy flags new code, fix it (don't suppress with `#[allow(...)]` unless the lint is a false positive — explain why in a comment if so).

### Manual

1. `npm run tauri dev`
2. Log into local Subsonic/Navidrome instance
3. Test playback, seeking, pause/resume, volume control
4. Test cover art caching (should be cached on second view)
5. Test search and artist fetches

## Packaging & Distribution

- Desktop installers come from the Tauri bundler: `npm run tauri:build` (targets in `src-tauri/tauri.conf.json`: deb, rpm, nsis, dmg) writes to `target/release/bundle/`. `release.yml` runs it per OS (`--bundles deb,rpm` on Ubuntu 22.04, `nsis` on Windows, `dmg` per arch on `macos-14`) and attaches the files with `gh release upload`. macOS builds are unsigned (no Apple Developer account).
- A bare `cargo build --release -p firmium` needs `--features custom-protocol` (embeds `dist/`; otherwise the binary loads the dev URL). `PKGBUILD` (Arch) and `firmium.spec` run `npm ci && npm run build` first; COPR (`packaging/firmium.spec`, `copr.yml`) ships prebuilt `dist/` + `cargo vendor` in the vendor tarball because mock builds are offline. Icons come from `src-tauri/icons/`.
- Termium: `release.yml` `build-termium-linux/windows/macos` jobs attach installers (no tarballs/zips): `.deb` + `.rpm` (Linux x86_64, via `cargo-deb`/`cargo-generate-rpm`, metadata in `termium/Cargo.toml`), an NSIS `.exe` (`packaging/termium.nsi`, Start Menu shortcut, no PATH entry) and a `.dmg` per macOS arch (binary only, unsigned). Not in AUR/COPR; or build from source via `cargo build --release -p termium`.
- `scripts/bump-version.sh <ver>` updates `Cargo.toml`, `package.json`, `src-tauri/tauri.conf.json`, `CLAUDE.md`, `PKGBUILD`, `firmium.spec`, the Android `build.gradle.kts`, and the AUR folders.
- **In-app updater**: not ported to the Tauri build yet; `.deb`/`.rpm`/COPR/AUR users update through their package manager.
- Android: no in-app updater (native Kotlin app) — updates via Play Store or manual APK install.
- Linux `.desktop` launcher: `packaging/firmium.desktop`. App icons under `src-tauri/icons/`.

## Documentation

- End-user docs (installing, building from source, usage, custom themes, settings reference) in `firmium-docs` repo, `src/content/*.md`, built with Vite + Svelte, deployed via GitHub Pages
- For which changes require updating which docs page, see AGENTS.md "Keep Docs in Sync"

## Key Files

- `src-tauri/src/` — Tauri shell (`lib.rs`, `commands.rs`, `events.rs`, `cli.rs`)
- `src/` — Svelte UI (`App.svelte`, `views/`, `components/`, `lib/`, `styles/`)
- `backend/core/` — app logic shared by UI shells
- `termium/src/main.rs` — Termium entry point
- `termium/src/app.rs` — Termium `App` state and ratatui render/input loop
- `termium/src/ui/` — Termium screen/widget renderers
- `backend/init.rs` — `Backend::new()`: builds shared handles, starts `queue_manager`
- `backend/events.rs` — `EventBus` + `BackendEvent` (backend → UI)
- `backend/state.rs` — `AppState` (connection + reqwest client + bus)
- `backend/config.rs` — `~/.config/<id>/config.toml` persistence, shared by desktop and termium
- `backend/audio/` — Audio playback engine (symphonia + cpal)
- `backend/commands/` — OpenSubsonic client, queue, lyrics, covers, downloads, EQ, stats
- `Cargo.toml` — workspace root (`members = ["backend", "src-tauri", "termium"]`)
- `themes/` — TOML theme files (embedded at compile time)
- `assets/` — bundled font and app icons (desktop)
- `packaging/` — `firmium.desktop`, rpm spec, `macos/Info.plist`
- `android/` — Separate native Kotlin/Compose Android app; see [android/CLAUDE.md](android/CLAUDE.md)

## OpenSubsonic API Integration

See [API.md](API.md) for full reference of all Subsonic/OpenSubsonic/Navidrome endpoints, authentication, and caveats.

App targets OpenSubsonic REST API (v1.16.1). Legacy Subsonic servers tolerated but unsupported. Requests include:
- `u` (username), `t` (MD5-hashed token), `s` (random salt), `v=1.16.1`, `c=firmium`, `f=json`
- MD5 hashing done on the Rust side; the plaintext password lives only in `AppState`/keyring
- `openSubsonicExtensions` detected on every response, stored in `AppState`'s `ConnectionState`
- OpenSubsonic fields used as primary: `displayArtist`, `releaseTypes[]`, `replayGain`, `bpm`, `genres[]`, `isCompilation`
- Settings page shows server badge ("OpenSubsonic" or "Subsonic") based on detected capabilities

Common endpoints: `getArtists`, `getAlbum`, `search3`, `stream`, `getCoverArt`, `scrobble`, etc.

## Versioning

- Always use semantic versioning

## Comments

- Add comment above new code only when WHY is non-obvious (hidden constraint, workaround, subtle invariant). Well-named code doesn't need WHAT explained.
- Use existing comments to understand surrounding code.

## Performance Considerations

- **Cover Art Caching**: Disk-based cache under app cache dir (`commands/cover_cache.rs`), up to 200MB budget; LRU (by mtime) eviction when budget exceeded. Persists across restarts; served via the asset protocol so covers survive restarts without re-downloading.
- **Album Fetching**: Paginated with `maxItems=500` (Subsonic API limit, `backend/commands/subsonic.rs`)
- **Long lists**: albums, artists, and album/playlist track lists use `src/lib/VirtualList.svelte`, which only renders rows on screen.
- **Search**: Limited to 40 albums, 100 songs per query (`commands/subsonic.rs::search`)
- **Playback Concurrency**: One audio stream per device active at a time; multiple devices can play different streams concurrently
- **CPU**: Release build has `opt-level = 3` + LTO + `codegen-units = 1`; `strip = false` keeps debug symbols for crash reporting

# Foundational Thinking Principles

Apply to all interactions: conversations, code, debugging, planning, anything.

## 1. Think Before Acting

**Don't assume. Surface confusion. Present tradeoffs explicitly.**

Before committing to direction:
- State assumptions explicitly — especially about constraints: production or one-off? Performance targets? Integrate with existing code? Specific environment?
- Uncertain: name it. Don't hide confusion behind confident recommendations.
- Multiple interpretations: present them, don't pick silently.
- Simpler approach exists: mention it.
- Something unclear: stop. Say what's confusing. Ask.

Applies to code, architecture, debugging, conversations. Clarity first.

## 2. Simplicity First

**Minimum code/explanation that solves problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" not requested.
- No error handling for impossible scenarios.
- If you write 200 lines and could be 50, rewrite it.

Ask: "Would senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- Unrelated dead code: mention it, don't delete it.

When changes create orphans (unused imports, dead variables):
- Remove them.
- Verify pre-existing "dead" code really is dead, then remove it.

Test: every changed line should trace directly to user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix bug" → "Write test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

Can't write test for it: goal isn't clear enough. Surfaces vague requirements before wasting time coding.

For multi-step tasks, state brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria = loop independently. Weak criteria = constant clarification.

---

**For autonomous tool use and multi-step workflows, see `AGENTS.md`.**

## Meta: Guidelines Are Defaults, Not Laws

If you say "I want this abstracted," "I need error handling for X," or "performance matters more than simplicity here," that overrides guidelines above. Principles are defaults for when direction is unclear. Your judgment always wins.

## 5. Verify, Don't Assume Implementation Details

**Don't assume user's environment, tools, or IDE capabilities.**

Before recommending something:
- Does their IDE support X? (Ask or check, don't assume.)
- Is tool Y installed? (Verify or provide install steps.)
- Can their OS do Z? (Check constraints first — especially macOS, Linux kernels, terminal emulators.)
- Supported version? (Test environment assumptions.)

Catches silent failures. Recommendation that works on your machine but breaks on theirs is worse than no recommendation.

# Extra (still important)

## Dependencies

**Always research/web search dependencies before adding them.**

- Ensures dependencies are up to date.
- Confirms dependencies still safe to use — no supply chain attacks.
- Don't use dependency if you don't have to. Unless there's real need, most can be remade here. Exceptions apply if doing it here would be genuinely stupid and unmaintainable.

## Questioning

**Always ask user questions and interrogate them.**

For example:
- Adding any features.
- Changing UI.
- Debugging.
- Etc.
