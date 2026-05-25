# v2.0.0

Re-wrote code base from vanilla JS to Vite - and Svelte

## Changed

- **Frontend Build System** — Migrated from webpack/default bundler to Vite for improved DX and performance
  - Development server now provides instant hot reload for Svelte/CSS/JS changes
  - Build process optimized for faster compilation and smaller bundles

- **App Structure** — Replaced single 1835-line `src/app.js` monolith with specialized components
  - Root component (App.svelte) now handles auth bootstrapping and view routing
  - View routing improved with dedicated component per page
  - UI logic split across reusable components (PlayerBar, Sidebar, LyricsPanel, etc.)

- **Styling Enhancements** — Expanded `src/style.css` with improved responsive design and component styling
  - Enhanced light/dark mode support across all new components

- **Audio Module** — Minor refinements to `src-tauri/src/audio.rs` for improved streaming compatibility

## Technical Notes

- Svelte component hot reloading works out-of-box with Vite during development
- All audio and Tauri IPC logic remains unchanged; refactoring is purely architectural
- Playback state management continues to use centralized Svelte stores
- Component tree now reflects logical feature boundaries (views → components → lib modules)

---

# v1.6.0

## Added

- **Full Windows Platform Support** — Multi-platform build and release infrastructure
  - NSIS installer bundling for Windows (added `nsis` target to `tauri.conf.json`)
  - Windows system diagnostics: GPU detection via PowerShell WMI queries (`Win32_VideoController`)
  - Package manager detection for Windows (`winget`, `chocolatey`, `scoop`)
  - Updated CI/CD pipeline with multi-platform matrix builds (Ubuntu 22.04 + Windows latest)
  - Manual workflow dispatch support for on-demand releases

## Changed

- **Icon Optimization** — Reduced icon file sizes across all formats:
  - PNG files optimized (32x32: 974B → 699B, 128x128: 3.5K → 2.1K, 128x128@2x: 7K → 4.3K)
  - Windows ICO optimized from 86.6K to 1.3K
  - macOS ICNS optimized from 98.5K to 5.3K

- **Release Workflow Refactor** — Complete GitHub Actions pipeline overhaul:
  - Multi-platform matrix strategy (Linux + Windows builds in parallel)
  - Separated platform-specific dependencies (libsecret for Linux keyring support)
  - Improved Rust caching with swatinem/rust-cache
  - Node.js 20 LTS pinned for consistency

## Removed

- **Legacy Windows Icons** — Removed unused Square-format icons (`Square30x30Logo.png`, `Square44x44Logo.png`, etc.) that were not used in the app bundle

---

# v1.5.1

## Fixed

- **Mixed-content blocker fix for HTTP Subsonic servers** — WebView loaded from `tauri://localhost` was silently blocking HTTP requests to plain `http://` targets (e.g. Navidrome on `http://localhost:4533`). Migrated Subsonic API and cover-art fetches from `window.fetch()` to `window.__TAURI__.http.fetch()` routed through reqwest, bypassing WebKit's mixed-content blocker. Added required `http:default` capability scope to `capabilities/default.json`. Tested against Navidrome 0.61.2.

- **Audio bridge fixes** — Fixed issues in audio-bridge.js playback event handling and state synchronization.

## Changed

- Updated Tauri HTTP plugin capabilities to allow wildcard HTTP/HTTPS URLs for flexibility with user-selected server hosts.

---

# v1.5.0

## Added

- **Lyrics Query Normalization**: New `normalizeLrclibQuery()` function in `src/app.js` intelligently normalizes song titles and artist names before querying lrclib.net
  - Removes common version suffixes (Remix, Live, Extended Mix, Acoustic, Instrumental, Remaster, Cover, Edit)
  - Strips featured artist info from track titles
  - Extracts primary artist name from featured/collaboration listings
  - Significantly improves lyrics match rate for remixes, covers, and featuring tracks

- **PKGBUILD Binary Name Flexibility**: Added fallback logic to detect both `firmium-desktop` and `Firmium` binary names in deb bundles, ensuring compatibility with Tauri naming convention changes

---

## Changed

- **PKGBUILD Refactor**: Simplified deb bundle directory detection from hardcoded path to glob pattern (`*_${pkgver}_amd64`) for better resilience to Tauri updates
  - Uses `find` with maxdepth limit for safer directory traversal
  - Improved error messaging with actionable guidance

---

