// keyring is only available on desktop OSes.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use keyring::Entry;
use std::io::Write as _;
#[cfg(not(target_os = "android"))]
use std::sync::Arc;
use tauri::Manager;

// ============================================================================
// ANDROID SECURE STORAGE PLUGIN
// ============================================================================

/// Holds the JNI handle to the Android SecureStoragePlugin (EncryptedSharedPreferences).
/// Only exists on Android; desktop uses the OS keyring directly.
#[cfg(target_os = "android")]
struct SecureStorageHandle<R: tauri::Runtime>(tauri::plugin::PluginHandle<R>);

/// Tauri plugin that registers the Kotlin SecureStoragePlugin on Android
/// and stores its handle in managed app state for use by credential commands.
fn secure_storage_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("secure-storage")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin(
                    "com.fossisawesome.firmium",
                    "SecureStoragePlugin",
                )?;
                app.manage(SecureStorageHandle(handle));
            }
            let _ = (app, api);
            Ok(())
        })
        .build()
}

/// Whether the app was launched with --debug. Stored in managed state so commands can read it.
pub struct DebugMode(pub bool);

/// Playback state reported by both the Rust (rodio) and Kotlin (ExoPlayer) audio engines.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

/// Audio device information, shared between the Rust audio engine and the Android stub.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub default: bool,
}

#[cfg(not(target_os = "android"))]
mod audio;
#[cfg(not(target_os = "android"))]
use audio::AudioPlayer;

/// Holds the JNI handle to the Android AudioPlugin (ExoPlayer).
#[cfg(target_os = "android")]
struct AudioHandle<R: tauri::Runtime>(tauri::plugin::PluginHandle<R>);

/// Registers the Kotlin AudioPlugin on Android. On desktop this is a no-op —
/// the Rust rodio engine is used directly via AudioPlayer managed state.
fn audio_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("audio")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin(
                    "com.fossisawesome.firmium",
                    "AudioPlugin",
                )?;
                app.manage(AudioHandle(handle));
            }
            let _ = (app, api);
            Ok(())
        })
        .build()
}

/// Holds the JNI handle to the Android NowPlayingPlugin (MediaSession + notification).
#[cfg(target_os = "android")]
struct NowPlayingHandle<R: tauri::Runtime>(tauri::plugin::PluginHandle<R>);

/// Registers the Kotlin NowPlayingPlugin on Android so its commands are callable from JS.
fn now_playing_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("now-playing")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin(
                    "com.fossisawesome.firmium",
                    "NowPlayingPlugin",
                )?;
                app.manage(NowPlayingHandle(handle));
            }
            let _ = (app, api);
            Ok(())
        })
        .build()
}

// ============================================================================
// THEME LOADING
// ============================================================================

/// Color variables for a theme, matching the CSS custom properties in style.css.
#[derive(serde::Deserialize, serde::Serialize)]
struct ThemeColors {
    bg: String,
    surface: String,
    surface2: String,
    border: String,
    text: String,
    muted: String,
    accent: String,
    accent_dim: String,
    error: String,
    font: Option<String>,
    timing: Option<String>,
}

/// Raw shape of a .toml theme file on disk.
#[derive(serde::Deserialize)]
struct ThemeFile {
    name: String,
    color_scheme: Option<String>,
    colors: ThemeColors,
}

/// Serialized theme entry returned to the frontend via list_themes.
#[derive(serde::Serialize)]
struct ThemeEntry {
    id: String,
    name: String,
    color_scheme: String,
    colors: ThemeColors,
}

// Themes embedded at compile time by build.rs — used on Android where
// std::fs cannot read APK assets, and as a fallback on all platforms.
include!(concat!(env!("OUT_DIR"), "/embedded_themes.rs"));

/// Parse a TOML string into a ThemeEntry, returning None if invalid.
fn parse_theme(id: &str, content: &str) -> Option<ThemeEntry> {
    let tf = toml::from_str::<ThemeFile>(content).ok()?;
    Some(ThemeEntry {
        id: id.to_string(),
        name: tf.name,
        color_scheme: tf.color_scheme.unwrap_or_else(|| "dark".to_string()),
        colors: tf.colors,
    })
}

