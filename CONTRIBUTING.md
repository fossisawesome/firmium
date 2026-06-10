# Contributing to Firmium

Thanks for your interest in contributing! Firmium is an OpenSubsonic music streaming client for desktop (Linux/Windows) and Android. This guide will help you get started.

## Quick Start

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Node.js 18+
- On Linux: `libssl-dev`, `libxdo-dev`, `libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libsecret-1-dev`
- On Windows: no extra system dependencies needed

### Set Up Your Environment

```bash
# Clone the repo
git clone https://github.com/fossisawesome/firmium
cd firmium

# Install dependencies
npm install

# Start the dev server (builds Rust backend + serves frontend)
npm run dev:app
```

The app will open in a dev window. Log into a Subsonic/Navidrome server to test.

## Project Structure

```
firmium/
├── src/                      # Svelte frontend (single-page app)
│   ├── App.svelte           # Root component & routing
│   ├── components/          # Reusable UI components
│   ├── views/               # Full-page views (one per route)
│   └── lib/                 # Logic modules (stores, API client, audio bridge)
├── src-tauri/               # Rust backend & Tauri config
│   ├── src/
│   │   ├── lib.rs          # All Tauri commands (audio, auth, credentials)
│   │   ├── audio.rs        # rodio playback engine
│   │   └── main.rs         # Thin entry point
│   └── tauri.conf.json     # App metadata & permissions
├── android/                 # Native Kotlin/Compose Android app (separate)
├── themes/                  # TOML theme files
├── CLAUDE.md               # Detailed architecture & conventions
├── agents.md               # Behavioral guidelines for AI-assisted work
└── package.json            # npm scripts & dependencies
```

**Key principle**: Desktop and Android are separate codebases. Desktop is Tauri (Rust + Svelte); Android is native Kotlin/Compose. They share the OpenSubsonic API contract but no code.

## Development Workflow

### Desktop (Tauri + Svelte)

**Frontend Changes**
- All Svelte, JavaScript, CSS changes in `src/` hot-reload instantly via Vite
- Svelte stores in `src/lib/stores.js` are the single source of truth for app state
- No need to restart the dev server

**Rust Backend Changes**
- Changes to `src-tauri/src/*.rs` require a dev server restart
- Run `npm run dev:app` again to rebuild Rust and restart the dev window
- New Tauri commands must be:
  1. Defined in `src-tauri/src/lib.rs` with `#[tauri::command]` macro
  2. Registered in `tauri::generate_handler![]` in `lib.rs`
  3. Added to `src-tauri/capabilities/default.json` for permission scoping
  4. Called from frontend via `tauriInvoke()` in `src/lib/tauri.js`

**Testing Playback**
1. Start dev server: `npm run dev:app`
2. Log into a real Navidrome or Subsonic instance
3. Test playback, seeking, pause/resume, volume, cover art caching
4. Check DevTools (F12) for console errors

### Android

See [android/CLAUDE.md](android/CLAUDE.md) for Android-specific setup and conventions. Key commands:
```bash
npm run android:build     # assembleRelease via Gradle
npm run android:debug     # assembleDebug + install on device
```

## Code Style & Conventions

### General
- **Simplicity first**: No speculative abstractions or premature optimization
- **Surgical changes**: Touch only what's needed; don't refactor unrelated code
- **Comments**: Only when the WHY is non-obvious (a workaround, a constraint, a subtle invariant)
- **Semantic versioning**: Always bump versions correctly

### Rust (src-tauri/src/)
- Use `eprintln!()` for debugging (visible in dev server console)
- Thread-safe playback via `Arc`, `Mutex`, `RwLock` — don't bypass these
- Sessions identified by UUID; query state via `AudioPlayer::get_state(session_id)`
- Audio playback lives in `audio.rs`; keep it modular and testable

### Svelte/JavaScript (src/)
- No TypeScript; type-check responses manually
- Svelte stores in `src/lib/stores.js` — all mutable state goes here
- Components subscribe reactively to stores
- Playback orchestration in `src/lib/playback.js`
- API calls via `Api` class in `src/lib/api.js`