## Fixed

- **PKGBUILD Duplicate Desktop File Issue**: Fixed potential double-installation of desktop launcher entries through conditional logic
  - Now safely handles mismatched naming between Tauri's output and expected AUR paths

- **Binary Installation Fallback**: PKGBUILD now gracefully handles Tauri's variable binary naming without requiring manual updates

---

# v1.4.0

### Synced Lyrics Support 🎙
- **New Lyrics Panel**: Added an animated side panel (slides in from the right) to display song lyrics
- **LRC Format Support**: Integrated LRCLIB API for fetching synced lyrics in LRC format
- **Real-time Sync**: Lyrics automatically highlight the current line during playback, with smooth transitions between lines
- **Multiple Fallbacks**: Supports synced lyrics, plain text lyrics, and instrumental track detection
- **Visual States**: Active lines are highlighted in accent color with larger font; past lines fade out, upcoming lines appear muted

### UI Improvements
- New 🎙 button in the playback controls to toggle the lyrics panel
- Lyrics panel with header, close button, and scrollable content area
- Responsive CSS styling for lyric lines with three states: active, past, and upcoming

## Technical Changes

### Frontend (JavaScript)
- **New Modules**:
  - `parseLrcTimestamp()`: Converts LRC timestamps (mm:ss.xx format) to milliseconds
  - `parseLrc()`: Parses LRC text into structured lyric lines with timestamps
  - `LrclibApi`: External lyrics provider using the LRCLIB public API (no key required)
  - `Lyrics`: State manager for lyrics panel (open/closed, parsed lines, active line index)
- **Playback Integration**: Added `Lyrics.syncToPosition()` calls to the playback position update loop
- **DOM Updates**: Direct DOM manipulation for performance (250ms update interval)

### Styling (CSS)
- New `.lyrics-panel` styles with smooth width transition and animations
- Lyric line styling with three states: `.active` (current), `.past` (played), `.upcoming` (future)
- Special styles for unsynced lyrics and instrumental tracks
- Responsive layout with scrollable content and fixed footer padding

## Performance Considerations
- Lyrics panel slides in/out with CSS transitions for smooth 60fps animations
- LRC parsing happens asynchronously to avoid blocking playback
- Lyric line highlighting uses CSS classes rather than DOM recreation

## Bug Fixes
- Fixed `http` addresses not working as server URL

## Known Limitations
- LRCLIB API rate limiting: ~10 requests per second (should be sufficient for normal usage)
- Lyrics require internet connection to fetch from LRCLIB
- Some tracks may not have lyrics available in LRCLIB

---

# v1.3.0

## New Features

### Logging System
- Added comprehensive app logging via new Rust commands: `write_log()`, `delete_logs()`, `get_log_path()`
- Implemented `AppLogger` module that intercepts all `console.log()`, `console.warn()`, and `console.error()` calls
- Logs are automatically timestamped and persisted to `app-logs.txt` in the app data directory
- Each log entry includes ISO 8601 timestamp, log level (INFO/WARN/ERROR), and message content

### Crossfade Feature
- Added crossfade support for seamless transitions between tracks
- New Playback store methods: `getCrossfadeEnabled()`, `setCrossfadeEnabled()`, `getCrossfadeDuration()`, `setCrossfadeDuration()`
- Configurable crossfade duration (1–12 seconds, default 5 seconds)
- Crossfade settings persisted to localStorage (`firmium_crossfade`, `firmium_crossfade_duration`)
- Crossfade automatically triggers when approaching end of current track (respects repeat-one mode)
- Crossfade skipped if queue is exhausted and repeat-all is disabled

### Version Tracking
- Added `get_app_version()` Rust command to retrieve app version from Cargo.toml at compile time
- Enables runtime version display in settings/about pages

## Improvements

### Documentation Updates
- Updated all Subsonic API references to OpenSubsonic throughout codebase
- Refined API documentation in CLAUDE.md: clarified OpenSubsonic v1.16.1 targeting, added explanation of OpenSubsonic extensions detection
- Added details on server badge detection ("OpenSubsonic" vs "Subsonic" based on capabilities)
- Documented OpenSubsonic field priorities: `displayArtist`, `releaseTypes[]`, `replayGain`, `bpm`, `genres[]`, `isCompilation`
- Added code comments policy to CLAUDE.md

