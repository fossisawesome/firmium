# v3.1.3

## Added

- **Rust-driven position events** (`playback-position`): The finish-watcher thread in `audio.rs` now emits `playback-position` every ~300ms, removing the need for JS polling on desktop. `AudioBridge` subscribes via a new `_unlistenPosition` listener and re-emits as a `'position'` event.
- **Event-driven position tracking on desktop**: `startPositionTracking()` in `playback.js` now uses `bridge.on('position', ...)` on desktop instead of a `setInterval` poll — lower IPC overhead and more accurate timing.

## Changed

- **Cover cache switched from entry count to byte budget**: `coverCache.js` now evicts by total blob size (50 MB cap) instead of a fixed 150-entry count. `addCover()` accepts an optional `sizeBytes` argument; `api.js` passes `blob.size` when storing entries.
- **Dependency cleanup** (`Cargo.toml`): Removed `tauri-plugin-shell`, `tauri-plugin-opener`, `tauri-plugin-fs`, and `sysinfo`. Removed corresponding permissions (`fs:default`, `opener:default`, `shell:default`) from `capabilities/default.json`.
- **Tokio feature set trimmed**: `tokio` now uses `rt-multi-thread`, `macros`, `time` only (was `full`).
- **Release profile optimized**: `opt-level` raised from `2` to `3`; `codegen-units = 1` added for maximum LTO effectiveness.
- **`get_machine_info` command removed**: System diagnostics (CPU, GPU, distro, package manager via `sysinfo`/`lspci`) removed from backend and Settings UI — the Settings page no longer shows a "System" row.

## Removed

- `get_machine_info` Tauri command and `SystemInfo` struct from `lib.rs`.
- `sysinfo` crate dependency.
- `tauri-plugin-shell`, `tauri-plugin-opener`, `tauri-plugin-fs` plugin dependencies and their capability permissions.

# v3.1.0

## Added

- **Native Android queue playback** (`set_queue`, `skip_to_next`, `skip_to_previous`, `skip_to_queue_index`): The full play queue is now loaded into a single ExoPlayer playlist on Android so track transitions happen in the native layer even when the WebView is backgrounded and JS timers are frozen.
- **`track-changed` event** (Android): ExoPlayer fires `onMediaItemTransition` and Kotlin emits `track-changed` to JS, which updates stores, scrobbles, lyrics, and Now Playing without any JS timer involvement.
- **Background/foreground recovery** (Android): A `visibilitychange` listener detects when the app returns to the foreground and either emits `finished` (if playback ended while backgrounded) or restarts position tracking.
- **Mobile search overlay** (`MobileSearch.svelte`): Full-screen animated search overlay, replacing in-page search navigation on Android. State (query, results) is persisted between opens via new stores.
- **Mobile settings overlay** (`MobileSettings.svelte`): Full-screen animated settings overlay for Android. Settings are organized into collapsible sections (Appearance, Playback, Services, Account, Debug) instead of a flat list.
- **Mobile page header**: Each page on Android now shows a page title and icon-button shortcuts to Search and Settings.
- **Android back-button handling**: Hardware/gesture back navigates through overlays (Search → Settings → Queue → Player → view history) before the OS exits the app.
- **Artist avatar photos in ArtistList**: Circular artist photos loaded lazily from the server's MusicBrainz/Last.fm integration; falls back to a default avatar SVG.
- **Artist image upgrades on HomeView**: Recent-artists cards fetch server artist images in the background and upgrade from album cover art when available.
- **Mobile play-all circle button**: Album detail and playlist detail views show a floating circular play button on Android, replacing the text "Play All" button.
- **`IconBack`, `IconPlus`, `IconPlayCircle`** icons added to `icons.js`.
- **`mobileSearchOpen`, `mobileSettingsOpen`, `savedSearchQuery`, `savedSearchSongs`, `savedSearchAlbums`** stores added.
- **`navBack`** exported from stores for programmatic back navigation.