### Themes
- TOML files in `themes/` directory
- Loaded at runtime via `list_themes()` Tauri command
- If you add or modify theme loading, update `src/content/custom-themes.md` in `firmium-docs`

## Testing

Currently no automated test suite. Manual testing required:

1. **Playback**: play, pause, resume, seek, volume control
2. **Cover Art**: verify caching on second view
3. **Search**: artist/album/song search with Wikipedia bio fetch
4. **Auth**: login with different servers, credential storage
5. **Playlists**: create, add tracks, delete (if applicable)
6. **Edge Cases**: network interruption, malformed responses, large libraries

Run `npm run dev:app`, interact with the app, and check DevTools for errors.

## Documentation

When you make changes to settings, themes, or build/packaging commands, update the docs:

- **Settings** → update `src/content/settings.md` in [firmium-docs](https://github.com/fossisawesome/firmium-docs)
- **Themes** → update `src/content/custom-themes.md` in `firmium-docs`
- **Build Commands** → update `src/content/building-from-source.md` in `firmium-docs`
- **Features** → add to appropriate page in `src/content/*.md` in `firmium-docs`

Docs are built with Vite + Svelte (rendered via `src/lib/Markdown.svelte`) and deployed to GitHub Pages. The docs use Firmium's dark theme (CSS variables from `--bg` and `--accent`).

## Submitting Changes

### Before You Start
1. Check if an issue or discussion exists for your idea
2. For major changes, open an issue or discussion first to align on approach
3. Fork the repo and create a feature branch: `git checkout -b feature/your-feature-name`

### Making Your Change
1. Write code following the conventions above
2. Test thoroughly (see Testing section)
3. Run a final check: `npm run dev:app` should start cleanly with no console errors
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

- **Architecture questions**: Read [CLAUDE.md](CLAUDE.md) first, then open a discussion
- **AI-assisted work**: See [agents.md](agents.md) for behavioral guidelines when working with Claude Code
- **Android-specific issues**: See [android/CLAUDE.md](android/CLAUDE.md)
- **Bug reports**: Open an issue with reproduction steps and environment details
- **Feature requests**: Open a discussion or issue describing the use case

## Common Tasks

### Add a New Tauri Command
1. Define the function in `src-tauri/src/lib.rs` with `#[tauri::command]`
2. Add it to `tauri::generate_handler![]` in `lib.rs`
3. Add it to `src-tauri/capabilities/default.json`
4. Call it from frontend via `tauriInvoke()` in `src/lib/tauri.js`

### Add a New Settings Option
1. Add the field to `src/views/Settings.svelte`
2. Store the value in `src/lib/stores.js` (writable store)
3. Persist to localStorage via `SafeStorage` in `src/lib/utils.js`
4. Update `src/content/settings.md` in `firmium-docs`

### Add Audio Playback Feature
1. Extend `audio.rs` (e.g., new method in `AudioPlayer`)
2. Expose a Tauri command in `lib.rs`
3. Wire it in `src/lib/audio-bridge.js` (AudioBridge class)
4. Call it from `src/lib/playback.js` or a component

### Fix an Android-Only Issue
1. See [android/CLAUDE.md](android/CLAUDE.md) for the Android architecture
2. Native Android code is in `android/app/src/main/java/com/fossisawesome/firmium/`
3. Run `npm run android:debug` to build and test on a device
4. Use `adb logcat` to debug

## Release Process

Handled by maintainers:
- Bump version in `package.json` and `src-tauri/tauri.conf.json`
- Update `CHANGELOG.md` with user-facing changes
- Tag release and push to GitHub
- CI builds and publishes installer bundles (deb, rpm, Windows NSIS)

## Code of Conduct

Be respectful. We welcome all contributions that align with the project's goals: a fast, secure, open-source OpenSubsonic client for desktop and mobile.

## License

By contributing, you agree that your changes are licensed under the [GPL-3.0](LICENSE) license, the same as the project.

---

Happy coding! If anything is unclear, open an issue or discussion.
