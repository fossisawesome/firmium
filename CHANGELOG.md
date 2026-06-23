# v7.0.0

## Desktop

### Rewritten from Tauri/Svelte to native iced

The desktop app is now a single pure-Rust binary built on [iced](https://iced.rs) 0.14. There is no WebView, no JavaScript, no npm, and no Node.js.

- **UI framework**: iced 0.14 replaces the old Tauri/Svelte layer. The entire UI is implemented in `src/app.rs` as an `App` struct, `Message` enum, `update()`, and `view()` — single source of truth, no stores.
- **Backend event bus**: new `backend/events.rs` (`EventBus`, `BackendEvent`) uses a `tokio::sync::broadcast` channel. Backend → UI events (`PlaybackStateChanged`, `PlaybackPosition`, `PlaybackFinished`, `QueueStateChanged`, `SessionExpired`) arrive via an `iced::Subscription` bridging the broadcast channel into `Message::Backend(BackendEvent)`.
- **Backend init**: `backend/init.rs` (`Backend::new()`) builds shared handles and starts the `queue_manager` background task.
- **Config**: `~/.config/firmium/config.toml` replaces `localStorage`. Server URL, username, theme, volume, and saved accounts persist across restarts.
- **Font**: `LiberationMono` bundled at compile time; no system font required.
- **No Tauri commands**: all backend calls are direct Rust function calls via `iced::Task::perform` — no IPC overhead.
- **Codebase layout**: `src-tauri/` removed. Backend modules now live in `backend/`; iced UI in `src/`.

### New features

- **ListenBrainz scrobbling** (desktop) — Settings now exposes a ListenBrainz user token field. Completed tracks are submitted to ListenBrainz alongside the existing OpenSubsonic scrobble.

- **Recently played tracks on home screen** — the home view now shows a "Recently Played" row of tracks derived from local play history (`backend/db.rs::PlayHistory::recent_plays`), visible immediately on login without a network call. `RecentPlay` struct added to `db.rs`.

- **Save password checkbox** — the login form has a "SAVE PASSWORD" checkbox (checked by default). When unchecked, the password is used for the session but not written to the OS keyring.

- **Crossfade / gapless mutual exclusion enforced in UI** — enabling crossfade now automatically disables gapless, and vice versa. Enabling crossfade while in strict bit-perfect mode downgrades bit-perfect to relaxed. `src/app.rs::Message::SetCrossfadeEnabled`, `Message::SetGapless`.

- **Responsive visualizer bar sizing** — bar width and spacing are now auto-computed from the canvas width when `bar_width` is not explicitly set, so bars fill the panel correctly at any window size. `src/viz/shader.rs`.

- **Theme and download format drop-downs** — themes and download format are now selected via `iced::widget::pick_list` instead of a list of buttons. `ThemeEntry` and `ThemeColors` derive `PartialEq` and `Display` (`backend/commands/themes.rs`).

### Changed

- **Login form redesign** — the setup view now shows a dark semi-transparent backdrop behind a rounded card. Form fields are left-aligned. The logo is replaced by a plain "Firmium" text heading.

- **Account switcher simplified** — the overlay now shows the connected server hostname and a single "DISCONNECT" button. The previous multi-account list UI (add/switch/remove accounts) is removed.

- **Sidebar** — the "Offline" and "Recap" navigation items are removed from the sidebar in this release.

- **Styled widgets** — `text_input`, `slider`, `scrollable`, and `toggler` now use consistent theme-token-based styles (`text_input_style`, `slider_style`, `thin_scroll_style`, `toggler_style` helper fns in `src/app.rs`).

### Build & packaging

- **Windows NSIS installer rebuilt without Tauri** — `packaging/firmium.nsi` is a new standalone NSIS script that packages the native iced binary. Tauri's bundler is no longer involved.

- **CI split into per-platform jobs** — `release.yml` now has separate `build-linux` and `build-windows` jobs (previously a matrix). Linux job packages `.deb` and `.rpm` directly via `dpkg-deb` and `rpmbuild`; Windows job builds the NSIS installer via `makensis`. The `npm-audit` and Svelte/TS check steps are removed from `ci.yml` and `audit.yml`.

---

## Android

### New features

- **Firmium Recap** — full-screen horizontally swipeable recap cards (top tracks, top artists, top albums, top genre, time-of-day breakdown, day-of-week breakdown, biggest discovery, listening streak). Available from the Recap nav item. `RecapScreen.kt`.

- **Local play history via Room** — play events are now recorded to a SQLite database (`firmium_play_history.db`) using Room. Schema: `PlayEntity` with track ID, title, artist, album, cover art ID, genre, BPM, Unix timestamp, and duration played. `FirmiumDatabase.kt`, `PlayDao.kt`, `PlayEntity.kt`, `PlayHistoryRepository.kt`. The DAO mirrors the desktop's `rusqlite` aggregation queries (top tracks/artists/albums, by-hour, by-day-of-week, streak, biggest discovery).

- **Notification shuffle and repeat icons** — the now-playing notification now shows dedicated vector drawables for shuffle, repeat-all, and repeat-one states. `ic_notif_shuffle.xml`, `ic_notif_repeat.xml`, `ic_notif_repeat_one.xml`.

- **`AlbumCard` shared component** — a new `AlbumCard.kt` composable consolidates the album card rendering used across Home, Album List, and Artist Detail screens.

- **Theme import from file** — users can import a `.toml` custom theme via the Android file picker. `ThemeImport.kt`.

- **Share utilities** — `ShareUtils.kt` provides the logic to share Recap card images via the Android share sheet.

---

# v6.6.0

## Android

### New features

- **Multi-server support** — the account dialog now lists previously connected servers. Tapping "Connect" switches to that server immediately using the saved password; entries can be removed individually. Server list persists across restarts via `DataStore`. `AuthManager.kt`, `AuthViewModel.kt`, `AccountDialog.kt`, `AppPreferences.kt`.

- **Star ratings** — tracks show a 1–5 star rating row in the full-screen player. Tapping the current rating clears it. Ratings are submitted to the server via `setRating` (OpenSubsonic API) and reflected immediately in local state. `FullScreenPlayer.kt`, `PlayerViewModel.kt`, `PlaybackController.kt`, `ApiClient.kt`, `Song.kt`.

- **Notification favorite toggle** — the now-playing notification exposes a custom Favorite/Unfavorite action (star icon) that toggles a 5-star rating on the current track and updates the notification icon immediately. `NowPlayingController.kt`, `PlaybackController.kt`.

- **Album list decade + genre filters** — the album list screen surfaces filter chips for decade (e.g. "1990s") and genre derived from the loaded library. Multiple chips can be active; active filters combine (AND between groups). A "Clear" chip appears when any filter is active. `AlbumListScreen.kt`.

- **Album detail BPM filter** — album detail shows BPM range filter chips (All / <80 / 80–120 / 120+) when at least one track in the album has BPM data. Filtering updates the track list and play-all indices in place. `AlbumDetailScreen.kt`.

## Desktop

### New features

- **Multi-server support** — the setup screen lists previously connected servers above the login form. Clicking "Connect" restores the saved password from the OS keyring (keyed per server URL) and reconnects without re-entering credentials. Server list stored in `localStorage` via the `serverList` store. `Setup.svelte`, `stores.ts`, `api.ts` (Keyring now accepts optional `service` for per-server keyring entries), `credentials.rs`.

- **Star ratings** — track rows in album detail and playlist detail show a 1–5 star rating control when connected to a server. Tapping the active star clears the rating. Ratings call `set_rating` (Rust/Tauri → OpenSubsonic `setRating`) and update local track state immediately. `TrackRow.svelte`, `AlbumDetail.svelte`, `PlaylistDetail.svelte`, `subsonic.rs`, `api.ts`, `mappers.rs`, `tauri-commands.ts`.

- **Album list decade + genre filters** — the desktop album list surfaces filter chips for decade and genre derived from the loaded library. Chips combine; a clear button appears when filters are active. `AlbumList.svelte`, `utils.ts` (`extractGenres`, `albumDecade`).

- **Album detail BPM filter** — same BPM range filter chips as Android, shown when BPM data is present. `AlbumDetail.svelte`.

- **Playlist detail BPM filter** — BPM range filter chips on playlist detail when BPM data is available. `PlaylistDetail.svelte`.

---

# v6.5.0

## Android

### New features

- **Wear OS companion app** — a new `:wear` Gradle module delivers a native Wear OS remote control for the phone app. The watch shows the now-playing track (title, artist, cover art) and exposes play/pause, next, previous, and volume controls. Communication uses the Wearable Data Layer: the phone pushes state via `DataClient`; the watch sends transport commands back via `MessageClient`. Phone-side bridge: `wear/WearStateSync.kt` (pushes state), `wear/WearRemoteService.kt` (receives commands), `wear/WearContract.kt` (shared path/key constants). Watch-side: `WearPlaybackClient.kt`, `RemoteScreen.kt`, `MainActivity.kt`.

- **Unified server + local library** — the library screen now merges both sources when a server is connected rather than switching between them exclusively. Albums and artists from the local library that are not present on the server are appended to the server results; server entries win on duplicates to preserve server IDs for scrobbling and lyrics. Album/artist detail routing uses an `albumId`/`artistId` prefix (`local:`) to pick the right source per item. `LibraryViewModel.kt`.

- **Home screen local-only cards** — when a server is connected, local-only albums (downloaded but not on the server) are surfaced in the Recent and Random album rows on the home screen alongside server content. `LibraryViewModel.kt`.

### Bug fixes

- **Repeat-all broken after last track** — the previous `skipToIndex(0)` call at end-of-queue failed because the session had already been released. Now calls `playAt(queue, 0)` to recreate a fresh ExoPlayer instance. `PlaybackController.kt`.

- **Playback finish not firing in background** — replaced the `100ms` polling loop that detected `Player.STATE_ENDED` with a direct `onPlaybackStateChanged` callback. The polling loop ran on `Dispatchers.Main`, which Android throttles when the app is backgrounded, causing repeat and auto-next to silently stop. `AudioPlayer.kt`.

### Visualizer tuning

- **Capture buffer size** — changed from the maximum capture size to the midpoint between min and max. The largest buffer was over-resolving for the orb/bar renders and causing latency. `Visualizers.kt`.

- **Bass sensitivity reduced** — bass energy multiplier dropped from `3.5×` to `1.8×` to reduce over-triggering on bass-heavy tracks. `Visualizers.kt`.

- **Bar count reduced** — `BAR_COUNT` reduced from 40 to 10 for a cleaner look at typical phone screen sizes. `Visualizers.kt`.

- **Bar frequency mapping** — switched from quadratic (`pow(2)`) to `pow(1.5)` band spacing and raised the upper frequency bound from 75% to 90% of the spectrum so the bars cover a wider range more evenly. Gain multiplier reduced from `1.6×` to `1.1×`. `Visualizers.kt`.

## Build

- **Gradle multi-module** — `:wear` module added to `android/settings.gradle.kts`. Android build scripts now target `:app` explicitly (`assembleRelease` → `:app:assembleRelease`) so the root-level tasks don't ambiguously include the wear module.

- **npm wear scripts** — added `wear:build`, `wear:debug`, and `wear:install` npm scripts that delegate to `:wear:assembleRelease`, `:wear:assembleDebug`, and `:wear:installDebug` respectively. `package.json`.

- **Wearable dependency** — `com.google.android.gms:play-services-wearable:19.0.0` added to `:app`'s dependencies for the Wearable Data Layer client. `android/app/build.gradle.kts`.

- **Kotlin JVM target** — moved `jvmTarget` from `android { kotlinOptions {} }` (deprecated) to top-level `kotlin { compilerOptions {} }` in both `:app` and `:wear` modules. `android/app/build.gradle.kts`, `android/wear/build.gradle.kts`.

- **COPR CI hardening** — tag input validated against `^v[0-9]+\.[0-9]+\.[0-9]+$` before use in shell to prevent injection from malicious branch names. Job granted minimal `contents: read` permission. `.github/workflows/copr.yml`.

---

# v6.4.1

## Android

### Bug fixes

- **Web CI test failure** — fixed a failing assertion in `MediaTreeTest.kt` that was breaking the CI test run on the web target.

- **Adaptive launcher icon** — added `mipmap-anydpi-v26/ic_launcher.xml` and `ic_launcher_round.xml` with an adaptive-icon definition (foreground `@mipmap/ic_launcher_foreground`, background `@color/ic_launcher_background`). The launcher background color is set to `#1a1a2e` (dark navy) in `res/values/colors.xml`, so the icon shapes correctly on all Android 8+ launchers.

---

## Misc

- **FEATURES.md** — added a comprehensive user-facing feature reference listing every capability across desktop and Android in one place.

---

# v6.4.0

## Android

### New features

- **Onboarding screen** — first-run pager (5 panels) introduces the app before the server login prompt. Only shown once; completion stored in `AppPreferences.ONBOARDED`. `OnboardingScreen.kt`.
- **Multiple visualizer types** — the audio visualizer now supports three modes: Orb (default), Bars, and Oscilloscope. A single `rememberVisualizerData` composable captures FFT + waveform data from one shared `android.media.audiofx.Visualizer` and feeds all three renderers. Tap the visualizer in the full-screen player to cycle types live; the selected type persists across sessions. `Visualizers.kt`, `MusicOrb.kt`.
- **Visualizer settings** — new "Visualizer" section in Settings with an enable/disable switch and a dropdown to choose the visualizer type. `SettingsScreen.kt`, `AppPreferences.kt`.
- **Add-to-playlist FAB on album art** — a circular accent-color "+" button sits at the lower-right corner of the album art in the full-screen player (both portrait and landscape orientations). Replaces the "Add to playlist" button that was in the secondary controls row. `FullScreenPlayer.kt`.
- **Shuffle and repeat moved to main controls row** — the shuffle and repeat toggles are now inline with the prev/play/next buttons. The secondary controls row is now queue and similar tracks only. `FullScreenPlayer.kt`.
- **Android Auto: A–Z music browser** — the flat "Albums" browse node is replaced by "Music", which shows an A–Z + "#" letter index. Opening a letter filters the album list for that bucket. Albums are cached after the first fetch so subsequent letter taps are instant. `FirmiumMediaBrowserService.kt`, `MediaTree.kt`.
- **Android Auto: playlist shuffle entry** — each playlist in the Android Auto browser now includes a "Shuffle" playable item at the top, letting the user shuffle the whole playlist in one tap. `FirmiumMediaBrowserService.kt`, `MediaTree.kt`.
- **Android Auto: queue list, shuffle, and repeat** — the `MediaSession` queue is published on every track change so Android Auto shows an "Up Next" list and lets the user skip to any item. Shuffle and repeat state are reflected in the session and respond to steering-wheel control commands. `NowPlayingController.kt`, `PlaybackController.kt`.
- **Notification cover art accent color** — the notification and Android Auto UI are tinted to the dominant (or vibrant) color extracted from the cover art via `androidx.palette`. `NowPlayingController.kt`.
- **Playlist mosaic covers** — the music-note placeholder in the playlists list is replaced by a Spotify-style mosaic built from up to 4 distinct track covers (1 full / 2 side-by-side / 3 left-tall + 2 stacked right / 4 grid). `PlaylistMosaic.kt`, `PlaylistsScreen.kt`.
- **Remove track from playlist** — each track row in the playlist detail screen now has a remove button. `PlaylistDetailScreen.kt`.
- **Svalbard theme** — new built-in dark blue theme (bg `#0b1117`, accent `#6cc8e0`). Available from Settings. `Theme.kt`.

### Bug fixes

- **Notification showed 0:00 elapsed when paused** — `NowPlayingController` now tracks `lastPositionMs`/`lastDurationMs` so pausing mid-track keeps the real elapsed time in the notification rather than resetting to 0:00. `NowPlayingController.kt`.
- **Status-bar icon was the full launcher icon** — switched to a dedicated monochrome vector drawable `ic_stat_firmium` so the notification icon meets Android Material guidelines. `NowPlayingController.kt`, `res/drawable/ic_stat_firmium.xml`.
- **Multiple permission dialogs only showed notifications** — three separate single-permission launchers (notifications, `RECORD_AUDIO`, storage) were replaced by one `RequestMultiplePermissions` launcher. Launching several single-permission requests back-to-back caused all but the first dialog to be silently dropped. `MainActivity.kt`.

---

## Desktop

- **Onboarding screen** — new `Onboarding.svelte` first-run pager matching the Android flow: 5 panels, dot indicators, animated hex logo on the welcome panel, and a "Get started" button on the last panel. `src/components/Onboarding.svelte`.
- **Playlist mosaic covers** — new `PlaylistMosaic.svelte` component with the same 1/2/3/4-cover grid layout as Android, used on the playlists view. `src/components/PlaylistMosaic.svelte`.

---

## Themes

- **Svalbard** — new dark blue theme. `themes/svalbard.toml`.

---

## Assets

- Updated app icons across all resolutions and platforms: Android launcher icons (all mipmap densities, round and foreground variants), iOS app icons (all @1x/@2x/@3x sizes), desktop icons (32×32, 64×64, 128×128, 128×128@2x, all Windows Store tile sizes, icns, ico, png), icon source SVG, and site favicon. `src-tauri/icons/`, `android/app/src/main/res/mipmap-*/`, `readme/favicon.svg`.

---

# v6.3.0

## Android Auto support

Firmium now works as a media app on Android Auto. The car's display can browse your library (Home, Albums, Artists, Playlists) and search by voice, and transport controls (play/pause/skip/seek) work from the steering wheel or car UI.

- Added `FirmiumMediaBrowserService` — the `MediaBrowserServiceCompat` that Android Auto binds to. Exposes the full browse tree and publishes the shared `MediaSessionCompat` token so the car's now-playing screen stays in sync.
- Added `MediaTree` — typed media-node IDs and a parser for the browse tree. Album track IDs use `|` as delimiter to avoid collisions with Subsonic/local IDs that contain `:`.
- `NowPlayingController` now exposes `session()` to give the service access to the media session before any track has been played, and advertises `ACTION_PLAY_FROM_MEDIA_ID`, `ACTION_PLAY_FROM_SEARCH`, and `ACTION_PREPARE_FROM_MEDIA_ID` on the idle state so Android Auto can start cold.
- `AndroidManifest.xml` declares the service as exported with a `MediaBrowserService` intent filter and includes the required `automotive_app_desc` metadata.

#### Playback controller refactor (required for Auto)

The queue/transport/scrobble logic was moved out of `PlayerViewModel` (which only exists while the phone app is in the foreground) into a new app-scoped `PlaybackController`.

- `PlaybackController` owns the `PlayerState`, queue management, transport controls, position tracking, and scrobble/reporting — everything that must keep working when only the car is driving playback.
- `PlayerState` moved from `PlayerViewModel` to `PlaybackController` / `audio/` package.
- `PlayerViewModel` is now a thin facade: it delegates to `PlaybackController` and adds the UI-only concerns (lyrics, similar tracks).
- `FirmiumApplication` lazily instantiates `PlaybackController` as `app.playback`; `FirmiumMediaBrowserService` accesses it directly.

#### Instructions

Sideloaded APKs are hidden from Android Auto by default. To use Firmium in the car, unlock **Unknown sources** in Android Auto's developer settings (tap the version number ten times). See the README and the [Android Auto guide](https://docs.firmium.app/android-auto).

---

### ReplayGain toggle (Android + Desktop)

ReplayGain normalization can now be turned on or off at runtime without restarting playback.

**Android**

- `AppPreferences` persists `replay_gain_enabled` (key: `replay_gain_enabled`, default: `true`).
- `AudioPlayer.setReplayGainEnabled()` updates the live volume multiplier on all active sessions immediately.
- Settings > Playback now includes a **ReplayGain** toggle row (`SettingsScreen` / `FirmiumPlaybackPanel`).
- `PlayerState` carries `replayGainEnabled`; `PlayerViewModel.setReplayGainEnabled()` persists and applies the change.

**Desktop**

- `Session` stores the replay-gain factor as an `AtomicU32` (f32 bits), making it live-updatable from outside the decode loop.
- `spawn_decode_feeder` writes the initial factor into `session.replay_gain_factor` and reads it atomically on every decoded chunk, so a toggle takes effect on the next chunk with no restart.
- `AudioPlayer.set_all_replay_gain_factors()` propagates a new factor (or `1.0` to disable) to all active sessions.
- `QueueState` / `QueueStateInner` / `QueueStateSnapshot` carry `replay_gain_enabled` (default: `true`).
- `play_at`, crossfade, and gapless preload all respect `replay_gain_enabled` — passing `None` as the gain factor when disabled.
- New Tauri command `set_replay_gain_enabled` updates state and zeroes out gain on live sessions when disabled. Exposed in `capabilities/default.json`.
- `init_playback_settings` accepts a `replay_gain_enabled` parameter so the setting is restored correctly on startup.
- Frontend: `replayGainEnabled` store in `stores.ts`, persisted to `localStorage` key `firmium_replaygain`. `setReplayGainEnabled()` syncs the store, storage, and Rust.
- Settings > Playback has a **ReplayGain** toggle (desktop `Settings.svelte`).

---

# v6.2.0

## Added

- **Queue management moved to Rust backend** (`src-tauri/src/queue_manager.rs`, `src-tauri/src/queue_state.rs`, `src-tauri/src/commands/queue.rs`): All queue orchestration, playback sequencing, track advance, crossfade triggering, scrobbling, and play queue synchronization are now driven by a Rust background task (`queue_manager.rs`) that listens to playback events (`playback-position`, `playback-finished`) from the audio engine. This eliminates race conditions and timing issues that arose from split orchestration across TypeScript and Rust. The queue is persisted server-side via `savePlayQueue` (debounced 4s) whenever a new track starts or the queue is shuffled. Frontend changes are minimal: `src/lib/playback.ts` now has only position-tracking and lyrics-sync logic; all queue mutation calls have been replaced with Rust command invocations (`set_queue`, `play_queue_index`, `queue_next`, etc.).

- **13 new Rust-side queue commands** (`src-tauri/src/commands/queue.rs`): `init_playback_settings`, `set_queue`, `set_queue_seamless`, `shuffle_and_play`, `play_queue_index`, `queue_next`, `queue_prev`, `toggle_play`, `seek_queue`, `set_queue_volume`, `set_repeat_mode`, `toggle_shuffle`, `set_crossfade_settings`, `set_gapless_enabled`. All queue mutations now go through Rust, ensuring atomicity and correctness. The frontend calls these via `tauriInvoke()` and listens to `queue-state-changed` events for reactive updates.

- **Cover color extraction** (`src-tauri/src/commands/cover_colors.rs`, `src/lib/types/tauri-commands.ts`, `src/lib/playback.ts`): New `extract_cover_colors` and `extract_cover_colors_from_path` Tauri commands extract a 3-color `OrbPalette` (primary, secondary, tertiary) and an optional dominant color from cover art via saturation-weighted pixel bucketing (similar to the old `coverColor.ts` logic, now in Rust). The frontend uses this in the visualizer and other UI elements. The old `src/lib/coverColor.ts` file has been deleted; color extraction now lives entirely in Rust via the new `image` crate dependency.

- **Queue state event sync** (`src/lib/stores.ts::listenToQueueState`, event listener for `queue-state-changed`): Frontend subscribes to `queue-state-changed` events emitted by the Rust queue manager whenever queue, playback settings, or volume change. The `QueueStatePayload` includes the full queue, current index, repeat/shuffle/crossfade/gapless flags, volume, and current player ID. This keeps all Svelte stores (queue, queueIdx, repeatOne, repeatAll, shuffleEnabled, volume, etc.) in sync with Rust ground truth without polling.

- **API documentation** (`API.md`): Comprehensive reference for Navidrome's Subsonic/OpenSubsonic endpoints, authentication, browsing, search, playlists, media streaming, and known Navidrome-specific caveats. Documents the endpoints and parameters Firmium uses, OpenSubsonic extensions advertised by Navidrome, and extended response fields. Also covers Firmium-specific implementation details (auth token generation, streaming reader, cover cache, lyrics cascade, etc.).

## Changed

- **Playback orchestration** (`src/lib/playback.ts`): Stripped down to lyrics and position tracking only. Queue navigation (`playAt`, `crossfadeToNext`, `setQueueSeamless`, `shufflePlay`), track advance, and play queue save logic have been moved to the Rust queue manager. Frontend now calls `set_queue`, `play_queue_index`, `queue_next`, `queue_prev`, `toggle_play` commands instead.

- **Volume and settings changes** (`src/lib/stores.ts`): `setVolume()`, `setCrossfadeEnabled()`, `setCrossfadeDuration()`, `setGaplessEnabled()` now invoke corresponding Rust commands (`set_queue_volume`, `set_crossfade_settings`, `set_gapless_enabled`) in addition to updating localStorage. Changes are immediately reflected server-side and broadcast to other clients via `queue-state-changed`.

- **PlayerBar controls** (`src/components/PlayerBar.svelte`): Volume and seek now invoke `set_queue_volume` and `seek_queue` Rust commands directly instead of going through the `AudioBridge` class. The `AudioBridge` no longer handles volume or seek; it is now read-only (for playback state polling and visualizer event wiring).

- **AudioBridge interface** (`src/lib/audio-bridge.ts`): Removed `setVolume()`, `seek()`, `play()`, `startCrossfadeIn()`, `pause()`, `resume()`, `stop()` methods. The bridge is now a passive listener for `playback-position` and `playback-finished` events; all playback mutations go through the queue manager's Rust commands. `currentPlayerId` property added to allow filtering events when multiple audio sessions might fire (safety-only, normally one active session).

- **Image crate dependency** (`src-tauri/Cargo.toml`): Added `image = { version = "0.25.10", default-features = false, features = ["jpeg", "png", "webp"] }` for cover color extraction.

## Fixed

- **Race conditions in crossfade and track advance** (`src-tauri/src/queue_manager.rs`): The old split architecture (frontend decides when to crossfade, Rust decides when the track is finished) could lead to lost scrobble events or double-scrobbles if timing was unlucky. Now all decisions are centralized in Rust, eliminating race windows.

- **Stale token tracking removed** (`src/lib/stores.ts`, `src/lib/playback.ts`): The old `_playToken` / `bumpToken()` / `getPlayToken()` mechanism (which tried to cancel stale play requests) has been removed as it is no longer needed; the queue manager ensures requests are sequential and atomic.

## Deprecated

- **`AudioBridge.play()`, `pause()`, `resume()`, `stop()`, `seek()`, `setVolume()`, `startCrossfadeIn()`**: These methods are now no-ops or removed. Use the corresponding Rust queue commands instead. The `AudioBridge` class remains as a status-polling and event-wiring utility but no longer drives playback.

---

# v6.1.6

## Added

- **Desktop bit-perfect audio mode** (`src/views/Settings.svelte`, `src/lib/stores.ts`, `src/style.css`, `src-tauri/src/commands/playback.rs`, `src-tauri/src/audio/mod.rs`, `src-tauri/src/lib.rs`): New "Bit-Perfect Audio" selector (Off / Relaxed / Strict) in the desktop Playback settings panel. **Off**: all audio is resampled to the device default rate. **Relaxed**: attempts to reopen the output stream to each track's native sample rate; falls back to resampling on failure. **Strict**: same rate-matching, but also disables crossfade. Mode is persisted to localStorage (`firmium_bit_perfect_mode`) and forwarded to the Rust backend via the new `set_bit_perfect_mode` Tauri command. Enabling Strict disables crossfade; enabling crossfade while in Strict downgrades the mode to Relaxed.

## Changed

- **Android bit-perfect mode removed** (`android/.../audio/AudioPlayer.kt`, `android/.../data/storage/AppPreferences.kt`, `android/.../viewmodel/PlayerViewModel.kt`, `android/.../ui/screens/SettingsScreen.kt`, `android/.../ui/navigation/AppNavGraph.kt`): The Android bit-perfect audio setting (Off / Relaxed / Strict), `bitPerfectMode` preference key, `setBitPerfectMode` ViewModel function, `BitPerfectModeSelector` composable, and all related crossfade/gapless interlock logic have been removed. ExoPlayer now always uses the standard software pipeline; crossfade and gapless settings are no longer locked or dimmed.

## Fixed

- **Android visualizer mutes playback on Samsung devices** (`android/.../ui/components/MusicOrb.kt`, `android/.../MainActivity.kt`): Attaching a `Visualizer` to an ExoPlayer audio session without the `RECORD_AUDIO` permission can cause Samsung's audio stack to silence playback. The orb's `DisposableEffect` now checks for the permission at runtime and skips `Visualizer` creation entirely if it is not granted — the orb continues its idle animation without audio reactivity. `MainActivity` now requests `RECORD_AUDIO` at launch so users who grant it get full visualizer functionality.

---

# v6.1.5

## Fixed

- **Android HTTP cleartext in release builds** (`android/app/build.gradle.kts`): Added `manifestPlaceholders["usesCleartextTraffic"] = "true"` to the release build type so plain-HTTP Subsonic servers remain reachable after Proguard/minification (debug builds already allowed this via the debug manifest).
- **Android API error parsing robustness** (`android/.../data/api/ApiClient.kt`): Malformed or non-JSON responses (e.g. HTML error pages from a misconfigured reverse proxy) now fail with a clear `"Invalid response from <action>"` message instead of an opaque `NullPointerException`. Also guards against a missing `subsonic-response` key and a null `status` field, preventing crashes on partial or unexpected server responses.
- **Android MusicOrb frozen on pause** (`android/.../ui/components/MusicOrb.kt`): The `DisposableEffect` now also keys on `isPlaying`. When playback is paused or the audio session is detached, the Visualizer is torn down and `bass` is reset to `0f`, so the orb returns to its idle (collapsed) state instead of remaining frozen at its last captured radius.
- **Android AlbumDetailScreen null safety** (`android/.../ui/screens/AlbumDetailScreen.kt`): Replaced `pendingSong != null` / `pendingSong!!` double-check with `pendingSong?.let { song -> … }`, eliminating a potential race-condition null dereference when showing the add-to-playlist dialog.
- **Desktop `StreamingReader` seek overflow** (`src-tauri/src/audio/streaming_reader.rs`): `SeekFrom::Current` and `SeekFrom::End` arithmetic now uses `saturating_add` instead of plain `+`, preventing integer overflow panics when the seek offset or buffer length is near `i64::MAX`.

## Added

- **Android plain-HTTP warning in account setup** (`android/.../ui/screens/AccountDialog.kt`): When the server URL uses `http://` and the host is not a recognized local address (localhost, 127.0.0.1, `::1`, RFC 1918 ranges, `.local`), a muted warning line is shown below the Server URL field: _"Connecting over plain HTTP to a non-local server sends your credentials unencrypted. Use HTTPS if possible."_

---

# v6.1.4

## Changed

- **Desktop visualizer FFT resolution** (`src-tauri/src/visualizer.rs`): FFT window doubled (1024 → 2048), ring buffer doubled (4096 → 8192), bar count increased (24 → 32), analysis interval reduced from 50 ms to 16 ms (~60 fps).
- **Desktop visualizer smoothing** (`src-tauri/src/visualizer.rs`): Bass and per-bar values now use exponential attack/decay smoothing (fast rise, slow fall) instead of raw FFT output, eliminating flicker.
- **Desktop bass detection** (`src-tauri/src/visualizer.rs`): Bass now uses the peak magnitude in the 40–250 Hz band (skipping DC and subsonic), replacing the 0–250 Hz average.
- **Desktop bar frequency mapping** (`src-tauri/src/visualizer.rs::compute_bars`): Bars now map 40 Hz–18 kHz log-spaced using the actual sample rate, using peak per band instead of average for more reactive display.
- **Desktop visualizer renderer** (`src/components/VisualizerPanel.svelte`): Rewritten from Canvas 2D to WebGL2 — orb and bars rendered via GLSL fragment shaders (fullscreen-triangle technique, additive blending for glow).
- **Android orb color cycling** (`android/app/src/main/java/com/fossisawesome/firmium/ui/components/MusicOrb.kt`): Orb elements (core glow, rings, wisps, particles) now cycle continuously through all three palette colors via smooth linear interpolation, instead of static palette assignments.
- **Android cover art shadow** (`android/app/src/main/java/com/fossisawesome/firmium/ui/components/FullScreenPlayer.kt`): Drop shadow on cover art is suppressed when the orb is active to prevent visual bleed through orb layers.

## Added

- **Android `RECORD_AUDIO` permission** (`android/app/src/main/AndroidManifest.xml`): Required for the `Visualizer` API attachment used by the orb.

---

# v6.1.3

## Added

- **Cover-art-driven orb visualizer palette** (desktop `src/lib/coverColor.ts::extractOrbPalette`, `src/components/VisualizerPanel.svelte`): The orb visualizer now extracts a 3-color palette (primary, secondary, tertiary) from the current track's cover art using a saturation-weighted bucket algorithm. Works for both local and streamed tracks; falls back to the default purple palette on error or missing art.
- **Richer orb visuals** (`src/components/VisualizerPanel.svelte`): The orb now renders a 4-layer radial glow bloom, a white-hotspot core, 3 staggered expanding rings, 4 orbiting energy wisps, and a 28-particle field. Colors are taken from the cover-art palette. The bar visualizer also now uses palette colors with amplitude-based opacity.
- **Continuous visualizer animation loop** (`src/components/VisualizerPanel.svelte`): Replaced the event-driven `draw()` call with a `requestAnimationFrame` loop (`animate()`), giving smooth animation independent of audio event frequency. Loop starts when the panel opens and cancels on close.
- **Local file preference for streamed tracks** (desktop `src/lib/playback.ts::streamUrlFor`, `src-tauri/src/commands/local_library.rs::find_local_match`, `src/lib/localApi.ts::findLocalMatch`): `streamUrlFor` now checks the local library for a matching downloaded file (by title + album/artist, case-insensitive) before falling back to the server stream URL. Works transparently for all queue sources, not just `local:` tracks.
- **Skip re-download if already cached** (`src-tauri/src/commands/downloads.rs`, `src-tauri/src/commands/local_library.rs::LocalLibraryCache::has_local_match`): `download_track` now exits early if the local library already contains a matching track, preventing redundant downloads.
- **`find_local_match` Tauri command** (`src-tauri/src/commands/local_library.rs`): Returns the absolute file path of a locally cached track matching title + (album or artist). Exposed to the frontend via `src/lib/localApi.ts`.
- **`prewarm_local_library` Tauri command** (`src-tauri/src/commands/local_library.rs`): Triggers an eager local library scan on startup so subsequent lookups are instant. Called fire-and-forget from `App.svelte` on mount.
- **Android bit-perfect audio mode** (Android `audio/AudioPlayer.kt`, `data/storage/AppPreferences.kt`, `viewmodel/PlayerViewModel.kt`, `ui/screens/SettingsScreen.kt`, `ui/navigation/AppNavGraph.kt`): New "Bit-Perfect Audio" setting (Off / Relaxed / Strict) in the Playback panel. **Off**: standard ExoPlayer software pipeline (48 kHz resample). **Relaxed**: hardware audio offload enabled; crossfade fires only when adjacent tracks share the same format (suffix, sample rate, bit depth); gapless disabled. **Strict**: hardware offload required with gapless; hard cuts only. Enabling any bit-perfect mode disables crossfade; enabling crossfade resets bit-perfect to Off. Persisted to `AppPreferences` (`bit_perfect_mode` key).

## Changed

- **Android crossfade / skip respects bit-perfect mode** (`viewmodel/PlayerViewModel.kt::skipToNext`, position-polling loop): In Relaxed bit-perfect mode, crossfade only fires when adjacent tracks have matching audio formats (`audioFormatsMatch` helper checks suffix, sample rate, and bit depth); mismatched formats cause a hard cut. Bit-perfect controls dim and lock incompatible settings (crossfade, gapless) in the UI.

## Fixed

- **Desktop auto-login edge cases** (`src/App.svelte`): Auto-login now correctly handles all credential states: keyring load errors show the account modal; an empty/missing keyring entry shows the modal; `autoLogin` enabled with `savePassword` disabled now shows the modal instead of silently doing nothing.

---

# v6.1.2

## Added

- **Shuffle play** (desktop `src/lib/playback.ts::shufflePlay`, `src/views/PlaylistDetail.svelte`; Android `viewmodel/PlaylistViewModel.kt`): New `shufflePlay(tracks)` function shuffles the input array via Fisher-Yates swap and enables shuffle mode before calling `setQueueSeamless`, so subsequent `nextTrack()` calls pick random queue items. Desktop adds a **Shuffle** button (new `IconShuffle` icon) on the playlist detail page alongside Play All, with the same disabled state when the playlist is empty.
- **Playlist track reordering** (desktop `src/views/PlaylistDetail.svelte::moveTrack`, `src/lib/stores.ts::playlistsStore.moveTrack`, `src/lib/playback.ts`, `src/components/TrackRow.svelte`; Android `viewmodel/PlaylistViewModel.kt::moveTrack`, `data/storage/PlaylistRepository.kt::moveTrack`, `ui/screens/PlaylistDetailScreen.kt`): Tracks can now be moved up/down within a playlist via new move buttons (up/down chevrons, visible on hover for desktop, always visible on mobile) on each track row. Desktop's TrackRow gains `onMove` callback and `isFirst`/`isLast` props to disable move buttons at boundaries. For local-only playlists, the order is persisted locally via `stores.ts::playlistsStore.moveTrack()`. For synced playlists, the new order is pushed to the server by removing all original indices and re-adding song IDs in the new order (OpenSubsonic's `updatePlaylist` has no native "move" operation). Android's `PlaylistDetailScreen` accepts an `onMoveTrack` callback and decorates each track with `canMoveUp`/`canMoveDown` flags.
- **Track row move buttons styling** (desktop `src/style.css`): New `.track-move-btn` class and hover/disabled states; buttons are hidden by default (opacity 0) and shown on `.track-row:hover`. Mobile layout (`html.is-mobile-layout`) always shows move buttons (opacity 1) for touch accessibility.
- **Icon**: New `IconChevronUp` SVG in `src/lib/icons.ts` (inverse polyline of `IconChevronDown`).