## Changed

- **`playAt()` on Android** now calls `bridge.setQueue()` with all stream URLs pre-built in one auth round-trip, then lets ExoPlayer manage subsequent transitions natively. The desktop path is unchanged.
- **Crossfade and gapless preload** guarded with `!isMobile` so they only run on desktop (ExoPlayer handles transitions natively on Android).
- **Now-playing prev/next actions** on Android call `bridge.skipToPrevious()`/`bridge.skipToNext()` instead of `playAt()`.
- **MobilePlayer**: Removed the close chevron button — swipe down is the only dismiss gesture. Queue sheet now opens *on top of* the player (player stays visible underneath). Lyrics toggle moved from secondary controls to tapping the album art. Secondary controls now show Add-to-Playlist (IconPlus) instead of Lyrics.
- **MobilePlayer swipe**: Spring-back animation added for partial drag; `artTouchMoved` flag distinguishes taps from swipes on the cover art.
- **Sidebar on mobile**: Search and Settings items hidden from the tab bar — they are accessed via the header icons instead.
- **Settings view** reorganized into labeled sections (Appearance, Playback, Services, Account, Debug) that collapse on mobile.
- **`AudioSession.trackId`** renamed to `currentTrackId` in Kotlin `AudioPlugin` (mutable; updated on each queue transition).
- **Lyrics panel z-index** raised to 400 on mobile so it sits above MobilePlayer (z-index 300).
- **Mini-player bar progress row** hidden on mobile (`display: none`).
- **Artist row layout** updated: gap-based flex, circular avatar photo, `artist-info` flex-grows to push album count right.
- **Responsive layout tweaks**: Small-phone (`< 375px`) and wider-phone (`> 430px`) breakpoints added for cover art and player sizing.
- **`_unlistenTrackChanged`** lifecycle properly wired and torn down in `AudioBridge.destroy()`.

## Fixed

- Settings.svelte indentation inconsistency for `isAutoLoginEnabled` and `handleLrclib` corrected.
- `onArtTouchEnd` now only fires the lyrics toggle on a genuine tap (no significant movement), preventing accidental lyrics open during swipes.
- Issue #3 (hopefully)

## Removed

- `IconLyrics` removed from MobilePlayer imports (no longer used in secondary controls).
- Close chevron button removed from MobilePlayer top bar.

---

# v3.0.0

## Added

- **Mobile / Android support**: New `MobilePlayer.svelte` full-screen player overlay and `QueueSheet.svelte` queue bottom sheet for touch-first layouts. `platform.js` detects Android vs desktop at runtime.
- **Custom theming engine**: Themes loaded from the `themes/` directory via a new `list_themes` Tauri command. `applyThemeData()` applies CSS custom properties (`--bg`, `--surface`, `--accent`, etc.) directly to `:root`, replacing the old `data-theme` attribute approach. The Settings page now receives the full theme list.
- **SVG icon library** (`src/lib/icons.js`): Centralised icon strings (Play, Pause, Loading, Prev, Next, Repeat, Lyrics, Volume, Music, ChevronDown) replacing inline emoji.
- **`playerControls.js`**: Extracted `togglePlay()`, `prevTrack()`, `nextTrack()`, and `cycleRepeat()` from `PlayerBar.svelte` into a shared module reused by both the desktop bar and the mobile player.
- **`nowPlaying.js`**: New module for OS-level now-playing metadata integration.
- **Audio fade-in on playback start**: 25 ms `fade_in` applied to every new source to eliminate the start-of-playback pop (`audio.rs`).
- **Audio volume ramp on pause/stop**: Volume ramped to 0 over ~20 ms (5 steps × 4 ms) before pausing or stopping, then restored, eliminating audible clicks (`audio.rs`).
- **Debug-mode devtools gate**: DevTools keyboard shortcuts (F12, Ctrl+Shift+I/J/C) are blocked in release builds unless the app is launched with `--debug`. Controlled by a new `is_debug_mode` Tauri command.
- **`has-player` CSS class**: `document.documentElement` gains `has-player` whenever a track is loaded, enabling CSS rules that shift layout when the player bar is visible.
- **`is-mobile-layout` CSS class**: Applied via `matchMedia('(max-width: 640px)')` and the `isMobile` platform flag, giving Android tablets the mobile layout regardless of physical screen width.
- **Android capabilities** (`src-tauri/capabilities/android.json`): Separate Tauri capability file for Android permission scoping.

