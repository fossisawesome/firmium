# Changelog

## v6.3.0

### Android Auto support

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