## Changed

- **Playlist creation now syncs initial tracks** (desktop `src/views/PlaylistsView.svelte`, Android `data/storage/PlaylistRepository.kt`): When creating a new local playlist with track(s) and immediately syncing to the server, the initial track list is now passed to `Api.updatePlaylist()` (desktop) or `api.updatePlaylist()` (Android) immediately after `createPlaylist()`, instead of deferring to a later sync cycle. Ensures the server playlist contains all intended tracks from the start.
- **OpenSubsonic JSON edge case handling** (`src-tauri/src/commands/subsonic.rs::array_field`, Android `data/api/ApiClient.kt::jsonArray`): Some OpenSubsonic servers return a single object instead of a one-element array when a collection (playlists, playlist entries, etc.) contains exactly one item. `array_field` now checks if the result is an object and wraps it in a vec, and `ApiClient.jsonArray` (new helper) detects single objects via `isJsonArray` and wraps them in a list. This fixes fetching playlists or entries on servers with single items.
- **Shuffle button styling** (desktop `src/style.css`): New `.shuffle-all-btn` class, matching the `.play-all-btn` style (transparent bg, border, accent color on hover).

## Fixed

- **Synced playlists with initial tracks**: Creating a new local playlist with tracks, then syncing it to the server, now correctly transfers those tracks instead of creating an empty server playlist.
- **Single-item playlist/entry fetches**: Fixed OpenSubsonic servers that return a single item as an object instead of an array (e.g., a playlist with one entry).