## Changed

- **`main.rs` refactored into a library crate**: All Tauri commands, data mappers, audio init, and plugin wiring moved to `src-tauri/src/lib.rs`. `main.rs` is now a thin entry-point that calls `app_lib::run()`.
- **`PlaybackState` and `AudioDevice` promoted to crate root**: Defined once in `lib.rs`; `audio.rs` re-exports them via `pub use crate::PlaybackState` and `pub use crate::AudioDevice`.
- **`DeviceSinkBuilder` import guarded** with `#[cfg(not(target_os = "android"))]` to allow compilation on Android where device enumeration differs.
- **`AudioPlayer::new` guarded** with `#[cfg(not(target_os = "android"))]` for the same reason.
- **`PlayerBar.svelte` simplified**: Removed inline control logic (now in `playerControls.js`), replaced emoji icons with SVG strings from `icons.js`, and added a tap handler to open `MobilePlayer` on mobile.
- **Theme application**: `applyTheme(id)` replaced by `applyThemeById(id)` + `applyThemeData(theme)` pair; theme is now applied from a loaded theme object rather than setting a DOM attribute.
- **Login error handling hardened**: `clearAuth()` is now called if the post-login API check fails, preventing a half-authenticated state.
- **`Settings.svelte`**: Receives `loadedThemes` prop and renders the full theme picker from the list returned by `list_themes`.
- **`stores.js`**: Added `mobilePlayerOpen`, `queueSheetOpen` stores for mobile overlay state.
- **`style.css`**: Refactored to use CSS custom properties throughout; `is-mobile-layout` and `has-player` class-driven layout rules added.
- **Package updates**: `package.json` / `package-lock.json` updated; `Cargo.toml` / `Cargo.lock` updated for new crate structure and Android target support.
- **Icons refreshed**: All app icon files regenerated (`32×32`, `128×128`, `128×128@2x`, `icon.icns`, `icon.ico`, `icon.png`).
- **`tauri.conf.json`** updated for v3 bundle identifiers and capability references.
- **`PKGBUILD` / `firmium.spec`** updated to v3.0.0.
- **`README.md`** updated to reflect v3 feature set.

## Removed

- `public/lrclib-logo.png` removed (no longer referenced in the UI).
- Inline `togglePlay`, `prevTrack`, `nextTrack`, `cycleRepeat` functions removed from `PlayerBar.svelte` (replaced by shared `playerControls.js`).
- Old `data-theme` attribute theming removed in favour of CSS variable injection.

## Fixed

- Audible pop at the start of each track (fixed by 25 ms fade-in in `audio.rs`).
- Audible click on pause and stop (fixed by volume ramp in `audio.rs`).
- Half-authenticated app state after a failed login (fixed by calling `clearAuth()` on error in `App.svelte`).

---

# v2.1.0

## Added