### Rust Backend Improvements
- Refactored comments in `audio.rs` and `main.rs` to reference OpenSubsonic terminology
- Improved certificate validation documentation (self-signed cert instructions)
- New capabilities registered in `default.json`: `write_log`, `delete_logs`, `get_log_path`, `get_app_version`

### Frontend Architecture
- Added `AppLogger` IIFE module for centralized logging
- Console method patching ensures all existing and new logs are persisted
- Improved error handling in logger (graceful fallback if Rust invocation fails)
- Better formatting of logged objects: stringifies JSON when possible, falls back to `String()` conversion

### UI/Styling Enhancements
- Enhanced `style.css` with additional styling improvements
- Updated visual components to better reflect UI refresh

## Bugfixes & Cleanup

- Fixed terminology consistency across codebase (Subsonic → OpenSubsonic)
- Improved comment clarity in audio streaming module

## Notes

- All new Tauri commands are properly scoped in `default.json` for security
- Logging is non-blocking; failures to write logs do not interrupt playback
- Crossfade feature integrates seamlessly with existing repeat modes (repeat-one, repeat-all)
- Version command allows UI to display app version without hardcoding

---

# v1.2.1

### Performance Optimizations
- Added duration caching to reduce redundant Rust-JS roundtrips during playback position tracking
- Optimized position tracking interval from 100ms to 250ms for improved responsiveness without excessive updates
- Refined mutex locking strategy in `StreamingReader::Seek` implementation to reduce lock contention

### Developer Documentation
- Added `MENTAL_MODEL.md` with comprehensive architecture and design philosophy documentation

## Improvements

### Rust Backend
- Cleaned up unused imports in `main.rs` (removed redundant `fs` and `File` imports)
- Removed unused `PlaybackStatus` struct definition
- Improved code clarity with better variable scoping in seek operations

### Frontend
- Refactored volume management to use `DEFAULT_VOLUME` constant for better maintainability
- Removed unused `_loading` state variable in `Store.UI`
- Improved playback state cleanup on track completion (now properly stops position tracking)

## Bug Fixes

- Fixed potential mutex lock deadlock scenarios in streaming seek operations
- Resolved playback position tracking cleanup on end-of-track
- Improved error handling in position update loop

---

# v1.2.0

### Security & Authentication
- Moved MD5 token generation from JavaScript to Rust backend via `generate_auth_params` command
- Enhanced Subsonic protocol compliance with cryptographically random salt generation
- Improved credential handling separation between frontend and backend

### Server Compatibility
- Added OpenSubsonic extension detection via `ServerInfo` store
- Improved compatibility tracking for server capabilities

---

## Bug Fixes

- Resolved audio streaming connection management issues
- Fixed potential thread safety concerns in audio playback integration
- Improved error handling for streaming response failures

---

# v1.0.0

## New Features

### Desktop Application
- **Cross-Platform Build System** - Support for Arch Linux (AUR/pacman), with infrastructure for additional OS's

### Audio Playback System
- **High-Performance Native Audio** - Rodio-based streaming engine replacing Web Audio API
  - Low-latency playback with minimal CPU usage
  - Streaming support (no full file buffering in memory)
  - Multiple audio format support (MP3, OGG, FLAC, WAV, etc.)

### Security & Credentials
- **OS Keyring Integration** - Passwords stored in system keyring (libsecret on Linux, Keychain on macOS)
  - Never stored in localStorage or plaintext
  - Automatic keyring entry management
  - Per-user credential handling

### User Interface
- **Settings Panel**
  - 8 color themes (Firmium, Gruvbox, Tokyo Night, Dracula, Catppuccin Mocha/Macchiato/Frappé/Latte)
  - Wikipedia artist biography integration toggle

---

## Bug Fixes
- Fixed window decoration toggle not persisting across sessions
- Resolved keyring API compatibility (libkeyring 3.0+ support)
- Fixed logout action button routing
- Resolved thread safety issues in audio playback integration
- Fixed stale callbacks in audio bridge cleanup

---

## Technical Improvements
- **State Management** - Centralized, immutable state store for Auth, UI, Playback, and Audio
- **Error Resilience** - Comprehensive error handling in API calls, audio playback, and network requests
- **Performance Optimizations**
  - Concurrent album track fetching with pool management
  - Cover art request deduplication
  - Safe storage wrapper for localStorage with fallback warnings
  - Efficient DOM updates with event delegation

---

## Platform Support
- Debian based, Fedora based, Arch based distros.