---

# v6.1.1

## Added

- **Manual playlist sync button** (desktop `src/views/PlaylistsView.svelte`, Android `PlaylistsScreen.kt`): Local-only playlists now show a **Sync** button that immediately calls `Api.createPlaylist()` and links the result via `stores.ts::playlists.setServerId()` (desktop) or `playlistViewModel.syncNow()` (Android), allowing users to push a playlist to the server on demand instead of waiting for the automatic retry loop. The button is disabled while a sync is in progress and hidden for playlists that are already synced or server-only.
- **Server playlist fetch on-demand in add-to-playlist popup** (`src/components/PlaylistMenu.svelte`): When the "add to playlist" context menu opens and server playlists haven't been fetched yet, `Api.getPlaylists()` is now called lazily, allowing server-only playlists (created on other devices but not previously seen locally) to be offered as add targets without blocking the initial menu open.

## Changed

- **Playlist merge displays cloud badges for synced/server-only entries** (`src/components/PlaylistMenu.svelte`, `src/views/PlaylistsView.svelte`): The unified playlist list now shows a small cloud icon badge next to playlist names to distinguish synced playlists (local with matching `serverId`) from server-only entries. `PlaylistMenu.svelte` also uses the merged list instead of local-only, and `PlaylistsView.svelte::unified` list now displays both local and server-only playlists with appropriate visual and interaction cues.
- **Playlist row UI refinements** (desktop `src/style.css`, Android `PlaylistsScreen.kt`): Added `.pl-popup-cloud` styling for the cloud badge icon in the add-to-playlist menu (margin, color, opacity). Android `PlaylistRow` now accepts optional `onSync` callback and displays the sync button alongside the delete button for unsync'd local playlists.
- **Audio backend refactoring** (`src-tauri/src/audio/mod.rs`): Extracted a `OpenedStream` type alias for the `(DecoderHandle, Option<f64>, Arc<Mutex<Vec<u8>>>, u32, u16)` tuple returned by `fetch_and_open` and `open_local_file`, reducing type complexity and improving code readability.
- **Android playlist sync helpers** (`viewmodel/PlaylistViewModel.kt`): Added `syncNow(id)` method to manually trigger `syncCreate()` for a single playlist, and `addTracksToServerOnly()` / `addTracksTo()` helpers to support adding tracks to server-only playlists without a local entry.