/// Read all valid .toml files from a directory into ThemeEntry values.
fn load_themes_from_dir(dir: &std::path::Path) -> Vec<ThemeEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") { continue }
        let id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        if let Some(t) = parse_theme(&id, &content) { result.push(t) }
    }
    result
}

/// Return all available themes. On Android built-ins come from the compile-time
/// embedded array; on desktop they are read from the resource directory (or source
/// dir in dev). User themes from the app config dir override built-ins on all platforms.
#[tauri::command]
fn list_themes(app_handle: tauri::AppHandle) -> Vec<ThemeEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut themes: Vec<ThemeEntry> = Vec::new();

    // User themes take priority — collect them first and record their IDs.
    if let Ok(config_dir) = app_handle.path().app_config_dir() {
        for t in load_themes_from_dir(&config_dir.join("themes")) {
            seen.insert(t.id.clone());
            themes.push(t);
        }
    }

    // On Android use the compile-time embedded themes; on desktop read from disk.
    #[cfg(target_os = "android")]
    {
        for (id, content) in EMBEDDED_THEMES {
            if !seen.contains(*id) {
                if let Some(t) = parse_theme(id, content) {
                    themes.push(t);
                }
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        #[cfg(debug_assertions)]
        let bundled_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../themes");
        #[cfg(not(debug_assertions))]
        let bundled_dir = app_handle.path().resource_dir()
            .map(|d| d.join("themes"))
            .unwrap_or_default();

        for t in load_themes_from_dir(&bundled_dir) {
            if !seen.contains(&t.id) { themes.push(t); }
        }
    }

    // Keep Firmium first; sort the rest alphabetically by display name.
    themes.sort_by(|a, b| {
        if a.id == "firmium" { return std::cmp::Ordering::Less }
        if b.id == "firmium" { return std::cmp::Ordering::Greater }
        a.name.cmp(&b.name)
    });

    themes
}

// ============================================================================
// OPENSUBSONIC DATA MAPPERS
// ============================================================================
// These structs mirror the shapes that the JS API layer produces from raw
// Subsonic JSON. Mapping on the Rust side gives us exhaustive pattern matching
// for release-type inference and keeps transform logic out of JS.

/// Mapped album, returned to JS in camelCase via serde.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Album {
    id: String,
    name: String,
    album_artist: String,
    artist_id: Option<String>,
    cover_art_id: Option<String>,
    song_count: Option<u32>,
    release_type: String,
    genres: Option<serde_json::Value>,
    year: Option<u32>,
    is_compilation: bool,
}

/// Mapped artist, returned to JS in camelCase via serde.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Artist {
    id: String,
    name: String,
    album_count: u32,
}

/// Mapped song, returned to JS in camelCase via serde.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Song {
    id: String,
    title: String,
    artist: String,
    album: String,
    album_id: Option<String>,
    duration: f64,
    track_number: Option<u32>,
    cover_art_id: Option<String>,
    replay_gain: Option<serde_json::Value>,
    bpm: Option<f64>,
    comment: Option<String>,
    genres: Option<serde_json::Value>,
}

