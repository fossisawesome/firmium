// keyring is only available on desktop OSes.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use keyring::Entry;
use std::io::Write as _;
use std::sync::Arc;
use tauri::Manager;

/// Whether the app was launched with --debug. Stored in managed state so commands can read it.
pub struct DebugMode(pub bool);

/// Playback state reported by the rodio audio engine.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

/// Audio device information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub default: bool,
}

mod audio;
use audio::AudioPlayer;

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

    // Read themes from disk (release: resource dir; debug: source themes/ dir).
    #[cfg(debug_assertions)]
    let bundled_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../themes");
    #[cfg(not(debug_assertions))]
    let bundled_dir = app_handle.path().resource_dir()
        .map(|d| d.join("themes"))
        .unwrap_or_default();

    for t in load_themes_from_dir(&bundled_dir) {
        if !seen.contains(&t.id) { themes.push(t); }
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

/// Save a password to the OS keyring.
#[tauri::command]
fn save_password(_service: &str, _user: &str, _pass: &str) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(_service, _user).map_err(|e| e.to_string())?;
        entry.set_password(_pass).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Retrieve a password from the OS keyring.
#[tauri::command]
fn get_password(_service: &str, _user: &str) -> Result<String, String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(_service, _user).map_err(|e| e.to_string())?;
        return entry.get_password().map_err(|e| e.to_string());
    }
    #[allow(unreachable_code)]
    Err("Keyring not available on this platform".to_string())
}

/// Delete a password from the OS keyring.
#[tauri::command]
fn delete_password(_service: &str, _user: &str) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(_service, _user).map_err(|e| e.to_string())?;
        entry.delete_credential().map_err(|e| e.to_string())?;
    }
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
// Delegates to the Rust rodio AudioPlayer. The JS AudioBridge calls these via Tauri IPC.

/// Helper: retrieves the managed rodio AudioPlayer.
fn get_player<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) -> Result<tauri::State<'_, Arc<AudioPlayer>>, String> {
    app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())
}


#[tauri::command]
fn play_stream<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    get_player(&app_handle)?.play_stream(stream_url, track_id.to_string(), replay_gain_db)
}

#[tauri::command]
fn preload_stream<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    get_player(&app_handle)?.preload_stream(stream_url, track_id.to_string(), replay_gain_db)
}

#[tauri::command]
fn pause_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.pause(player_id)
}

#[tauri::command]
fn resume_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.resume(player_id)
}

#[tauri::command]
fn stop_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.stop(player_id)
}

#[tauri::command]
fn set_volume<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str, volume: f32) -> Result<(), String> {
    get_player(&app_handle)?.set_volume(player_id, volume)
}

#[tauri::command]
fn get_volume<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<f32, String> {
    get_player(&app_handle)?.get_volume(player_id)
}

#[tauri::command]
fn get_playback_state<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<PlaybackState, String> {
    get_player(&app_handle)?.get_state(player_id)
}

#[tauri::command]
fn is_playback_finished<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<bool, String> {
    get_player(&app_handle)?.is_finished(player_id)
}

#[tauri::command]
fn get_track_duration<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<Option<f64>, String> {
    get_player(&app_handle)?.get_duration(player_id)
}

#[tauri::command]
fn get_current_position<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<f64, String> {
    get_player(&app_handle)?.get_current_position(player_id)
}

#[tauri::command]
fn seek_position<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str, position: f64) -> Result<(), String> {
    get_player(&app_handle)?.seek(player_id, position)
}

#[tauri::command]
fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
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
    get_player(&app_handle)?.crossfade_to(old_player_id, stream_url, track_id.to_string(), fade_duration_ms, target_volume, replay_gain_db)
}


// ============================================================================
// APPLICATION ENTRY POINT
// ============================================================================

/// App entry point.
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
        .plugin(tauri_plugin_http::init());

    builder
        .setup(move |_app| {
            let audio_player = Arc::new(
                AudioPlayer::new(_app.handle().clone()).expect("Failed to initialize audio player"),
            );
            _app.manage(audio_player);

            // Open DevTools immediately when --debug is passed.
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