---

# v6.1.0

## Added

- **Karaoke-style word-by-word lyrics animation** (`src/lib/playback.ts::computeWordTimings`, `src/components/LyricsPanel.svelte`; Android `viewmodel/LyricsController.kt::computeWordTimings`, `ui/components/LyricsSheet.kt`): for synced lyrics, each line's per-word timing is estimated from its LRC timestamp and the next line's start, proportional to word length. The active line fills word-by-word via a `requestAnimationFrame`/`withFrameNanos` loop interpolating between position-poll updates. New "Word-by-Word Lyrics Animation" toggle (`src/lib/stores.ts::lyricsWordFillEnabled`/`setLyricsWordFillEnabled`, Android `AppPreferences.lyricsWordFillEnabled`) lets users fall back to plain line highlighting.
- **Lyrics panel cover-art glow** (`src/lib/coverColor.ts::extractDominantColor`, `src/lib/playback.ts::updateLyricsGlow`, `src/lib/stores.ts::lyricsGlowColor`; Android `ui/components/ColorExtraction.kt::rememberDominantColor`): the lyrics panel/sheet background is now tinted with the current track's dominant cover color (8x8x8 histogram bucketing on desktop, `androidx.palette.Palette` on Android), darkened to a subtle radial-gradient glow.
- **Server-synced playlists** (desktop `src/lib/stores.ts::mergePlaylists`/`Playlist.createPending`/`createAttempts`/`markCreateAttempt`, `src/views/PlaylistsView.svelte`; Android new `data/model/ServerPlaylist.kt`, `data/api/ApiClient.kt::getPlaylists`/`getPlaylistTracks`/`createPlaylist`/`updatePlaylist`/`deletePlaylist`, `data/storage/PlaylistRepository.kt`, `viewmodel/PlaylistViewModel.kt::mergePlaylists`/`PlaylistListItem`): local playlists are now created, renamed, and have tracks added/removed on the server on a best-effort basis (failures are swallowed and retried up to 3 times via `retryPendingCreates`/the `PlaylistsView` `onMount` retry loop). The "Your Playlists"/"From Server" split is replaced by a single merged list (`mergePlaylists`), with a cloud badge distinguishing synced vs. server-only entries. Android's `AuthManager.buildUrl`/`ApiClient.fetch` gain list-of-pairs overloads to support repeated `songIdToAdd`/`songIndexToRemove` params.
- **App logo icon** (`src/lib/icons.ts::IconLogo`): gold-to-purple gradient hexagon matching `icon-source.svg`/the firmium-site logo, replacing the plain accent-colored `IconHexagon` in `Sidebar.svelte`'s brand area.

