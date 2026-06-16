# CLAUDE.md (Android)

Guidance specific to the native Android app in `android/`. See the root [CLAUDE.md](../CLAUDE.md) for project-wide conventions, principles, and the desktop (Tauri/Svelte) architecture.

## Tech Stack

- **Language**: Kotlin
- **UI**: Jetpack Compose
- **Audio**: Media3/ExoPlayer-based player + foreground `NowPlayingService`
- **HTTP**: OkHttp/Retrofit-style `ApiClient` for OpenSubsonic API calls
- **Credentials**: `SecureStorage` (Android Keystore-backed)
- **Packaging**: Gradle (`assembleDebug` / `assembleRelease`)
- **SDK**: minSdk 26 (Android 8.0), targetSdk/compileSdk 36

## Architecture (android/app/src/main/java/com/fossisawesome/firmium/)

Native Kotlin/Compose app, independent of the Tauri build, sharing the OpenSubsonic API contract with the desktop app.

- **MainActivity.kt / FirmiumApplication.kt**: App entry points
- **viewmodel/**: `AuthViewModel`, `LibraryViewModel`, `PlayerViewModel`, `PlaylistViewModel`, `SearchViewModel` — state holders feeding Compose UI
- **audio/**: `AudioPlayer`, `NowPlayingService` (foreground media service), `NowPlayingController`
- **data/api/**: `ApiClient`, `AuthManager` — OpenSubsonic REST client and auth/token handling
- **data/model/**: `Artist`, `Album`, `Song`, `Playlist` data classes
- **data/storage/**: `AppPreferences`, `PlaylistRepository`, `SecureStorage` (Keystore-backed credential storage)
- **ui/components/**: Compose UI — `PlayerBar`, `FullScreenPlayer`, `QueueSheet`, `LyricsSheet`, `AddToPlaylistDialog`, `FirmiumUi`/`FirmiumHeader`/`FirmiumTextField`/`FirmiumSwitch`/`FirmiumSlider`/`FirmiumBottomSheet`, `CoverImage`
- **ui/theme/**: `Theme.kt` — Compose theming

## Build Commands

Run from the repo root:

```bash
npm run android:build   # assembleRelease via Gradle
npm run android:debug   # assembleDebug via Gradle
npm run android:install # installDebug via adb
```

## Development Notes

- **Foreground service**: `NowPlayingService` requires `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_MEDIA_PLAYBACK` permissions (declared in `AndroidManifest.xml`). It must be started via `startForegroundService()` before calling `startForeground()` within 5 seconds or Android kills the app.
- **Compose recomposition**: ViewModels are the state source of truth. Never hoist mutable state into composables that are also updated from `viewModelScope` — it causes recomposition conflicts. Use `collectAsState()` from the ViewModel's `StateFlow`.
- **Credential storage**: `SecureStorage` uses Android Keystore — not `SharedPreferences`. Don't store passwords or tokens in `AppPreferences` (which uses `DataStore`/plain prefs).

## Networking Notes

- This app does **not** use `src/lib/*.js` — all networking goes through `ApiClient.kt` (OkHttp).
- `getLyrics`/`fetchLyricsForCurrent` and similar coroutine-based calls run on `viewModelScope.launch` (main dispatcher). Any blocking OkHttp `.execute()` call must be wrapped in `withContext(Dispatchers.IO)`, or it throws `NetworkOnMainThreadException` — which can be silently swallowed by surrounding catch blocks.

## Key Files

- `android/app/src/main/java/com/fossisawesome/firmium/MainActivity.kt` — Android app entry point
- `android/app/src/main/java/com/fossisawesome/firmium/data/api/ApiClient.kt` — OpenSubsonic API client
- `android/app/src/main/java/com/fossisawesome/firmium/audio/AudioPlayer.kt` — Audio playback engine
- `android/app/src/main/java/com/fossisawesome/firmium/audio/NowPlayingService.kt` — Foreground media service
