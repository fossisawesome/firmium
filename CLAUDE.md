# CLAUDE.md

**Version**: 8.0.1

Guidance for Claude Code (claude.ai/code) when working in this repository.

## Project Overview

**Firmium** is an OpenSubsonic music streaming client. This repo is a Cargo workspace (`Cargo.toml` `[workspace] members = ["backend", "desktop", "termium"]`) with three crates: a shared `backend` (no UI), a native [iced](https://iced.rs) desktop GUI (`desktop`), and a `ratatui` terminal client (`termium`) — both no web view, no JavaScript. Separate native Android app in `android/`, built with Kotlin + Jetpack Compose, sharing the same OpenSubsonic API contract but no Rust code.

### Tech Stack

**Shared backend (`backend/` crate, `firmium-backend`)** — plain Rust, no UI, consumed by both `desktop` and `termium` via `path` dependency.

**Desktop (`desktop/` crate — Linux, Windows, macOS, FreeBSD)**
- **UI**: [iced](https://iced.rs) 0.14 (pure Rust; `canvas` for the visualizer, `svg` for icons, bundled font)
- **Language**: Rust 2021 edition — UI and backend in one process, one crate
- **Audio**: `symphonia` 0.5 (decoding) + `cpal` 0.17 (output device I/O), hand-rolled engine
- **HTTP**: `reqwest` 0.13 for async OpenSubsonic API calls
- **Credentials**: OS keyring via `keyring` crate (libsecret/Secret Service on Linux and FreeBSD, Windows Credential Manager on Windows, Keychain via `apple-native` on macOS)
- **Packaging**: Linux (deb, rpm, Arch makepkg), Windows (NSIS installer), macOS (`.app` bundle in a `.dmg`, unsigned), FreeBSD (`.pkg` via `pkg create`)

**Termium (`termium/` crate — Linux, Windows, macOS, FreeBSD terminal client)**
- **UI**: [ratatui](https://ratatui.rs) 0.29 + `crossterm` 0.28 (event-stream) — TUI, no GUI toolkit
- **Async**: `tokio` (rt-multi-thread) for backend calls and event bus
- **Scope**: library browsing (albums/artists/playlists/search/home), playback controls, persistent now-playing bar, terminal bar visualizer, configurable keybindings (`toml`-based). No lyrics/EQ/recap/similar-tracks panels (desktop-only).
- Shares login/library/queue state with desktop and Android via the same OpenSubsonic server — logging into one, all see it.

**Android**

See [android/CLAUDE.md](android/CLAUDE.md) for Android tech stack and architecture.

## Architecture

### Backend (backend/)

The backend is its own crate (`firmium-backend`, `backend/` dir) — no UI, no IPC. Both `desktop` and `termium` depend on it via `path = "../backend"` and call these modules directly via async task spawning (`iced::Task::perform` on desktop, `tokio::spawn`/awaits on termium) or inline (sync fns); the backend pushes playback/queue events back to the UI over an in-process event bus (`backend/events.rs`, a `tokio::sync::broadcast` channel). Key modules:

- **init.rs**: `Backend::new()` builds the shared handles (event bus, `AudioPlayer`, `AppState`, `QueueState`, optional `PlayHistory`) and starts the `queue_manager` background task. Held by the iced `App` in `desktop/src/app/mod.rs` (and by termium's `App` in `termium/src/app.rs`).

- **events.rs**: `EventBus` (broadcast sender) and `BackendEvent` enum (`PlaybackStateChanged`, `PlaybackPosition`, `PlaybackFinished`, `QueueStateChanged`, `QueueExhausted`, `SessionExpired`). The UI subscribes via an `iced::Subscription` that bridges the broadcast channel into `Message::Backend(BackendEvent)`.

- **state.rs**: `AppState` — holds `ConnectionState` (server URL, username, password, detected `openSubsonicExtensions`) behind `parking_lot::RwLock`, plus shared async `reqwest::Client` used by `commands/subsonic.rs` and `commands/lyrics.rs`, plus the `EventBus` handle. Set via `set_connection`.

- **commands/**: Plain async/sync fns, re-exported via `commands/mod.rs`:
  - `themes.rs`: `list_themes()` — reads `.toml` theme files
  - `mappers.rs`: `map_albums()`, `map_artists()`, `map_songs()` — Rust-side mapping of raw Subsonic JSON to typed structs (including `infer_release_type()` and `format_track_info()` for `Song.trackInfo`)
  - `auth.rs`: `generate_auth_params()` — MD5 token hashing
  - `credentials.rs`: `save_password()`, `get_password()`, `delete_password()` — OS keyring
  - `subsonic.rs`: `set_connection()`, `validate_connection()`, OpenSubsonic API — album/artist/search/genre reads, playlist CRUD, `scrobble()`, `save_play_queue()`/`get_play_queue()` (cross-device queue sync), `get_song_lyrics()` (structured → legacy → LRCLIB cascade), `get_sonic_similar_tracks()`. Internal `subsonic_request()` builds authenticated requests, emits `BackendEvent::SessionExpired` on the bus on HTTP 401 or OpenSubsonic error codes 40/41.
  - `lyrics.rs`: `parse_lrc()`, `fetch_lrclib_lyrics()` — LRC parsing and LRCLIB fallback used by `get_song_lyrics()`
  - `cover_cache.rs`: `get_cover_art()`, `clear_cover_cache()` — disk-based cover art cache (200MB budget, mtime-based LRU eviction); the UI loads cached files into an `iced::widget::image::Handle` cached in `App`
  - `queue.rs` / `queue_manager.rs`: queue mutation (`set_queue`, `shuffle_and_play`, `play_queue_index`, …) and the background task that drains the bus for crossfade, gapless preload, track advance and scrobbling
  - `playback.rs`: thin wrappers over `AudioPlayer` (`play_stream`, `preload_stream`, pause/resume/stop, `seek_position`, `set_volume`, `crossfade_to`, `list_audio_devices`, …)
  - `downloads.rs`, `local_library.rs`, `equalizer.rs`, `stats.rs`, `cover_colors.rs`: offline downloads, `~/Music/Firmium` scan, EQ profiles, play-history aggregation (Recap / export), dominant-cover-color extraction
  - `app_info.rs`: `get_app_version()`

- **audio/**: Desktop-only audio playback module (`symphonia` decode + `cpal` output). Core design:
  - `streaming_reader.rs`: `StreamingReader` implements Read+Seek over HTTP response body (bytes buffered locally to keep Subsonic "Now Playing" status during playback); `VecSource`/`FileSource` are seekable `MediaSource` wrappers for seek-rebuild and local files.
  - `decoder.rs`: `DecoderHandle` wraps `symphonia` `FormatReader`/`Decoder`, exposing `next_samples()` (interleaved f32), `seek()`, and `sample_rate`/`channels`/duration. Defaults to 48000 Hz if container doesn't report sample rate.
  - `session.rs`: `Session` holds `Mutex<VecDeque<f32>>` ring buffer plus playback state (volume, playing, position). `spawn_decode_feeder()` runs blocking decode loop (ReplayGain, 25ms fade-in, visualizer tap, native seek via `SeekRequest`/`SeekReply`).
  - `output.rs`: cpal device negotiation (`find_compatible_config`, `open_with_config`/`open_default`) and realtime `mix_into` callback, which sums all active sessions' ring buffers (with per-session volume, channel adaptation, and linear-interpolation resampler that degenerates to passthrough when rates match).
  - `mod.rs`: `AudioPlayer` manages session lifecycle (loading → playing → paused/stopped), reopens output stream at each track's native sample rate via `reopen_stream_if_needed()` (deferred during crossfade). Thread-safe via `parking_lot::Mutex`/`RwLock`.
  - Session state: `PlaybackState` enum (Loading, Playing, Paused, Stopped)
  - Sessions stored in `Arc<RwLock<HashMap>>` — playback events fire on the `EventBus` (broadcast) consumed by `queue_manager` and the UI subscription
  - Supports `preload_stream()` and `crossfade_to()` for gapless playback

### iced UI (desktop/src/)

The UI is one iced application. There are no components or routes in the web sense — the whole UI is a state struct, a message enum, an `update`, and a `view`.

- **main.rs**: Entry point. Creates a tokio runtime and runs `iced::application(...)` wiring `App::update`, `App::view`, `App::theme`, `App::subscription`, the bundled font, and window size.
- **app/**: The UI, split by feature into a module directory. `mod.rs` holds `App` (all UI state), the top-level `view()`/`shell()`, `new()`, `theme()`, `subscription()` glue, and toast handling. `message.rs` holds the `Message` enum; `types.rs` holds the small supporting enums (`View`, `Panel`, `SettingsCategory`, …). `update/mod.rs` dispatches every `Message` variant (exhaustive match, one compiler-checked spot) to a `update_<domain>` method defined in a sibling file — `update/{auth,library,playlists,search,settings,equalizer,mix,transport,queue_resume,recap,podcasts,nav}.rs` — each of which spawns backend calls via `iced::Task::perform` whose result returns as another `Message`. `view/mod.rs` routes to one screen method per file — `view/{home,albums,artists,playlists,genres,podcasts,search,recap,settings,mix,panels,player_bar,overlays}.rs` (`home_view`, `album_list_view`, `album_detail_view`, `artists_view`, `playlists_view`, `search_view`, `mix_view`, `recap_view`, `settings_view`, the persistent `player_bar`, the right-dock panels in `panels.rs` (visualizer/queue/lyrics/EQ/audio-stats/similar), and `stack`-based modal overlays in `overlays.rs` (add-to-playlist, account switcher)). `styles.rs`, `format.rs`, `cover.rs`, `viz_colors.rs`, `subscription.rs`, `export.rs` hold shared free-function helpers (button/text styles, time/frequency formatting, cover art caching + the windowed-list `list_window` helper, visualizer color extraction, the event-bus subscription, file-save export).
- **theme.rs**: parses a theme's TOML tokens into `iced::Color`s and builds the `iced::Theme`. Built-ins under `themes/` are embedded at compile time via `include_dir`.
- **icons.rs**: the SVG icon set as raw string constants, recolored per theme through `svg::Style`.
- **viz/**: the visualizer, split into `mod.rs` (canvas::Program entry, bars / oscilloscope / orb modes), `pipeline.rs`/`shader.rs`/`shaders/` (wgpu custom shader pipeline), `particles.rs` (particle system), `config.rs` (particle count and other viz settings), `state.rs` (runtime FFT/particle state).
- **config.rs** (in `backend/`, shared with termium): `~/.config/<id>/config.toml` (server, last theme, volume, saved accounts). Passwords stay in the OS keyring, not here.

### Data Flow

```
desktop/src/app/  (App state, view, update — split into mod.rs/message.rs/types.rs/update/*.rs/view/*.rs)
    │  user action → Message
    ▼  Message::… handled in App::update
iced::Task::perform(backend fn) ──► backend/commands/…
    ├─ OpenSubsonic API calls (subsonic.rs, async reqwest::Client in AppState)
    │    ├─ MD5 auth token generation (auth.rs)
    │    ├─ JSON → typed structs (mappers.rs)
    │    └─ 401 / error 40/41 → EventBus.emit(SessionExpired)
    ├─ Cover art → disk cache (cover_cache.rs) → iced image::Handle
    ├─ Lyrics cascade (subsonic.rs::get_song_lyrics → lyrics.rs)
    └─ Audio playback (audio/)
         └─ StreamingReader (HTTP) → symphonia decode → cpal
    │  result future → Message    │  playback/queue events → EventBus (broadcast)
    ▼                             ▼  Subscription → Message::Backend(BackendEvent)
App::update mutates App state ──► App::view re-renders
```

Termium (`termium/src/`) follows the same backend, different front end: `app.rs` holds `App` state and the `ratatui` render/input loop, `keymap.rs` handles configurable keybindings, `ui/{home,albums,artists,playlists,search,login,player_bar,visualizer}.rs` are the ratatui screen/widget renderers. No `Message`/`Task` abstraction — reads backend state directly and awaits async calls in its event loop.

### Android App

Native Kotlin/Compose app in `android/`, independent of the desktop and termium Rust binaries, sharing OpenSubsonic API contract with them. See [android/CLAUDE.md](android/CLAUDE.md) for architecture, build commands, and conventions.

### Key Design Decisions

1. **Credentials in Keyring, Not Config**: System keyring (libsecret on Linux) stores credentials securely. `config.toml` keeps only the server URL, username and saved-account list — plaintext passwords are never written to disk.

2. **HTTP Streaming with Local Buffering**: `StreamingReader` keeps HTTP connection open during playback so Subsonic/Navidrome sees "Now Playing" status for full track duration, not just download moment.

3. **Synchronous HTTP Blocking**: `reqwest::blocking` used instead of async to simplify integration with `symphonia`'s decoder, which expects synchronous Read+Seek source. Decoding runs on `spawn_blocking` task per session.

4. **UUID-Based Session Tracking**: Each audio playback gets UUID, with own `Session` (ring buffer, volume, position) in `AudioPlayer` map.

5. **Per-Session Volume**: Each `Session` has own volume, applied in shared `cpal` mixing callback (`audio/output.rs::mix_into`) — independent of global output device volume.

### Known Cross-Platform Divergences

Desktop (iced/Rust) and Android (Kotlin/Compose) implement same features independently and have drifted. These are intentional or accepted differences — don't "fix" one to match other without checking with user first:

1. **Release type inference**: `commands/mappers.rs::infer_release_type()` (desktop) returns lowercase `"single"/"ep"/"album"` with title-text and songCount fallback heuristics. `ApiClient.kt::inferReleaseType()` (Android) returns Title Case `"Single"/"EP"/"Album"/"Compilation"/"Live"/"Remix"`, checks `isCompilation` first, no title/songCount fallback — `AlbumListScreen.kt::effectiveType()` does separate songCount-based reclassification on Android side.

2. **Crossfade gain handling**: Desktop's `AudioPlayer::crossfade_to` (Rust, `audio/mod.rs`) doesn't apply ReplayGain during fade ramp. Android's `AudioPlayer.crossfadeTo` multiplies by `gain` during ramp.

3. **Queue/playback model**: Android runs single ExoPlayer instance with full playlist loaded (native gapless/queue management). Desktop runs per-track decode sessions (`audio/session.rs`) with manual preload-and-promote to next session for gapless/crossfade transitions.

## Build & Run

### Prerequisites
- Rust 1.80+ (`rustup default stable`)
- On Linux: ALSA (`libasound2`), `libssl`, `libsecret` (keyring), `libxkbcommon`, plus a Vulkan/OpenGL driver for iced's `wgpu` renderer. Exact package names vary by distro — see `README.md` "System Dependencies".
- On Windows: no extra system dependencies (rustls handles TLS, Windows Credential Manager built-in)
- On macOS: Xcode Command Line Tools (`xcode-select --install`) only — CoreAudio and Keychain are built in, rustls handles TLS
- On FreeBSD: `alsa-lib`, `dbus`, `libxkbcommon` via `pkg install`, plus a Mesa/Vulkan driver. Built and tested only via CI (`cross-platform-actions` qemu VM) — no native FreeBSD runner in GitHub Actions.

### Commands

```bash
# Desktop GUI (debug build, recompiles on .rs changes)
cargo run -p desktop

# Termium TUI
cargo run -p termium

# Optimized release binaries → target/release/{firmium,termium}
cargo build --release

# Android (separate native app in android/, built with Gradle)
cd android && ./gradlew assembleRelease   # release APK
cd android && ./gradlew assembleDebug     # debug APK
cd android && ./gradlew installDebug      # install on connected device
```

There is no Node, npm, or Vite for `desktop`/`termium` — both are Rust crates in the `firmium` Cargo workspace.

### First-Time Setup

1. Clone repo; ensure Rust is installed (`rustup default stable`)
2. On Linux, install system dependencies (see `README.md`)
3. Run `cargo run -p desktop` to launch
4. In-app: enter Subsonic/Navidrome server URL, username, and password
5. Credentials saved to OS keyring; server URL + username stored in `config.toml`

## Development Notes

### Adding a UI Action / Backend Call (desktop)
- Add a variant to the `Message` enum in `desktop/src/app/message.rs`, emit it from the relevant `view/*.rs` method (e.g. `button(...).on_press(Message::Foo)`).
- Add the variant to the matching domain's or-pattern arm in `desktop/src/app/update/mod.rs`, then handle it in that domain's `update_<domain>` method in `desktop/src/app/update/<domain>.rs`. For a backend call, return `Task::perform(commands::module::fn(self.backend.app_state.clone(), …), Message::FooDone)`; the result comes back as another message.
- Async backend fns take owned `Arc<_>` handles (so the future is `'static`); sync fns take `&_`.
- Any struct carried inside a `Message` must derive `Debug` + `Clone` (the enum derives both).

### Adding Audio Playback Features
- Playback logic in `audio/`. New playback methods (e.g., equalizer) belong there — `mod.rs` for public `AudioPlayer` API, `session.rs` for per-track decode/state, `output.rs` for cpal mixing callback.
- All changes must maintain thread-safety (Arc, Mutex, RwLock).
- Sessions identified by UUID; use `AudioPlayer::get_state(session_id)` to query state.
- Crossfade implemented in Rust: `AudioPlayer::crossfade_to()` in `audio/mod.rs` ramps volume between outgoing and incoming sessions. The `queue_manager` task decides *when* to trigger (reacting to `PlaybackPosition` events on the bus); the engine performs the fade.

### UI State Management (desktop)
- All mutable app state lives on the `App` struct in `desktop/src/app/mod.rs` — single source of truth, no stores.
- `update` mutates `App` and returns a `Task`; `view` is a pure function of `App` state, re-run after every message.
- Backend → UI events arrive via the `EventBus` subscription as `Message::Backend(BackendEvent)`.

### Debugging
- `eprintln!()` prints to the terminal running `cargo run`.
- `RUST_BACKTRACE=1 cargo run` for panic backtraces.
- iced renderer issues on Wayland: try `WAYLAND_DISPLAY= cargo run` (forces XWayland) or set `WGPU_BACKEND=gl`.

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

1. `cargo run`
2. Log into local Subsonic/Navidrome instance
3. Test playback, seeking, pause/resume, volume control
4. Test cover art caching (should be cached on second view)
5. Test search and artist fetches

## Packaging & Distribution

- `cargo build --release` builds both workspace binaries: `target/release/firmium` (desktop GUI) and `target/release/termium` (TUI).
- `PKGBUILD` (Arch), `firmium.spec` / `packaging/firmium.spec` (rpm/COPR), and the `.deb` packaging install the `firmium` binary plus `packaging/firmium.desktop` and the icons under `assets/app-icons/`. Termium is not yet packaged separately (build from source via `cargo build --release -p termium`).
- macOS: `release.yml`'s `build-macos` job cross-compiles both `aarch64-apple-darwin` (Apple Silicon) and `x86_64-apple-darwin` (Intel) on a `macos-14` runner, assembles a `Firmium.app` bundle (binary + `assets/app-icons/icon.icns` + `packaging/macos/Info.plist` with the version substituted in), and wraps it into a `.dmg` via `hdiutil`. Builds are unsigned — no Apple Developer account yet, so no codesigning/notarization step.
- FreeBSD: `release.yml`'s `build-freebsd` job runs inside a FreeBSD VM via `cross-platform-actions/action` (no native FreeBSD GitHub Actions runner exists), builds with `cargo build --release`, and packages the binary + `packaging/firmium.desktop` + icons into a `.pkg` via `pkg create`. Not submitted to the official FreeBSD ports tree (out of scope — that's a separate manual review process).
- `scripts/bump-version.sh <ver>` updates `Cargo.toml`, `CLAUDE.md`, `PKGBUILD`, `firmium.spec`, the Android `build.gradle.kts`, and the AUR folders.
- **In-app updater**: not yet ported to the native build. The old Tauri self-updater (signed AppImage / NSIS via `release.yml`) was removed with the web layer; `.deb`/`.rpm`/COPR/AUR users update through their package manager. A native updater is a future task coupled to a redesign of the release pipeline.
- Android: no in-app updater (native Kotlin app) — updates via Play Store or manual APK install.
- Linux `.desktop` launcher: `packaging/firmium.desktop`. App icons under `assets/app-icons/`.

## Documentation

- End-user docs (installing, building from source, usage, custom themes, settings reference) in `firmium-docs` repo, `src/content/*.md`, built with Vite + Svelte, deployed via GitHub Pages
- For which changes require updating which docs page, see AGENTS.md "Keep Docs in Sync"

## Key Files

- `desktop/src/main.rs` — Desktop entry point: runs `iced::application(...)`
- `desktop/src/app/` — `App` state, `Message` enum, `update()`, `view()` — the whole desktop UI, split into `mod.rs`/`message.rs`/`types.rs`/`update/*.rs`/`view/*.rs` (one file per feature domain/screen)
- `desktop/src/theme.rs`, `desktop/src/icons.rs`, `desktop/src/viz/` — theming, icons, visualizer canvas + wgpu shader pipeline
- `termium/src/main.rs` — Termium entry point
- `termium/src/app.rs` — Termium `App` state and ratatui render/input loop
- `termium/src/ui/` — Termium screen/widget renderers
- `backend/init.rs` — `Backend::new()`: builds shared handles, starts `queue_manager`
- `backend/events.rs` — `EventBus` + `BackendEvent` (backend → UI)
- `backend/state.rs` — `AppState` (connection + reqwest client + bus)
- `backend/config.rs` — `~/.config/<id>/config.toml` persistence, shared by desktop and termium
- `backend/audio/` — Audio playback engine (symphonia + cpal)
- `backend/commands/` — OpenSubsonic client, queue, lyrics, covers, downloads, EQ, stats
- `Cargo.toml` — workspace root (`members = ["backend", "desktop", "termium"]`)
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

- **Cover Art Caching**: Disk-based cache under app cache dir (`commands/cover_cache.rs`), up to 200MB budget; LRU (by mtime) eviction when budget exceeded. Persists across restarts; loaded into an `iced::widget::image::Handle` cached in `App` (bounded to `MAX_COVER_HANDLES` decoded handles in memory, oldest evicted) so covers survive restarts without re-downloading.
- **Album Fetching**: Paginated with `maxItems=500` (Subsonic API limit, `backend/commands/subsonic.rs`)
- **Long lists**: albums, artists, and album/playlist track lists use a windowed renderer in `desktop/src/app/cover.rs` (the `list_window` helper: scroll offset + spacers) that only builds the rows currently on screen.
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