## Changed

- **Seek bar and switch touch targets enlarged on Android** (`ui/components/FirmiumSlider.kt::FirmiumSlider`/`FirmiumSeekBar`, `ui/components/FirmiumSwitch.kt`): hit area increased from 20dp to 48dp (sliders) and the switch's clickable bounds grown to a 48dp box around the existing 40x24dp track, for easier touch interaction without changing visual size.
- **Desktop seek bar feels more responsive** (`src/components/PlayerBar.svelte::endSeek`): the displayed position now jumps to the seek target immediately on release instead of waiting for the next position poll, and `isSeeking` stays true for 300ms afterward to ignore a stale in-flight `playback-position` event reflecting the pre-seek position. `durDisplay`/`seekMax`/`seekValue` now fall back through `trackDuration || currentTrack.duration || 0` instead of only checking `trackDuration`.
- **Streaming seek improved for OGG/MP4 and other bisection-seeking formats** (`src-tauri/src/audio/streaming_reader.rs::StreamingReader`): `byte_len()` now returns the stream's `Content-Length` (captured at construction) instead of `None`, letting `symphonia`'s bisection-based seek compute byte offsets from timestamps for forward seeks instead of failing with EOF.
- `decoder.rs::DecoderHandle::try_new` now filters out a reported `n_frames` of `0`, treating it as "unknown duration" rather than a zero-length track.
- `seek_position` (`src-tauri/src/commands/playback.rs`) is now an `async` command that runs the actual seek via `spawn_blocking`, avoiding blocking the Tauri async runtime during seeks.
- **Accessibility pass on desktop player/account/sidebar/playlist-menu controls** (`src/components/PlayerBar.svelte`, `AccountModal.svelte`, `Sidebar.svelte`, `PlaylistMenu.svelte`): player buttons (`prev`/`play`/`next`/`repeat`/`lyrics`/`similar tracks`/`visualizer`) and sidebar nav/account buttons gain `aria-label`/`aria-current`/`aria-hidden` attributes; the now-playing cover art gets a descriptive `alt`. `AccountModal` is now a focus-trapped `role="dialog"` (auto-focuses its close button, cycles Tab/Shift+Tab within the modal). The "add to playlist" popup auto-focuses its first item and supports Arrow Up/Down navigation between items (`handleItemKeydown`).
- **Gapless transition fix** (`src/lib/playback.ts::setQueueSeamless`): the seamless (no-restart) queue swap now only applies when the matched track is also at the target start index (`matchIdx === startIdx`), avoiding incorrectly preserving playback when the resumed queue reorders the current track.
- `data/api/ApiClient.kt`'s LRCLIB requests now send a `Lrclib-Client` header identifying Firmium, per LRCLIB's API guidelines.

