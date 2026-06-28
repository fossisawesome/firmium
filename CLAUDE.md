# CLAUDE.md

**Version**: 7.0.0

Guidance for Claude Code (claude.ai/code) when working in this repository.

## Project Overview

**Firmium** is an OpenSubsonic music streaming client. The desktop app (Linux + Windows) is a native [iced](https://iced.rs) (Rust) application — a single binary, no web view and no JavaScript — providing low-latency audio playback, OS-level credential storage, and integration with OpenSubsonic-compatible servers (e.g. Navidrome). Separate native Android app in `android/`, built with Kotlin + Jetpack Compose.

### Tech Stack

**Desktop (Linux, Windows)**
- **UI**: [iced](https://iced.rs) 0.14 (pure Rust; `canvas` for the visualizer, `svg` for icons, bundled font)
- **Language**: Rust 2021 edition — UI and backend in one process, one crate
- **Audio**: `symphonia` 0.5 (decoding) + `cpal` 0.17 (output device I/O), hand-rolled engine
- **HTTP**: `reqwest` 0.13 for async OpenSubsonic API calls
- **Credentials**: OS keyring via `keyring` crate (libsecret on Linux, Windows Credential Manager on Windows)
- **Packaging**: Linux (deb, rpm, Arch makepkg), Windows (NSIS installer)

**Android**

See [android/CLAUDE.md](android/CLAUDE.md) for Android tech stack and architecture.

## Architecture

### Backend (backend/)

The backend is plain Rust — no UI, no IPC. The iced UI calls these modules directly via `iced::Task::perform` (async fns) or inline (sync fns); the backend pushes playback/queue events back to the UI over an in-process event bus (`backend/events.rs`, a `tokio::sync::broadcast` channel). `src/main.rs` mounts every `backend/*.rs` module at the crate root via `#[path]` attributes, so backend code keeps using `crate::...` paths. Key modules:

- **init.rs**: `Backend::new()` builds the shared handles (event bus, `AudioPlayer`, `AppState`, `QueueState`, optional `PlayHistory`) and starts the `queue_manager` background task. Held by the iced `App` in `src/app.rs`.

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

### iced UI (src/)

The UI is one iced application. There are no components or routes in the web sense — the whole UI is a state struct, a message enum, an `update`, and a `view`.

- **main.rs**: Entry point. Mounts the `backend/*` modules at the crate root (`#[path]`), creates a tokio runtime, and runs `iced::application(...)` wiring `App::update`, `App::view`, `App::theme`, `App::subscription`, the bundled font, and window size.
- **app.rs**: The bulk of the UI (~3k lines). `App` (all UI state), the `Message` enum, `update()` (handles every message, usually by spawning a backend call with `iced::Task::perform` whose result returns as another `Message`), and `view()`. Each screen is a method on `App` returning an `iced::Element` (`home_view`, `album_list_view`, `album_detail_view`, `artists_view`, `playlists_view`, `search_view`, `mix_view`, `recap_view`, `settings_view`), plus the persistent `player_bar`, the right-dock panels (visualizer/queue/lyrics/EQ/audio-stats/similar), and `stack`-based modal overlays (add-to-playlist, account switcher). Long lists (albums) use a windowed renderer that only builds the rows on screen.
- **theme.rs**: parses a theme's TOML tokens into `iced::Color`s and builds the `iced::Theme`. Built-ins under `themes/` are embedded at compile time via `include_dir`.
- **icons.rs**: the SVG icon set as raw string constants, recolored per theme through `svg::Style`.
- **viz.rs**: the visualizer `canvas::Program` (bars / oscilloscope / orb), reading the latest FFT snapshot.
- **config.rs**: `~/.config/<id>/config.toml` (server, last theme, volume, saved accounts). Passwords stay in the OS keyring, not here.

### Data Flow

```
src/app.rs  (App state, view, update)
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

### Android App

Native Kotlin/Compose app in `android/`, independent of the desktop iced build, sharing OpenSubsonic API contract with desktop. See [android/CLAUDE.md](android/CLAUDE.md) for architecture, build commands, and conventions.

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

### Commands

```bash
# Develop (debug build, recompiles on .rs changes)
cargo run

# Optimized release binary → target/release/firmium
cargo build --release

# Android (separate native app in android/, built with Gradle)
cd android && ./gradlew assembleRelease   # release APK
cd android && ./gradlew assembleDebug     # debug APK
cd android && ./gradlew installDebug      # install on connected device
```

There is no Node, npm, or Vite — the desktop app is a single Rust crate.

### First-Time Setup

1. Clone repo; ensure Rust is installed (`rustup default stable`)
2. On Linux, install system dependencies (see `README.md`)
3. Run `cargo run` to launch
4. In-app: enter Subsonic/Navidrome server URL, username, and password
5. Credentials saved to OS keyring; server URL + username stored in `config.toml`

## Development Notes

### Adding a UI Action / Backend Call
- Add a variant to the `Message` enum in `src/app.rs`, emit it from the relevant `view` method (e.g. `button(...).on_press(Message::Foo)`).
- Handle it in `App::update`. For a backend call, return `Task::perform(commands::module::fn(self.backend.app_state.clone(), …), Message::FooDone)`; the result comes back as another message.
- Async backend fns take owned `Arc<_>` handles (so the future is `'static`); sync fns take `&_`.
- Any struct carried inside a `Message` must derive `Debug` + `Clone` (the enum derives both).

### Adding Audio Playback Features
- Playback logic in `audio/`. New playback methods (e.g., equalizer) belong there — `mod.rs` for public `AudioPlayer` API, `session.rs` for per-track decode/state, `output.rs` for cpal mixing callback.
- All changes must maintain thread-safety (Arc, Mutex, RwLock).
- Sessions identified by UUID; use `AudioPlayer::get_state(session_id)` to query state.
- Crossfade implemented in Rust: `AudioPlayer::crossfade_to()` in `audio/mod.rs` ramps volume between outgoing and incoming sessions. The `queue_manager` task decides *when* to trigger (reacting to `PlaybackPosition` events on the bus); the engine performs the fade.

### UI State Management
- All mutable app state lives on the `App` struct in `src/app.rs` — single source of truth, no stores.
- `update` mutates `App` and returns a `Task`; `view` is a pure function of `App` state, re-run after every message.
- Backend → UI events arrive via the `EventBus` subscription as `Message::Backend(BackendEvent)`.

### Debugging
- `eprintln!()` prints to the terminal running `cargo run`.
- `RUST_BACKTRACE=1 cargo run` for panic backtraces.
- iced renderer issues on Wayland: try `WAYLAND_DISPLAY= cargo run` (forces XWayland) or set `WGPU_BACKEND=gl`.

## Testing

No automated tests. Manual testing workflow:
1. `cargo run`
2. Log into local Subsonic/Navidrome instance
3. Test playback, seeking, pause/resume, volume control
4. Test cover art caching (should be cached on second view)
5. Test search and artist fetches

## Packaging & Distribution

- The desktop build is a single binary: `cargo build --release` → `target/release/firmium`.
- `PKGBUILD` (Arch), `firmium.spec` / `packaging/firmium.spec` (rpm/COPR), and the `.deb` packaging install that binary plus `packaging/firmium.desktop` and the icons under `assets/app-icons/`.
- `scripts/bump-version.sh <ver>` updates `Cargo.toml`, `CLAUDE.md`, `PKGBUILD`, `firmium.spec`, the Android `build.gradle.kts`, and the AUR folders.
- **In-app updater**: not yet ported to the native build. The old Tauri self-updater (signed AppImage / NSIS via `release.yml`) was removed with the web layer; `.deb`/`.rpm`/COPR/AUR users update through their package manager. A native updater is a future task coupled to a redesign of the release pipeline.
- Android: no in-app updater (native Kotlin app) — updates via Play Store or manual APK install.
- Linux `.desktop` launcher: `packaging/firmium.desktop`. App icons under `assets/app-icons/`.

## Documentation

- End-user docs (installing, building from source, usage, custom themes, settings reference) in `firmium-docs` repo, `src/content/*.md`, built with Vite + Svelte, deployed via GitHub Pages
- For which changes require updating which docs page, see AGENTS.md "Keep Docs in Sync"

## Key Files

- `src/main.rs` — Entry point: mounts backend modules, runs `iced::application(...)`
- `src/app.rs` — `App` state, `Message` enum, `update()`, `view()` — the whole UI
- `src/theme.rs`, `src/icons.rs`, `src/viz.rs`, `src/config.rs` — theming, icons, visualizer canvas, config persistence
- `backend/init.rs` — `Backend::new()`: builds shared handles, starts `queue_manager`
- `backend/events.rs` — `EventBus` + `BackendEvent` (backend → UI)
- `backend/state.rs` — `AppState` (connection + reqwest client + bus)
- `backend/audio/` — Audio playback engine (symphonia + cpal)
- `backend/commands/` — OpenSubsonic client, queue, lyrics, covers, downloads, EQ, stats
- `Cargo.toml` — single binary crate (iced + backend deps)
- `themes/` — TOML theme files (embedded at compile time)
- `assets/` — bundled font and app icons
- `packaging/` — `firmium.desktop`, rpm spec
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
- **Long lists**: albums, artists, and album/playlist track lists use a windowed renderer in `src/app.rs` (the `list_window` helper: scroll offset + spacers) that only builds the rows currently on screen.
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