- **Gapless playback**: New `preload_stream` Tauri command pre-fetches and decodes the next track in a paused state 30 seconds before the current track ends. When `play()` is called, the preloaded session is promoted instantly — no HTTP fetch or decode delay. Controlled by a new `gaplessEnabled` store (persisted to localStorage) and mutually exclusive with crossfade.
- **ReplayGain normalization**: `play_stream` and `preload_stream` now accept an optional `replayGainDb` parameter. The Rust audio layer applies the gain via `rodio::Source::amplify` so the master volume control remains unaffected.
- **Native crossfade**: `crossfade_to` Tauri command runs volume fade steps inside a Rust async task, eliminating per-step IPC round-trips.
- **Event-driven playback state**: The Rust backend now emits Tauri events (`playback-state-changed`, `playback-finished`) directly, replacing the 750 ms JS `setInterval` polling loop. The `AudioBridge` subscribes to these events via `@tauri-apps/api/event`.
- **Rust-side data mappers**: New `map_albums`, `map_artists`, `map_songs` Tauri commands perform OpenSubsonic JSON → typed struct mapping in Rust (with `serde` camelCase output), including `infer_release_type` logic previously in `api.js`.
- **Last.FM**: Added LastFM toggle to fetch artist biographys.
- **New themes**: Monokai Classic, Monokai Pro, Adwaita, Adwaita Dark, ayu, ayu Light, GitHub Dark, Nordfox, and Synthwave '84 added to `style.css`.
- **`firmium.spec` RPM spec file** added for Fedora/RHEL packaging.
- **Persistent HTTP client**: `AudioPlayer` now holds a reused `reqwest::blocking::Client` (with a `firmium-desktop/<version>` user-agent) to avoid rebuilding a TLS/connection pool per track.

## Changed

- **Home view redesign**: "Recently Played Tracks" section replaced by a deduplicated "Recently Played" albums row (derived from play history via `recentAlbumsFromSongs`). Removed the separate "Recently Played Albums" and "EPs" sections to reduce page length. Artist cards now navigate directly to the artist detail page instead of the artist list.
- **`api.js` mappers removed**: JS `mapAlbum`, `mapArtist`, `mapSong` functions deleted; all callers now invoke the Rust `map_*` Tauri commands. `getRecentAlbums`, `getRandomAlbums`, `getNewestAlbums` refactored to share a `_fetchAlbumList` helper.
- **`AudioBridge` gapless preload API**: `play()` checks for a matching preloaded session and promotes it; `preload()` method added. `startCrossfadeIn` updated to forward `replayGainDb`.
- **`StreamingReader` locking fix**: `fill_to` now releases the buffer lock before network reads to avoid blocking other readers; data is written to a temporary buffer and extended under the lock afterward.
- **`delete_logs` error handling**: No longer returns an error when the log file does not exist (uses `ErrorKind::NotFound` match).
- **`PLAYER_NOT_FOUND` constant**: Extracted magic string in `audio.rs` to a named constant.
- **Lyrics sync gate**: `syncLyricsToPosition` is now only called when the lyrics panel is open (`get(lyricsOpen)`), avoiding unnecessary work during normal playback.
- **`_loadFromStorage` helper**: Deduplicated `_loadRecentSongs` and `_loadPlaylists` storage loaders in `stores.js` into a single generic helper.
- **Scrollbar styling**: Panel scrollbars hidden (`scrollbar-width: none`) and border radii updated to `8px` in several components.
- **`vite.config.js`**: Minor configuration update.

## Removed

- **Wikipedia API**: Was too unreliable, and inaccurate. Replaced by LastFM
- **JS polling loop**: `statusCheckInterval` and related `setInterval`/`clearInterval` logic removed from `AudioBridge` — replaced by Tauri event listeners.
- **JS crossfade interval tracking**: `crossfadingPlayerId` / `crossfadeInterval` fields removed from `AudioBridge`; crossfade now runs natively in Rust.
- **`queue` store import from `HomeView`**: No longer needed after the "Recently Played" refactor.

## Fixed

- **Unhandled promise rejection in cover cache**: Added a `.catch(() => {})` no-op to the shared pending promise in `loadImage` so a failed image load never surfaces as an unhandled rejection.
- **Artist card navigation**: Clicking a recently-played artist now navigates to their detail page; falls back to the artist list only if no `artistId` is available.
- **Track duration display**: `trackDuration` store is now updated immediately when `getDuration()` returns a value during the position-tracking loop, fixing a race where the duration could show `0` for the first polling interval.

---

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