## Removed

- **Separate "Your Playlists"/"From Server" sections** (`src/views/PlaylistsView.svelte`): replaced by the unified, merged playlist list described above.

---

# v6.0.0

## Changed

- **Audio backend rewritten from `rodio` to hand-rolled `symphonia` + `cpal`** (`src-tauri/src/audio/`, replacing the old single-file `audio.rs`): new `streaming_reader.rs` (`StreamingReader`/`VecSource`/`FileSource`, Read+Seek over HTTP or local files), `decoder.rs` (`DecoderHandle` wrapping a `symphonia` `FormatReader`/`Decoder`), `session.rs` (`Session` ring buffer + `spawn_decode_feeder` decode loop), `output.rs` (cpal device negotiation and the `mix_into` mixing callback with per-session volume, channel adaptation, and a linear-interpolation resampler), and `mod.rs` (`AudioPlayer` session lifecycle). `Cargo.toml` drops `rodio` for `symphonia` + `cpal` directly.
- **Visualizer tap moved inline**: `visualizer.rs`'s `VisualizerTap` (a `rodio::Source` wrapper) is replaced by `process_chunk()`, called directly from `session::spawn_decode_feeder` for each decoded chunk — same downmix-to-mono/FFT/`firmium:audio-analysis` behavior, just without the `Source` wrapper.
- Minor clippy-driven cleanups in `local_library.rs` (`sort_by_key` instead of `sort_by` with a closure, combined nested `if` conditions, reordered `take`/`cloned`) and `downloads.rs` (`#[allow(clippy::too_many_arguments)]` on `download_track`), with no behavior change.
- `playback.ts::streamUrlFor` no longer needs to be `async` for local tracks; it now returns the `getLocalTrackPath` promise directly via `.then()`.

## Removed