/// Infer release type from explicit server fields, title keywords, and song count.
/// Mirrors the inferReleaseType() function previously in api.js.
fn infer_release_type(a: &serde_json::Value) -> String {
    // Prefer explicit releaseTypes[] array, then releaseType string.
    let explicit = a.get("releaseTypes")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .or_else(|| a.get("releaseType").and_then(|v| v.as_str()));

    if let Some(t) = explicit {
        return t.to_lowercase();
    }

    let title = a.get("name").or_else(|| a.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    if title.contains(" - single") || title.ends_with("(single)") || title.ends_with("- single") {
        return "single".to_string();
    }
    if title.contains(" - ep") || title.ends_with("(ep)") || title.ends_with("- ep") {
        return "ep".to_string();
    }

    let count = a.get("songCount").and_then(|v| v.as_u64()).unwrap_or(0);
    match count {
        1 | 2 => "single".to_string(),
        3..=6 => "ep".to_string(),
        _ => "album".to_string(),
    }
}

fn map_album(a: &serde_json::Value) -> Album {
    Album {
        id: a.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        name: a.get("name").or_else(|| a.get("title"))
            .and_then(|v| v.as_str()).unwrap_or("Unknown Album").to_string(),
        album_artist: a.get("displayArtist").or_else(|| a.get("artist"))
            .and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        artist_id: a.get("artistId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        cover_art_id: a.get("coverArt").and_then(|v| v.as_str()).map(|s| s.to_string()),
        song_count: a.get("songCount").and_then(|v| v.as_u64()).map(|n| n as u32),
        release_type: infer_release_type(a),
        genres: a.get("genres").cloned(),
        year: a.get("year").and_then(|v| v.as_u64()).map(|n| n as u32),
        is_compilation: a.get("isCompilation").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

fn map_artist(a: &serde_json::Value) -> Artist {
    Artist {
        id: a.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        name: a.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        album_count: a.get("albumCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    }
}

fn map_song(s: &serde_json::Value) -> Song {
    Song {
        id: s.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        title: s.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown Track").to_string(),
        artist: s.get("displayArtist").or_else(|| s.get("artist"))
            .and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        album: s.get("album").and_then(|v| v.as_str()).unwrap_or("Unknown Album").to_string(),
        album_id: s.get("albumId").and_then(|v| v.as_str()).map(|v| v.to_string()),
        duration: s.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0),
        track_number: s.get("track").and_then(|v| v.as_u64()).map(|n| n as u32),
        cover_art_id: s.get("coverArt").and_then(|v| v.as_str()).map(|v| v.to_string()),
        replay_gain: s.get("replayGain").cloned(),
        bpm: s.get("bpm").and_then(|v| v.as_f64()),
        comment: s.get("comment").and_then(|v| v.as_str()).map(|v| v.to_string()),
        genres: s.get("genres").cloned(),
    }
}

/// Map a batch of raw Subsonic album objects to typed Album structs.
#[tauri::command]
fn map_albums(albums: Vec<serde_json::Value>) -> Vec<Album> {
    albums.iter().map(map_album).collect()
}

/// Map a batch of raw Subsonic artist objects to typed Artist structs.
#[tauri::command]
fn map_artists(artists: Vec<serde_json::Value>) -> Vec<Artist> {
    artists.iter().map(map_artist).collect()
}

/// Map a batch of raw Subsonic song objects to typed Song structs.
#[tauri::command]
fn map_songs(songs: Vec<serde_json::Value>) -> Vec<Song> {
    songs.iter().map(map_song).collect()
}

// ============================================================================
// KEYRING / CREDENTIALS MANAGEMENT
// ============================================================================

/// Save a password to the OS keyring (desktop) or Android Keystore-backed
/// EncryptedSharedPreferences (Android).
#[tauri::command]
fn save_password<R: tauri::Runtime>(
    #[allow(unused_variables)] app: tauri::AppHandle<R>,
    _service: &str,
    _user: &str,
    _pass: &str,
) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(_service, _user).map_err(|e| e.to_string())?;
        entry.set_password(_pass).map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "android")]
    {
        #[derive(serde::Serialize)]
        struct Args<'a> { service: &'a str, user: &'a str, pass: &'a str }
        #[derive(serde::Deserialize)]
        struct Empty {}
        app.state::<SecureStorageHandle<R>>()
            .0
            .run_mobile_plugin::<Empty>("savePassword", Args { service: _service, user: _user, pass: _pass })
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Ok(())
}

/// Retrieve a password from the OS keyring (desktop) or Android EncryptedSharedPreferences.
#[tauri::command]
fn get_password<R: tauri::Runtime>(
    #[allow(unused_variables)] app: tauri::AppHandle<R>,
    _service: &str,
    _user: &str,
) -> Result<String, String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(_service, _user).map_err(|e| e.to_string())?;
        return entry.get_password().map_err(|e| e.to_string());
    }
    #[cfg(target_os = "android")]
    {
        #[derive(serde::Serialize)]
        struct Args<'a> { service: &'a str, user: &'a str }
        #[derive(serde::Deserialize)]
        struct Response { value: String }
        let resp = app.state::<SecureStorageHandle<R>>()
            .0
            .run_mobile_plugin::<Response>("getPassword", Args { service: _service, user: _user })
            .map_err(|e| e.to_string())?;
        return Ok(resp.value);
    }
    #[allow(unreachable_code)]
    Err("Unsupported platform".to_string())
}

