# Contributing to Firmium

Thanks for your interest in contributing! Firmium is an OpenSubsonic music streaming client for desktop (Linux, Windows, macOS, FreeBSD) and Android. This guide will help you get started.

## Quick Start

### Prerequisites
- Rust 1.80+ (install via [rustup](https://rustup.rs/))
- Node.js 20+ and npm
- On Linux: ALSA (`libasound2`), `libssl`, `libsecret`, `libxkbcommon`, `webkit2gtk-4.1` and `xdotool` (Tauri webview). Package names vary by distro — see `README.md`.
- On Windows: no extra system dependencies needed
- On macOS: Xcode Command Line Tools (`xcode-select --install`) only
- On FreeBSD: `alsa-lib`, `dbus`, `libxkbcommon` via `pkg install`, plus `webkit2gtk4`

### Set Up Your Environment

```bash
# Clone the repo
git clone https://github.com/fossisawesome/firmium
cd firmium

# Run the app (debug build; starts Vite and the Tauri shell)
npm install
npm run tauri dev
```

The app opens in its own window. Log into a Subsonic/Navidrome server to test. Termium (`cargo run -p termium`) needs no Node.

## Project Structure

```
firmium/
├── src/                      # Svelte 5 UI (render-only)
│   ├── App.svelte           # Shell: layout, startup, drag-drop import
│   ├── views/               # One screen per file
│   ├── components/          # Player bar, panels, rows, modals
│   ├── lib/                 # api.ts (invoke wrappers), stores.ts (state mirrors), ipc.ts, types.ts
│   └── styles/              # SCSS partials (main.scss @uses them)
├── src-tauri/               # Tauri shell: commands.rs (thin), events.rs, lib.rs, cli.rs
├── backend/                 # Rust backend (no UI)
│   ├── core/               # App logic: settings, accounts, library routing, playlists, stats
│   ├── init.rs             # Backend::new(): shared handles + queue_manager
│   ├── events.rs           # EventBus + BackendEvent (backend → UI)
│   ├── audio/              # Playback engine (symphonia decode + cpal output)
│   └── commands/           # OpenSubsonic client, queue, lyrics, covers, stats, …
├── termium/                 # ratatui terminal client
├── android/                 # Native Kotlin/Compose Android app (separate)
├── themes/                  # TOML theme files (embedded in the backend)
├── assets/                  # App icons
├── packaging/               # firmium.desktop, rpm spec
├── CLAUDE.md               # Detailed architecture & conventions
├── AGENTS.md               # Behavioral guidelines for AI-assisted work
└── Cargo.toml              # Workspace: backend, src-tauri, termium
```

**Key principle**: Desktop and Android are separate codebases. Desktop is Tauri (Rust backend, Svelte UI); Android is native Kotlin/Compose. They share the OpenSubsonic API contract but no code.

## Development Workflow

### Desktop (Tauri)

The Svelte UI only renders. Logic lives in Rust, so a feature is mostly a backend change.

- Put the logic in `backend/` (usually a `Core` method in `backend/core/`).
- Add a `#[tauri::command]` forwarder in `src-tauri/src/commands.rs` and register it in `generate_handler!` in `src-tauri/src/lib.rs`.
- Add a typed wrapper in `src/lib/api.ts` and any new shape in `src/lib/types.ts`, then build the screen in `src/views/` or `src/components/`.
- Backend → UI events (playback/queue) reach the UI as Tauri events (`playback-state-changed`, `playback-position`, `queue-state-changed`, `session-expired`); `src/lib/stores.ts` mirrors them.
- Types crossing the boundary need `Serialize` (and `Deserialize` when the UI sends them), camelCase.

**Testing Playback**
1. `npm run tauri dev`
2. Log into a real Navidrome or Subsonic instance
3. Test playback, seeking, pause/resume, volume, cover art caching
4. Watch the terminal for `eprintln!` errors / panics

### Android

See [android/CLAUDE.md](android/CLAUDE.md) for Android-specific setup and conventions. Key commands:
```bash
cd android && ./gradlew assembleRelease   # release APK
cd android && ./gradlew installDebug      # debug build on device
```

## Code Style & Conventions

### General
- **Simplicity first**: No speculative abstractions or premature optimization
- **Surgical changes**: Touch only what's needed; don't refactor unrelated code
- **Comments**: Only when the WHY is non-obvious (a workaround, a constraint, a subtle invariant)

### Backend (backend/)
- Use `eprintln!()` for debugging (visible in the `npm run tauri dev` terminal)
- Thread-safe playback via `Arc`, `Mutex`, `RwLock` — don't bypass these
- Sessions identified by UUID; query state via `AudioPlayer::get_state(session_id)`
- Audio playback lives in `backend/audio/`; keep it modular and testable
- Async backend fns take owned `Arc<_>` handles (so the future is `'static`); sync fns take `&_`

### UI (src/)
- No business logic in TypeScript: call the backend through `src/lib/api.ts`. Stores in `src/lib/stores.ts` mirror backend state plus view-only state (current view, open panel, toasts).
- Style with SCSS in `src/styles/`; use the CSS variables set from the active theme (`--bg`, `--accent`, …).
- API result types come from `backend/commands` (typed structs via `mappers.rs`); keep `src/lib/types.ts` in sync.

### Themes
- TOML files in `themes/` directory (built-ins embedded at compile time via `include_dir`)
- Merged with user themes by `list_themes()` in `backend/commands/themes.rs`; applied as CSS variables by `src/lib/stores.ts`
- If you add or modify theme loading, update `src/content/custom-themes.md` in `firmium-docs`

## Testing

Rust unit tests cover pure logic (`cargo test --workspace`); the UI is checked with `npm run check` and `npm test`. Manual testing is still required:

1. **Playback**: play, pause, resume, seek, volume control
2. **Cover Art**: verify caching on second view
3. **Search**: artist/album/song search with artist bio fetch
4. **Auth**: login with different servers, credential storage
5. **Playlists**: create, add tracks, delete (if applicable)
6. **Edge Cases**: network interruption, malformed responses, large libraries

Run `npm run tauri dev`, interact with the app, and watch the terminal for errors.

## Documentation

Doc-sync rules (which file to update for which kind of change — settings, themes, build commands, new features, README/CONTRIBUTING/API.md/CHANGELOG/android docs) live in one place: [AGENTS.md](AGENTS.md) § "Keep Docs in Sync". Check there rather than this file — this section used to duplicate that table and drifted out of date.

The `firmium-docs` site itself is built with Vite + Svelte (rendered via `src/lib/Markdown.svelte`) and deployed to GitHub Pages, using Firmium's dark theme (CSS variables from `--bg` and `--accent`).

## Submitting Changes

### Before You Start
1. Check if an issue or discussion exists for your idea
2. For major changes, open an issue or discussion first to align on approach
3. Fork the repo and create a feature branch: `git checkout -b feature/your-feature-name`

### Making Your Change
1. Write code following the conventions above
2. Test thoroughly (see Testing section)
3. Run a final check: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `npm run check` should pass and `npm run tauri dev` start cleanly with no panics
4. Commit with a clear message: "Add X feature" or "Fix Y bug"
5. If docs need updating, commit those changes together

### Submitting a PR
1. Push your branch and open a PR against `main`
2. Include a clear description of what changed and why
3. Reference any related issues
4. Wait for feedback — maintainers review for correctness, performance, and alignment with project goals

### PR Expectations
- Code follows project conventions (see Code Style above)
- Changes are focused (one feature or fix per PR, not a grab-bag)
- Tests (or manual test steps) are included
- Docs are updated if user-facing changes were made
- Commit history is clean (use atomic commits)

## Getting Help

- **Architecture questions**: Read docs first, then open a discussion
- **AI-assisted work**: See [AGENTS.md](AGENTS.md) for behavioral guidelines when working with Claude Code
- **Android-specific issues**: See [android/CLAUDE.md](android/CLAUDE.md)
- **Bug reports**: Open an issue with reproduction steps and environment details
- **Feature requests**: Open a discussion or issue describing the use case

## Common Tasks

### Add a New UI Action / Backend Call
1. Add a `Message` variant in `src/app.rs`
2. Emit it from the relevant `view` method (`button(...).on_press(Message::Foo)`)
3. Handle it in `App::update` — for a backend call, `Task::perform(commands::module::fn(...), Message::FooDone)`
4. Handle the result message; update `App` state so `view` re-renders

### Add a New Settings Option
1. Add the field to the `App` struct and a `Message` variant in `src/app.rs`
2. Render the control in `settings_view`
3. Persist it via `config.rs` (`Config` + `save_config`)
4. Update `src/content/settings.md` in `firmium-docs`

### Add Audio Playback Feature
1. Extend `backend/audio/` (e.g., new method in `AudioPlayer`)
2. Add a wrapper in `backend/commands/playback.rs` or `queue.rs` if needed
3. Call it from `App::update` via `Task::perform`
4. React to any resulting `BackendEvent` in `handle_backend`

### Fix an Android-Only Issue
1. See [android/CLAUDE.md](android/CLAUDE.md) for the Android architecture
2. Native Android code is in `android/app/src/main/java/com/fossisawesome/firmium/`
3. Run `cd android && ./gradlew installDebug` to build and test on a device
4. Use `adb logcat` to debug

## Release Process

Handled by maintainers:
- Bump version with `scripts/bump-version.sh <ver>` (updates `Cargo.toml`, `src-tauri/tauri.conf.json`, `package.json`, `CLAUDE.md`, `PKGBUILD`, `firmium.spec`, Android `build.gradle.kts`, AUR folders)
- Update `CHANGELOG.md` with user-facing changes
- Tag release and push to GitHub
- CI builds and publishes installer bundles (deb, rpm, Windows NSIS)

## License

By contributing, you agree that your changes are licensed under the [GPL-3.0](LICENSE) license, the same as the project.

---

Happy coding! If anything is unclear, open an issue or discussion.
