# CLAUDE.md (Android)

Guidance specific to native Android app in `android/`. See root [CLAUDE.md](../CLAUDE.md) for project-wide conventions, principles, and desktop (iced-rs/Rust) architecture.

## Tech Stack

- **Language**: Kotlin
- **UI**: Jetpack Compose
- **Audio**: Media3/ExoPlayer-based player + foreground `NowPlayingService`
- **HTTP**: OkHttp/Retrofit-style `ApiClient` for OpenSubsonic API calls
- **Credentials**: `SecureStorage` (Android Keystore-backed)
- **Packaging**: Gradle (`assembleDebug` / `assembleRelease`)
- **SDK**: minSdk 26 (Android 8.0), targetSdk/compileSdk 36

## Architecture (android/app/src/main/java/com/fossisawesome/firmium/)

Native Kotlin/Compose app, independent of the desktop iced/Rust build, sharing OpenSubsonic API contract with desktop. Second Gradle module, `wear/`, is Wear OS companion (remote control for phone playback over Wearable Data Layer); shares phone's `applicationId` but no code. Android TV support (`ui/tv/`, `TvMainActivity`) lives in this same module, not a separate one — it's a second UI layer over the same ViewModels/data/audio stack, not a separate app.

- **MainActivity.kt / FirmiumApplication.kt**: App entry points
- **TvMainActivity.kt**: Android TV entry point (separate `LEANBACK_LAUNCHER` activity), reusing the same ViewModels/data/audio layer as `MainActivity` — no touch-specific UI code
- **viewmodel/**: `AuthViewModel`, `LibraryViewModel`, `PlayerViewModel`, `PlaylistViewModel`, `SearchViewModel` — state holders feeding Compose UI
- **audio/**: `AudioPlayer`, `NowPlayingService` (foreground media service), `NowPlayingController`
- **wear/**: Wear OS companion bridge — `WearRemoteService` (receives watch transport commands), `WearStateSync` (pushes now-playing state + art to watch), `WearContract` (Data Layer paths/keys, mirrored in `:wear` module)
- **ui/tv/**: Android TV screens (androidx.tv Compose, D-pad-focusable) — `TvNavGraph` (own `NavHost`, separate from phone's `AppNavGraph`), `TvHomeScreen`/`TvAlbumListScreen`/`TvArtistListScreen`/`TvAlbumDetailScreen`/`TvArtistDetailScreen`/`TvPlaylistListScreen`/`TvPlaylistDetailScreen`/`TvSearchScreen`/`TvNowPlayingScreen`/`TvLoginScreen`/`TvMixScreen`/`TvSettingsScreen`/`TvEqualizerScreen`/`TvRecapScreen`, `TvComponents.kt` (`TvTile`/`TvActionButton` focus-aware wrappers over `androidx.tv.material3.Card`/`Button`, plus `TvToggleRow`/`TvCycleRow`/`TvStepperRow` D-pad-friendly stand-ins for switches/dropdowns/drag sliders). `TvNowPlayingScreen` hosts queue/lyrics/similar-tracks as switchable side panels instead of separate sheets. `TvEqualizerScreen` and settings reads (`AppPreferences`) are accessed directly, same as phone's `FirmiumEqualizerPanel` — not routed through a ViewModel. TV-only scope trims vs. phone: no word-fill lyrics animation, no parametric EQ editing/`.toml` import, no Last.fm config or cache/reset actions (all text-entry-heavy or maintenance actions that don't suit a remote).
- **data/api/**: `ApiClient`, `AuthManager` — OpenSubsonic REST client and auth/token handling
- **data/model/**: `Artist`, `Album`, `Song`, `Playlist` data classes
- **data/storage/**: `AppPreferences`, `PlaylistRepository`, `SecureStorage` (Keystore-backed credential storage)
- **ui/components/**: Compose UI — `PlayerBar`, `FullScreenPlayer`, `QueueSheet`, `LyricsSheet`, `AddToPlaylistDialog`, `FirmiumUi`/`FirmiumHeader`/`FirmiumTextField`/`FirmiumSwitch`/`FirmiumSlider`/`FirmiumBottomSheet`, `CoverImage`
- **ui/theme/**: `Theme.kt` — Compose theming

## Build Commands

Run from repo root:

```bash
npm run android:build   # :app assembleRelease via Gradle
npm run android:debug   # :app assembleDebug via Gradle
npm run android:install # :app installDebug via adb

npm run wear:build      # :wear assembleRelease (Wear OS companion)
npm run wear:debug      # :wear assembleDebug
npm run wear:install    # :wear installDebug (installs on connected watch/emulator)
```

## Development Notes

- **Foreground service**: `NowPlayingService` requires `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_MEDIA_PLAYBACK` permissions (declared in `AndroidManifest.xml`). Must be started via `startForegroundService()` before calling `startForeground()` within 5 seconds or Android kills app.
- **Compose recomposition**: ViewModels are state source of truth. Never hoist mutable state into composables also updated from `viewModelScope` — causes recomposition conflicts. Use `collectAsState()` from ViewModel's `StateFlow`.
- **Credential storage**: `SecureStorage` uses Android Keystore — not `SharedPreferences`. Don't store passwords or tokens in `AppPreferences` (which uses `DataStore`/plain prefs).

## Networking Notes

- App does **not** use `src/lib/*.js` — all networking goes through `ApiClient.kt` (OkHttp).
- `getLyrics`/`fetchLyricsForCurrent` and similar coroutine-based calls run on `viewModelScope.launch` (main dispatcher). Any blocking OkHttp `.execute()` call must be wrapped in `withContext(Dispatchers.IO)`, or throws `NetworkOnMainThreadException` — can be silently swallowed by surrounding catch blocks.

## Key Files

- `android/app/src/main/java/com/fossisawesome/firmium/MainActivity.kt` — Android app entry point
- `android/app/src/main/java/com/fossisawesome/firmium/data/api/ApiClient.kt` — OpenSubsonic API client
- `android/app/src/main/java/com/fossisawesome/firmium/audio/AudioPlayer.kt` — Audio playback engine
- `android/app/src/main/java/com/fossisawesome/firmium/audio/NowPlayingService.kt` — Foreground media service