/// Delete a password from the OS keyring (desktop) or Android EncryptedSharedPreferences.
#[tauri::command]
fn delete_password<R: tauri::Runtime>(
    #[allow(unused_variables)] app: tauri::AppHandle<R>,
    _service: &str,
    _user: &str,
) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(_service, _user).map_err(|e| e.to_string())?;
        entry.delete_credential().map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "android")]
    {
        #[derive(serde::Serialize)]
        struct Args<'a> { service: &'a str, user: &'a str }
        #[derive(serde::Deserialize)]
        struct Empty {}
        app.state::<SecureStorageHandle<R>>()
            .0
            .run_mobile_plugin::<Empty>("deletePassword", Args { service: _service, user: _user })
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Ok(())
}

// ============================================================================
// OPENSUBSONIC AUTH
// ============================================================================

/// Generate OpenSubsonic token-auth query parameters on the Rust side.
/// Keeps MD5 out of the JS layer — the frontend passes plaintext credentials
/// and receives the ready-to-use param map.
#[tauri::command]
fn generate_auth_params(username: String, password: String) -> serde_json::Value {
    use std::fmt::Write as _;
    // Cryptographically random 8-byte salt, hex-encoded.
    let salt_bytes: [u8; 8] = rand::random();
    let mut salt = String::with_capacity(16);
    for b in salt_bytes {
        let _ = write!(salt, "{:02x}", b);
    }
    let token = format!("{:x}", md5::compute(format!("{}{}", password, salt)));
    serde_json::json!({
        "u": username,
        "t": token,
        "s": salt,
        "v": "1.16.1",
        "c": "firmium",
        "f": "json"
    })
}

// ============================================================================
// LOGGING
// ============================================================================

/// Append a pre-formatted log entry (timestamp + level + message built by JS) to app-logs.txt.
/// In debug mode, also echoes to stderr so frontend console output is visible in the terminal.
#[tauri::command]
fn write_log(app_handle: tauri::AppHandle, entry: String) -> Result<(), String> {
    if app_handle.state::<DebugMode>().0 {
        eprintln!("[js] {}", entry);
    }
    let log_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let log_file = log_dir.join("app-logs.txt");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{}", entry).map_err(|e| e.to_string())
}

/// Delete the app-logs.txt file.
#[tauri::command]
fn delete_logs(app_handle: tauri::AppHandle) -> Result<(), String> {
    let log_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_file = log_dir.join("app-logs.txt");
    std::fs::remove_file(&log_file).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e.to_string()) }
    })
}

/// Return the absolute path to app-logs.txt so the UI can display it.
#[tauri::command]
fn get_log_path(app_handle: tauri::AppHandle) -> Result<String, String> {
    let log_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(log_dir.join("app-logs.txt").to_string_lossy().into_owned())
}

/// Expose debug mode to the frontend so it can block devtools shortcuts when false.
#[tauri::command]
fn is_debug_mode(app_handle: tauri::AppHandle) -> bool {
    app_handle.state::<DebugMode>().0
}

/// Return the app version string from Cargo.toml at compile time.
#[tauri::command]
fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ============================================================================
// AUDIO STREAM PLAYBACK INTERACTION HANDLERS
// ============================================================================
//
// Each command has two paths selected at compile time:
//   #[cfg(target_os = "android")] → delegates to the Kotlin AudioPlugin via JNI
//   #[cfg(not(...))]              → delegates to the Rust rodio AudioPlayer
//
// The JS AudioBridge calls these commands identically on both platforms.

/// Desktop-only helper: retrieves the managed rodio AudioPlayer.
#[cfg(not(target_os = "android"))]
fn get_player<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) -> Result<tauri::State<'_, Arc<AudioPlayer>>, String> {
    app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())
}