- **Bit-perfect Audio setting** (`set_bit_perfect_enabled`, `stores.ts::bitPerfectEnabled`/`setBitPerfectEnabled`/`activeStreamInfo`, the Settings > Playback toggle, and the "Bit-perfect" suffix in `PlayerBar.svelte`'s track info): the new `cpal` output path always negotiates a device config compatible with the track and resamples via `output.rs`'s linear interpolation, so the explicit reopen-at-native-rate toggle is no longer needed.

---

# v5.5.0

## Added

- **Offline local library** (`src-tauri/src/commands/local_library.rs`, new `lofty` dependency): when not connected to a server, the app reads `~/Music/Firmium` and maps its contents into the same `Album`/`Artist`/`Song` shapes used by the OpenSubsonic API, so the existing UI works unchanged offline. New `src/lib/dataSource.ts` (`dataSource` derived store) picks between `Api` (`src/lib/api.ts`) and the new `src/lib/localApi.ts` (`LocalApi`, backed by `get_local_albums`/`get_local_artists`/`get_local_album_tracks`/`get_local_artist_details`/`search_local`/`get_local_recent_albums`/`get_local_random_albums`/`get_local_newest_albums`/`get_local_genres_list`) based on `isAuthed`. Local ids are `local:<md5>`; cover art is read from embedded tags via `get_local_cover_art` and loaded with `loadLocalImage`. Android adds the equivalent `data/local/LocalLibraryRepository.kt`.
- **Drag-and-drop import** (`import_local_files` in `local_library.rs`): dropping audio files/folders onto the desktop window copies them into `~/Music/Firmium/<AlbumArtist>/<Album>/`, invalidates the local-library cache, and bumps `stores.ts::dataSourceVersion` so local views refetch. `App.svelte` shows a "Drop to add to your library" overlay during the drag and wires `getCurrentWindow().onDragDropEvent`.
- **Downloads** (`src-tauri/src/commands/downloads.rs::download_track`/`download_album`, `src/lib/api.ts::Api.downloadTrack`/`downloadAlbum`/`getLocalAlbumTrackKeys`): downloads tracks/albums from the connected server into `~/Music/Firmium/<AlbumArtist>/<Album>/<TrackNum> - <Title>.<ext>` (same layout as imports), then invalidates the local-library cache. New `DownloadButton.kt` (Android) and a "Downloads" settings category (`Settings.svelte`, `stores.ts::downloadFormat`/`setDownloadFormat`, new `IconDownload`) to choose "Original" (server's source file via `format=raw`) or a transcode target. Android adds `data/download/DownloadManager.kt`.
- **Audio visualizer** (`src-tauri/src/visualizer.rs`, new `rustfft` dependency): `VisualizerTap` taps the rodio decode chain, downmixes to mono, and an FFT-based analysis task emits `firmium:audio-analysis` ({ bass, bars }) events at 50ms intervals while enabled — no overhead when closed. New `VisualizerPanel.svelte` (orb or bars mode, toggled via `stores.ts::visualizerOpen`/`visualizerMode`/`setVisualizerMode`, new `IconWaveform` button in `PlayerBar.svelte`).
- **Cross-device play queue resume** (`savePlayQueue`/`getPlayQueue` OpenSubsonic extensions, `src/lib/api.ts::Api.savePlayQueue`/`getPlayQueue`, `RemotePlayQueue` type): the current queue, track, and position are saved to the server on play/pause and every 30s of playback (debounced via `playback.ts::schedulePlayQueueSave`). On login, `App.svelte::checkRemotePlayQueue` fetches any saved queue and shows the new `ResumeQueuePrompt.svelte`, which resumes playback via `playback.ts::setQueueSeamless` (keeps the current track playing uninterrupted if it's already in the new queue) and seeks to the saved position. Local-only (`local:`) tracks are excluded from saves and aren't resumable.
- **Account modal** (`AccountModal.svelte`, `stores.ts::showAccountModal`/`openAccountModal`/`closeAccountModal`): replaces the old full-screen logout/login flow. Opened via a new account button (`IconUser`) in `Sidebar.svelte`; shows connection status + Disconnect when authed, or the `Setup` login form when not. Also shown automatically on session expiry or auto-login failure. Android adds the equivalent `AccountDialog.kt`.
- **Shared `AlbumRow.svelte`/`TrackRow.svelte` components**: consolidate the duplicated album/track row markup previously repeated across `AlbumList.svelte`, `ArtistDetail.svelte`, `HomeView.svelte`, `SearchView.svelte`, and `PlaylistDetail.svelte`.
- **Responsive sidebar layout** (`App.svelte`, `src/style.css`): the sidebar collapses to an icon-only rail below 900px and a bottom tab bar below 640px (`is-mobile-layout`/`sidebar-collapsed` classes on `<html>`, toggled via `matchMedia`).

## Changed

- Playing audio now resolves its stream URL via `playback.ts::streamUrlFor`, which routes `local:`-prefixed track ids to a `file://` URL (via `getLocalTrackPath`) instead of the OpenSubsonic `stream` endpoint, for both normal playback and crossfade/gapless preload.
- `loadImage` (`src/lib/api.ts`) now routes `local:`-prefixed cover ids to `loadLocalImage` (`localApi.ts`) instead of `OpenSubsonicRouter`/`coverCache`.

---

# v5.4.0

## Added

- **Similar Tracks fallback for servers without `sonicSimilarity`** (`src-tauri/src/commands/subsonic.rs::get_similar_tracks_fallback`, `mappers.rs::SimilarMatch::new`, Android `ApiClient.kt::getSimilarTracksFallback`): combines genre-matched songs (`getSongsByGenre`, similarity 0.55) and tracks from Last.fm-similar artists (`getArtistInfo2` → `getTopSongs`, similarity 0.45), shuffled and capped to `count`. The Similar Tracks button in `PlayerBar.svelte` and the Android player are now always shown (previously hidden without `sonicSimilarity`); `toggleSimilarTracks`/`PlayerViewModel.fetchSimilarTracks` pick `get_sonic_similar_tracks` or the new fallback based on `hasSonicSimilarity`.
- **`Song.artistId`** (`mappers.rs::map_song`, `src/lib/types/tauri-commands.ts`): mapped from `artistId`, used to drive the similar-artists lookup in the fallback above.

## Fixed

- **`openSubsonicExtensions` detection** (`subsonic.rs::subsonic_request`/`validate_connection`): extension entries are objects (`{name, versions}`), not strings — `open_subsonic_extensions` now reads `.name` from each entry. `validate_connection` also calls `getOpenSubsonicExtensions` directly, since regular endpoints like `getAlbumList2` don't include the `openSubsonicExtensions` field.

---

# v5.3.0

## Added

- **OpenSubsonic extension detection** (`src-tauri/src/commands/subsonic.rs::get_open_subsonic_extensions`, Android `ApiClient.kt::openSubsonicExtensions`/`hasExtension`): the extensions advertised by the server are now tracked from every API response, so the app can show or hide extension-gated features. `src/lib/stores.ts`'s `openSubsonicExtensions` is now typed `string[] | null` and reset on logout.
- **Playback reporting** (`playbackReport` extension): desktop (`src-tauri/src/commands/subsonic.rs::report_playback`, `src/lib/api.ts::Api.reportPlayback`) and Android (`ApiClient.kt::reportPlayback`, `PlayerViewModel.kt::reportPlaybackCurrent`) now report `starting`/`playing`/`paused`/`stopped` state and position to the server on play, pause, resume, track change, and track end. No-op if the server hasn't advertised the extension.
- **Sonic similarity / similar tracks** (`sonicSimilarity` extension): new `get_sonic_similar_tracks`/`find_sonic_path` Tauri commands (`src-tauri/src/commands/subsonic.rs`, `mappers.rs::SimilarMatch`/`map_similar_matches`) and Android `ApiClient.getSonicSimilarTracks`/`SimilarMatch`. Desktop adds a `SimilarTracksPanel.svelte` component (new `.similar-tracks-panel` styles in `src/style.css`, full-screen overlay on mobile like the lyrics panel), toggled from a new hexagon button in `PlayerBar.svelte` (`src/lib/stores.ts`: `similarTracksOpen`/`similarTracksTrackId`/`similarTracksResults`/`similarTracksStatus`, `hasSonicSimilarity` derived store). Android adds `PlayerViewModel.fetchSimilarTracks`/`SimilarTracksState`, a new `SimilarTracksSheet.kt` bottom sheet, and a "Similar Tracks" button (`Icons.Default.Hub`) in `FullScreenPlayer.kt`, shown only when the server supports the extension.
- **`LoadingState.svelte`**: shared loading/error/empty-state wrapper component for desktop list views.

## Changed

- **Shared `AlbumRow` component** (Android, new `ui/components/AlbumRow.kt`): consolidates the three near-identical album row implementations previously duplicated in `AlbumListScreen.kt` (`MusicAlbumRow`), `ArtistDetailScreen.kt` (`ArtistAlbumRow`), and `SearchScreen.kt` (`SearchAlbumRow`), with `showArtist`/`coverSize`/`coverRadius` parameters to cover their differences.
- `android/.../ui/navigation/AppNavGraph.kt`: the album/artist detail screen slide transitions are now shared `detailEnterTransition`/`detailExitTransition`/`detailPopEnterTransition`/`detailPopExitTransition` constants instead of duplicated per-route lambdas.


## Fixed

- Issue 20, crash on playing music.

---

# v5.2.0

## Added

- **OpenSubsonic API moved to Rust backend** (`src-tauri/src/commands/subsonic.rs`, new `src-tauri/src/state.rs`): `set_connection`/`validate_connection`/`get_albums`/`get_artists`/`get_album_tracks`/`get_artist_details`/`get_artist_info`/`search`/`get_recent_albums`/`get_random_albums`/`get_newest_albums`/`get_genres_list`/`get_playlists`/`get_playlist_tracks`/`create_playlist`/`update_playlist`/`delete_playlist`/`scrobble`/`get_song_lyrics` are now Tauri commands backed by a shared async `reqwest::Client` in `AppState`. `src/lib/api.ts`'s `Api` object is now a thin set of `tauriInvoke` wrappers; `OpenSubsonicRouter`/`Api.fetch` and the raw response-shape interfaces are removed. `stores.ts`'s `setAuth`/`clearAuth` now call `set_connection` to push credentials into Rust.
- **Disk-based cover art cache** (`src-tauri/src/commands/cover_cache.rs`): cover art is now cached on disk under the app cache dir (200MB budget, mtime-based LRU eviction) and served to the frontend via Tauri's asset protocol (`assetProtocol` enabled in `tauri.conf.json`, scoped to `$APPCACHE/covers/*`, with `protocol-asset` Cargo feature). `src/lib/coverCache.ts` is now a thin wrapper (`getCoverArt`/`clearAll`) around `get_cover_art`/`clear_cover_cache`, converting paths via `convertFileSrc`. The old in-memory blob-URL LRU cache is removed.
- **Lyrics cascade moved to Rust** (`src-tauri/src/commands/lyrics.rs`, `subsonic.rs::get_song_lyrics`): structured OpenSubsonic lyrics → legacy `getLyrics` → LRCLIB fallback now runs entirely in Rust. `src/lib/lyrics.ts` and `src/lib/api.test.ts` are removed; `LyricLine`/`LyricsResult` types moved to `src/lib/types/tauri-commands.ts`.
- **`Song.trackInfo`** (`src-tauri/src/commands/mappers.rs::format_track_info`): the "FLAC · 96 kHz · 24-bit · 1411 kbps" format summary is now computed once on the Rust side and included on every mapped `Song`, instead of being recomputed in `PlayerBar.svelte`. `formatTrackInfo` is removed from `src/lib/utils.ts`.
- **`firmium:session-expired` now a Tauri event**: emitted from `subsonic.rs` on HTTP 401 / OpenSubsonic error codes 40/41, and listened for in `App.svelte` via `@tauri-apps/api/event` instead of a `window` `CustomEvent`.

## Changed

- `Sidebar.svelte`/`Settings.svelte`: `clearAll()` (cover cache wipe) is now async and awaited before clearing list cache/auth on logout and cache wipe.

---

# v5.1.0

## Added

- **Bit-perfect audio output** (`src-tauri/src/audio.rs`): On desktop, the output stream is now reopened to match each track's native sample rate and channel count when possible, avoiding rodio's forced resampling. New `set_bit_perfect_enabled` command and a "Bit-perfect Audio" toggle in Settings > Playback (`src/views/Settings.svelte`, `src/lib/stores.ts`). Reopens are skipped while a crossfade is in flight to avoid silencing the outgoing track.
- **Track format display**: The player bar now shows a "FLAC · 96 kHz · 24-bit · 1411 kbps"-style summary of the current track's format, plus "Bit-perfect" when the output device is running at the track's native rate. Added to desktop (`src/components/PlayerBar.svelte`, `src/lib/utils.ts`: `formatTrackInfo`) and Android (`PlayerBar.kt`, `FullScreenPlayer.kt`, `Song.kt`: `formatTrackInfo()`).
- **Extra song metadata from OpenSubsonic** (`samplingRate`, `bitDepth`, `suffix`, `contentType`): now mapped through on both desktop (`src-tauri/src/commands/mappers.rs`, `src/lib/types/tauri-commands.ts`) and Android (`ApiClient.kt`, `Song.kt`).
- **Release workflow CI gate** (`.github/workflows/release.yml`): A new `check-ci` job verifies the `CI` workflow passed for the tagged commit before `create-release` runs.

## Removed

- **`--debug` flag and in-app logging**: Removed `write_log`/`delete_logs`/`get_log_path`/`is_debug_mode` Tauri commands (`src-tauri/src/commands/logging.rs` → `app_info.rs`), the `app-logs.txt` file/console patching (`src/lib/utils.ts`: `AppLogger`), the Settings > Debug "Log File"/"Delete Logs" rows, and the `DebugMode` state, DevTools auto-open, and Windows console allocation in `lib.rs`/`main.rs`. Devtools shortcuts are now always blocked.

## Changed

- **`AudioPlayer` playback/crossfade methods** (`src-tauri/src/audio.rs`) now take `&Arc<Self>` instead of `&self`, and the output stream/mixer are wrapped in an `RwLock` to support bit-perfect reopening.
- **Android: "Delete Logs" renamed to "Clear Cache"** (`SettingsScreen.kt`, `AppNavGraph.kt`): now clears the app's cache directory instead of referring to removed log files.

---

# v5.0.0

## Added

- **Frontend migrated to TypeScript**: `src/lib/*.js` and `src/main.js` converted to `.ts` (`api.ts`, `audio-bridge.ts`, `coverCache.ts`, `icons.ts`, `lazyLoad.ts`, `lyrics.ts`, `playback.ts`, `playerControls.ts`, `playlistMenu.ts`, `stores.ts`, `tauri.ts`, `main.ts`), with new `tsconfig.json`, `src/vite-env.d.ts`, and generated Tauri command types in `src/lib/types/tauri-commands.ts`.
- **Test suite**: new Vitest setup (`vitest.config.ts`, `vitest.setup.ts`) with unit tests for `api.ts`, `playback.ts`, `playerControls.ts`, and `utils.ts`; new Rust unit tests for the auth and theme commands; new Android `LyricsControllerTest.kt`.
- **Virtualized lists** (`src/lib/VirtualList.svelte`): album, artist, and playlist views now render only visible rows, improving scroll performance on large libraries.
- **In-app auto-updater** (`src/lib/updater.ts`): checks the configured update endpoint and installs/relaunches via `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process`, surfaced under Settings > Debug > Software Update (Windows and Linux AppImage builds).
- **Rust backend split into command modules** (`src-tauri/src/commands/`): `auth.rs` (OpenSubsonic token-auth generation, keeping MD5 out of the JS layer), `credentials.rs` (OS keyring save/get/delete), `themes.rs` (theme discovery/merging, including Android compile-time embedded themes), `playback.rs`, `logging.rs`, and `mappers.rs`. `lib.rs` is now just the app entry point and command registry.
- **Android lyrics sync controller** (`android/.../viewmodel/LyricsController.kt`): owns lyrics fetching, caching, and position-sync state for the currently playing track, mirroring `syncLyricsToPosition` from `playback.ts`, with cancellation-safe fetches and a guard against stale track switches.
- **CodeQL security scanning** (`.github/workflows/codeql.yml`): New CI workflow runs CodeQL analysis across the JS/TS and Java/Kotlin codebases on push, PR, and a weekly schedule, with the Android build run manually via Gradle to satisfy the Java/Kotlin analysis.
- **New CI workflows**: `.github/workflows/ci.yml` (lint/build/test) and `.github/workflows/audit.yml` (dependency audits).
- **Dependabot** (`.github/dependabot.yml`): Automated dependency update PRs for npm, Cargo, Gradle, and GitHub Actions.
- **`.github/FUNDING.yml`**: project funding links.
- **`CONTRIBUTING.md`**: New contributor guide covering setup, branching, and PR conventions.
- **`android/CLAUDE.md`**: Dedicated guidance file for the Android app's tech stack and architecture, referenced from the root `CLAUDE.md`.
- **App icon redesign**: All icon assets (Linux, Windows, macOS, iOS) regenerated from a new `icon-source.svg` at higher resolution.

## Changed

- **`src-tauri/src/audio.rs` rewritten**: significant cleanup of the `rodio`-based playback engine alongside the `commands/` module split.
- **`copr.yml` hardened**: Untrusted GitHub Actions expressions (PR titles, branch names) no longer interpolated directly into shell scripts, closing a script-injection vector. Explicit workflow permissions added.
- **Dependency updates**: `vite`, `svelte`, `concurrently`, `uuid`, `toml` (Rust), `androidx.compose:compose-bom`, `androidx.lifecycle:lifecycle-runtime-compose`, `org.jetbrains.kotlinx:kotlinx-coroutines-android`, `com.squareup.okhttp3:logging-interceptor`, and the Gradle wrapper all bumped to their latest compatible versions. GitHub Actions (`actions/checkout`, `actions/setup-node`, `actions/setup-java`, `actions/github-script`, `android-actions/setup-android`) updated to their latest major versions.
- **`CLAUDE.md` and `agents.md`** restructured: project guidance split between the root `CLAUDE.md` and the new `android/CLAUDE.md`, with `agents.md` updated for autonomy/escalation rules.

---

# v4.0.1

## Fixed

- **Android: FullScreenPlayer drag handle z-order** (`android/app/.../FullScreenPlayer.kt`): Drag handle is now drawn last (above scrollable content) so its `pointerInput` wins over scroll gestures when swiping down from the top. The handle box now has an explicit `height(56.dp)` hit target instead of relying on padding.
- **Android: bottom-navigation pop-back with fallback** (`android/app/.../AppNavGraph.kt`): `popBackStack()` could silently fail when the destination route was not in the back stack (e.g. an artist page opened from a different tab). The nav handler now calls `navController.navigate()` as a fallback when `popBackStack` returns `false`, so tapping a nav item always works.
- **Android: media controls restart playback after queue ends** (`android/app/.../PlayerViewModel.kt`): `MediaSession.onPlay` and the `"stopped"` playback state branch now call `playAt()` to restart the current track instead of doing nothing. Repeat-once (`"one"`) mode also uses `playAt()` after resetting repeat to `"none"`, since the ExoPlayer session is already released at that point and `seek+resume` would be no-ops.
- **Android: media session kept alive on queue end** (`android/app/.../PlayerViewModel.kt`): On playback finish, `nowPlaying` metadata is updated with `updatePlaybackState(false)` rather than cleared, so OS media controls (lock screen, headset buttons) remain functional and can trigger the new restart-on-play behaviour.

---

# v4.0.0

## Removed

- **Mobile/Android Tauri layer entirely** (`src/components/MobilePlayer.svelte`, `MobileSearch.svelte`, `MobileSettings.svelte`, `src/components/QueueSheet.svelte`): All four mobile-specific overlay components deleted. The Android app now lives as a separate native Compose app in `android/` and no longer shares UI code with the Tauri desktop app.
- **`src/lib/nowPlaying.js`**: OS now-playing metadata module removed; it was only wired up on Android via the Kotlin `NowPlayingPlugin`.
- **`src/lib/platform.js`**: Runtime Android/desktop detection helper removed; no longer needed without dual-platform UI code.
- **Android Tauri plugins from `lib.rs`**: `SecureStoragePlugin`, `AudioPlugin`, and `NowPlayingPlugin` registration blocks (all `#[cfg(target_os = "android")]`) removed. `lib.rs` is now desktop-only.
- **Android CI steps** (`.github/workflows/release.yml`): Android NDK installation, Android target cross-compilation, and APK build/publish steps removed from the release workflow.

## Changed

- **`src/lib/audio-bridge.js`** simplified: Mobile-specific methods (`setQueue`, `skipToNext`, `skipToPrevious`, `skipToQueueIndex`, `getQueueIndex`, `startNowPlaying`) and Android event listeners (`track-changed`, unlisten wiring) removed. Bridge is now desktop-only.
- **`src/lib/playback.js`** simplified: `isMobile` guards, `visibilitychange` background-recovery handler, and Android queue-sync logic (`getQueueIndex` call on foreground) removed. Crossfade/gapless preload no longer conditionally disabled for mobile.
- **`src/lib/stores.js`**: `mobilePlayerOpen`, `queueSheetOpen`, `mobileSearchOpen`, `mobileSettingsOpen`, `savedSearchQuery`, `savedSearchSongs`, `savedSearchAlbums`, and `navBack` stores removed.
- **`src/App.svelte`**: Mobile overlay mounts (`MobilePlayer`, `MobileSearch`, `MobileSettings`), back-button handler, and `is-mobile-layout` class logic removed. Root component is now leaner and desktop-focused.
- **`src/components/PlayerBar.svelte`**: Mobile player open tap handler removed; bar is always the primary playback control surface.
- **`src/components/Sidebar.svelte`**: Mobile header icon shortcuts (Search, Settings) removed.
- **`src/views/AlbumDetail.svelte`, `ArtistDetail.svelte`, `PlaylistDetail.svelte`**: Mobile-specific floating play button and header layout variants removed.
- **`package.json`**: Android build/install npm scripts removed.

## Fixed

- **CI release workflow** (`.github/workflows/release.yml`): Removed broken NDK r27 install step and Android-target Rust toolchain setup that were no longer needed after the Android split, reducing CI failure surface.

---

# v3.1.6

## Added

- **`get_current_queue_index` command** (`src-tauri/src/lib.rs`, `AudioPlugin.kt`): New Tauri command (Android only) that returns the native ExoPlayer's current queue index and track ID. Exposed via `AudioBridge.getQueueIndex()` in `audio-bridge.js`. Used to re-sync JS state when the app returns to the foreground after track transitions happened while backgrounded.
- **Background queue sync on visibility change** (`src/lib/playback.js`): When the app regains focus, the visibility handler now calls `getQueueIndex()` and advances `queueIdx`, triggers scrobbling, re-fetches lyrics, and updates the MediaSession notification if ExoPlayer advanced tracks while the WebView was suspended.
- **`has_watcher` guard on audio sessions** (`src-tauri/src/audio.rs`): `PlaybackSession` now tracks whether a finish-watcher async task is already running. Prevents duplicate position/finished events from being spawned after pause-resume cycles.

## Changed

- **Pause no longer sleeps under write lock** (`src-tauri/src/audio.rs`): The previous 20 ms volume-ramp-to-zero used `thread::sleep` inside a `RwLock` write, blocking the tokio executor. Replaced with an instant mute (`set_volume(0.0)`) then restore after the sink pauses — same pop-free result, no blocking.
- **`overlayTransformStyle` fixed as plain `$derived`** (`src/components/MobilePlayer.svelte`): Previously wrapped in `$derived(() => ...)` (a function), which Svelte 5 doesn't track reactively via the function form. Changed to a plain `$derived(expr)` so `dragOffset`, `closing`, and `springing` are properly tracked and the inline style updates on change.
- **Close gesture restricted to top handle area** (`src/components/MobilePlayer.svelte`): Swipe-down-to-close now only activates when the touch originates on the `mp-topbar` element (the drag handle). Touches on the album art, lyrics, or controls no longer accidentally trigger the dismiss animation.
- **Position tracking guards for mid-await track changes** (`src/lib/playback.js`): Added checks after each `await` in the polling interval to bail out if `currentTrack` changed while the IPC call was in flight, preventing stale position/duration values from clobbering the newly loaded track.
- **CI: APK step hardened** (`.github/workflows/release.yml`): Print-and-publish step now uses `set -euo pipefail`, validates that a signed APK and its fingerprint were found before proceeding, shares the APK path via `$GITHUB_OUTPUT`, and writes release notes via a temp file instead of a heredoc to avoid quoting pitfalls. Upload step reuses the path from the fingerprint step instead of re-globbing.

## Fixed

- **`fetchAndShowLyrics` left `activeLyricIdx` stale on track change** (`src/lib/playback.js`): When a lyrics fetch was superseded by a track change, the early-return skipped resetting `activeLyricIdx`, leaving the highlighted lyric line stuck on the previous track. Now resets to `-1` on early exit.

---

# v3.1.5

## Fixed

- **Search section headers misaligned** (`style.css`): "SONGS" and "ALBUMS" headers in the search view were flush against the left edge. Added `padding-left: 16px` to `.section-header` so they align with the rest of the list content.

---

# v3.1.4

## Added

- **COPR publish workflow** (`.github/workflows/copr.yml`): New CI workflow automatically builds and publishes an RPM to Fedora COPR on every GitHub release. Runs in a Fedora 42 container to ensure correct RPM macros.
- **Artist bio toggle on mobile** (`ArtistDetail.svelte`): Bio is now hidden behind a "Show Bio" button on mobile to reduce visual clutter. Tapping reveals/hides the biography section.
- **`packaging/` directory**: New packaging assets added to the repo.

## Changed

- **Mobile search auto-closes on track play** (`MobileSearch.svelte`): Selecting a track from search results now calls `closeSearch()` immediately after starting playback, dismissing the search overlay.
- **Artist page actions layout** (`style.css`): Play-all and bio-toggle buttons are now on the same row via a new `.artist-page-actions` flexbox wrapper.
- **Mobile tracklist header layout** (`style.css`): `.tracklist-header` on mobile changed from `flex-direction: column` to `row` so the header items stay inline.

## Fixed

- **Duplicate `align-items: center` removed** (`style.css`): Conflicting `align-items: center` in the mobile `.tracklist-header` rule cleaned up.

---

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