// Serializable arg structs reused across Android audio command dispatch calls.
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidPlayStreamArgs<'a> {
    #[serde(rename = "streamUrl")]  stream_url:     &'a str,
    #[serde(rename = "trackId")]    track_id:       &'a str,
    #[serde(rename = "replayGainDb")] replay_gain_db: Option<f32>,
}
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct AndroidPlayerIdResp { #[serde(rename = "playerId")] player_id: String }
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidPlayerIdArgs<'a> { #[serde(rename = "playerId")] player_id: &'a str }
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidSeekArgs<'a> { #[serde(rename = "playerId")] player_id: &'a str, position: f64 }
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidVolumeArgs<'a> { #[serde(rename = "playerId")] player_id: &'a str, volume: f32 }
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct AndroidVolumeResp { volume: f32 }
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct AndroidStateResp { state: String }
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct AndroidFinishedResp { finished: bool }
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct AndroidPositionResp { position: f64 }
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct AndroidDurationResp { duration: Option<f64> }
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidCrossfadeArgs<'a> {
    #[serde(rename = "oldPlayerId")]    old_player_id:    &'a str,
    #[serde(rename = "streamUrl")]      stream_url:       &'a str,
    #[serde(rename = "trackId")]        track_id:         &'a str,
    #[serde(rename = "fadeDurationMs")] fade_duration_ms: u64,
    #[serde(rename = "targetVolume")]   target_volume:    f32,
    #[serde(rename = "replayGainDb")]   replay_gain_db:   Option<f32>,
}

// Args for the native queue commands (Android only).
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidQueueTrack {
    #[serde(rename = "streamUrl")]    stream_url:     String,
    #[serde(rename = "trackId")]      track_id:       String,
    #[serde(rename = "replayGainDb")] replay_gain_db: Option<f32>,
}
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidSetQueueArgs {
    tracks:       Vec<AndroidQueueTrack>,
    #[serde(rename = "startIndex")] start_index: usize,
    volume:       f32,
}
#[cfg(target_os = "android")]
#[derive(serde::Serialize)]
struct AndroidSkipToIndexArgs<'a> {
    #[serde(rename = "playerId")] player_id: &'a str,
    index: usize,
}
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct AndroidQueueIndexResp {
    index: usize,
    #[serde(rename = "trackId")] track_id: String,
}

// Input struct for set_queue — JS sends camelCase, serde_json handles it via renames.
#[derive(serde::Deserialize)]
struct QueueTrackInput {
    #[serde(rename = "streamUrl")]    stream_url:     String,
    #[serde(rename = "trackId")]      track_id:       String,
    #[serde(rename = "replayGainDb")] replay_gain_db: Option<f32>,
}

/// Helper: get the AudioHandle state on Android.
#[cfg(target_os = "android")]
fn get_audio_handle<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<tauri::State<'_, AudioHandle<R>>, String> {
    app.try_state::<AudioHandle<R>>()
        .ok_or_else(|| "Audio plugin not registered".to_string())
}

#[tauri::command]
fn play_stream<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidPlayerIdResp>("playStream", AndroidPlayStreamArgs { stream_url, track_id, replay_gain_db })
        .map(|r| r.player_id).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.play_stream(stream_url, track_id.to_string(), replay_gain_db)
}

#[tauri::command]
fn preload_stream<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidPlayerIdResp>("preloadStream", AndroidPlayStreamArgs { stream_url, track_id, replay_gain_db })
        .map(|r| r.player_id).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.preload_stream(stream_url, track_id.to_string(), replay_gain_db)
}

#[tauri::command]
fn pause_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("pausePlayback", AndroidPlayerIdArgs { player_id })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.pause(player_id)
}

#[tauri::command]
fn resume_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("resumePlayback", AndroidPlayerIdArgs { player_id })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.resume(player_id)
}

#[tauri::command]
fn stop_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("stopPlayback", AndroidPlayerIdArgs { player_id })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.stop(player_id)
}

#[tauri::command]
fn set_volume<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str, volume: f32) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("setVolume", AndroidVolumeArgs { player_id, volume })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.set_volume(player_id, volume)
}

#[tauri::command]
fn get_volume<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<f32, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidVolumeResp>("getVolume", AndroidPlayerIdArgs { player_id })
        .map(|r| r.volume).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.get_volume(player_id)
}

#[tauri::command]
fn get_playback_state<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<PlaybackState, String> {
    #[cfg(target_os = "android")]
    {
        let state_str = get_audio_handle(&app_handle)?.0
            .run_mobile_plugin::<AndroidStateResp>("getPlaybackState", AndroidPlayerIdArgs { player_id })
            .map(|r| r.state).map_err(|e| e.to_string())?;
        return match state_str.as_str() {
            "loading"  => Ok(PlaybackState::Loading),
            "playing"  => Ok(PlaybackState::Playing),
            "paused"   => Ok(PlaybackState::Paused),
            _          => Ok(PlaybackState::Stopped),
        };
    }
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.get_state(player_id)
}

#[tauri::command]
fn is_playback_finished<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidFinishedResp>("isPlaybackFinished", AndroidPlayerIdArgs { player_id })
        .map(|r| r.finished).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.is_finished(player_id)
}

#[tauri::command]
fn get_track_duration<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<Option<f64>, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidDurationResp>("getTrackDuration", AndroidPlayerIdArgs { player_id })
        .map(|r| r.duration).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.get_duration(player_id)
}

#[tauri::command]
fn get_current_position<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<f64, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidPositionResp>("getCurrentPosition", AndroidPlayerIdArgs { player_id })
        .map(|r| r.position).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.get_current_position(player_id)
}

#[tauri::command]
fn seek_position<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str, position: f64) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("seekPosition", AndroidSeekArgs { player_id, position })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.seek(player_id, position)
}

#[tauri::command]
fn list_audio_devices<R: tauri::Runtime>(_app_handle: tauri::AppHandle<R>) -> Result<Vec<AudioDevice>, String> {
    #[cfg(target_os = "android")]
    return Ok(vec![AudioDevice { name: "Default Output".to_string(), default: true }]);
    #[cfg(not(target_os = "android"))]
    Ok(AudioPlayer::list_devices())
}

#[tauri::command]
fn crossfade_to<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    old_player_id: &str,
    stream_url: &str,
    track_id: &str,
    fade_duration_ms: u64,
    target_volume: f32,
    replay_gain_db: Option<f32>,
) -> Result<String, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidPlayerIdResp>("crossfadeTo", AndroidCrossfadeArgs {
            old_player_id, stream_url, track_id, fade_duration_ms, target_volume, replay_gain_db,
        })
        .map(|r| r.player_id).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    get_player(&app_handle)?.crossfade_to(old_player_id, stream_url, track_id.to_string(), fade_duration_ms, target_volume, replay_gain_db)
}

// ── Native queue commands (Android only) ──────────────────────────────────────

// Loads all tracks into a single ExoPlayer playlist and starts at startIndex.
// Returns a playerId usable with all standard playback commands.
#[tauri::command]
fn set_queue<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, tracks: Vec<QueueTrackInput>, start_index: usize, volume: f32) -> Result<String, String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<AndroidPlayerIdResp>("setQueue", AndroidSetQueueArgs {
            tracks: tracks.into_iter().map(|t| AndroidQueueTrack {
                stream_url: t.stream_url, track_id: t.track_id, replay_gain_db: t.replay_gain_db,
            }).collect(),
            start_index,
            volume,
        })
        .map(|r| r.player_id).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    Err("set_queue is only supported on Android".to_string())
}

#[tauri::command]
fn skip_to_next<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("skipToNext", AndroidPlayerIdArgs { player_id })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    { let _ = player_id; Err("skip_to_next is only supported on Android".to_string()) }
}

#[tauri::command]
fn skip_to_previous<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("skipToPrevious", AndroidPlayerIdArgs { player_id })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    { let _ = player_id; Err("skip_to_previous is only supported on Android".to_string()) }
}

#[tauri::command]
fn skip_to_queue_index<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str, index: usize) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return get_audio_handle(&app_handle)?.0
        .run_mobile_plugin::<serde_json::Value>("skipToQueueIndex", AndroidSkipToIndexArgs { player_id, index })
        .map(|_| ()).map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    { let _ = (player_id, index); Err("skip_to_queue_index is only supported on Android".to_string()) }
}

// Returns the current queue position tracked by the native player — used by the
// JS visibility handler to re-sync queueIdx after tracks advanced while backgrounded.
#[tauri::command]
fn get_current_queue_index<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "android")]
    {
        let resp = get_audio_handle(&app_handle)?.0
            .run_mobile_plugin::<AndroidQueueIndexResp>("getQueueIndex", AndroidPlayerIdArgs { player_id })
            .map_err(|e| e.to_string())?;
        return Ok(serde_json::json!({ "index": resp.index, "trackId": resp.track_id }));
    }
    #[cfg(not(target_os = "android"))]
    { let _ = (app_handle, player_id); Err("get_current_queue_index is only supported on Android".to_string()) }
}

// ============================================================================
// NOW PLAYING NOTIFICATION (Android only)
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
struct NowPlayingArgs {
    title: String,
    artist: String,
    album: String,
    #[serde(rename = "coverUrl")]
    cover_url: String,
    #[serde(rename = "isPlaying")]
    is_playing: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
struct PlaybackStateArgs {
    #[serde(rename = "isPlaying")]
    is_playing: bool,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct NowPlayingEmpty {}

// Individual params match the JS payload keys (Tauri maps camelCase↔snake_case).
// A struct param named `args` would require the JS to nest fields under an `args` key.
#[tauri::command]
fn update_now_playing<R: tauri::Runtime>(
    #[allow(unused_variables)] app_handle: tauri::AppHandle<R>,
    #[allow(unused_variables)] title: String,
    #[allow(unused_variables)] artist: String,
    #[allow(unused_variables)] album: String,
    #[allow(unused_variables)] cover_url: String,
    #[allow(unused_variables)] is_playing: bool,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let args = NowPlayingArgs { title, artist, album, cover_url, is_playing };
        app_handle
            .state::<NowPlayingHandle<R>>()
            .0
            .run_mobile_plugin::<NowPlayingEmpty>("updateNowPlaying", &args)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn update_playback_state<R: tauri::Runtime>(
    #[allow(unused_variables)] app_handle: tauri::AppHandle<R>,
    #[allow(unused_variables)] is_playing: bool,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let args = PlaybackStateArgs { is_playing };
        app_handle
            .state::<NowPlayingHandle<R>>()
            .0
            .run_mobile_plugin::<NowPlayingEmpty>("updatePlaybackState", &args)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn clear_now_playing<R: tauri::Runtime>(
    #[allow(unused_variables)] app_handle: tauri::AppHandle<R>,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        #[derive(serde::Serialize)]
        struct Empty {}
        app_handle
            .state::<NowPlayingHandle<R>>()
            .0
            .run_mobile_plugin::<NowPlayingEmpty>("clearNowPlaying", Empty {})
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================================================================
// APPLICATION ENTRY POINT
// ============================================================================

/// App entry point — called by main() on desktop and by the Android JNI bridge on mobile.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let debug_mode = std::env::args().any(|a| a == "--debug");

    if debug_mode {
        eprintln!("[firmium] debug mode — frontend console and Rust output will appear here");
        // Surface Tauri/wry internal logs if RUST_LOG isn't already set by the caller.
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "tauri=debug,wry=debug,firmium=debug");
        }
    }

    let builder = tauri::Builder::default()
        .manage(DebugMode(debug_mode))
        .plugin(secure_storage_plugin())
        .plugin(audio_plugin())
        .plugin(now_playing_plugin())
        .plugin(tauri_plugin_http::init());

    builder
        .setup(move |_app| {
            // rodio/cpal cannot open a default sink on Android without the oboe
            // C++ toolchain configured — skip audio init there.
            // Audio commands return Err("Audio Player state not registered") on Android,
            // which the JS layer already handles gracefully.
            #[cfg(not(target_os = "android"))]
            {
                let audio_player = Arc::new(
                    AudioPlayer::new(_app.handle().clone()).expect("Failed to initialize audio player"),
                );
                _app.manage(audio_player);
            }

            // Open DevTools immediately when --debug is passed.
            #[cfg(not(mobile))]
            if debug_mode {
                if let Some(win) = _app.get_webview_window("main") {
                    win.open_devtools();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Themes
            list_themes,
            // Data mappers
            map_albums,
            map_artists,
            map_songs,
            // Credentials
            save_password,
            get_password,
            delete_password,
            // Auth
            generate_auth_params,
            // Logging
            write_log,
            delete_logs,
            get_log_path,
            get_app_version,
            is_debug_mode,
            // Audio playback
            play_stream,
            preload_stream,
            pause_playback,
            resume_playback,
            stop_playback,
            set_volume,
            get_volume,
            get_playback_state,
            is_playback_finished,
            get_track_duration,
            get_current_position,
            seek_position,
            list_audio_devices,
            crossfade_to,
            set_queue,
            skip_to_next,
            skip_to_previous,
            skip_to_queue_index,
            get_current_queue_index,
            // Now Playing notification (Android)
            update_now_playing,
            update_playback_state,
            clear_now_playing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
