use keyring::Entry;
use std::io::Write as _;
use std::sync::Arc;
use tauri::Manager;
use sysinfo::System;

mod audio;
use audio::{AudioPlayer, PlaybackState};

/// System information structure for diagnostics
#[derive(serde::Serialize)]
struct SystemInfo {
    cpu: String,
    gpu: String,
    distro: String,
    version: String,
    package_manager: String,
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

/// Save a password to the OS system keyring (e.g. libsecret on Linux).
/// This is the correct place to store credentials — not localStorage.
#[tauri::command]
fn save_password(service: &str, user: &str, pass: &str) -> Result<(), String> {
    let entry = Entry::new(service, user).map_err(|e| e.to_string())?;
    entry.set_password(pass).map_err(|e| e.to_string())?;
    Ok(())
}

/// Retrieve a password from the OS system keyring.
#[tauri::command]
fn get_password(service: &str, user: &str) -> Result<String, String> {
    let entry = Entry::new(service, user).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

/// Delete a password from the OS system keyring.
#[tauri::command]
fn delete_password(service: &str, user: &str) -> Result<(), String> {
    let entry = Entry::new(service, user).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())?;
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
#[tauri::command]
fn write_log(app_handle: tauri::AppHandle, entry: String) -> Result<(), String> {
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

/// Return the app version string from Cargo.toml at compile time.
#[tauri::command]
fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ============================================================================
// SYSTEM DIAGNOSTICS
// ============================================================================

/// Fetch machine specifications for the settings/about page.
///
/// Made async to avoid blocking the Tauri event loop: `lspci` can take
/// 1-3 seconds on some systems, which would freeze the UI if run synchronously.
#[tauri::command]
async fn get_machine_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let distro = System::name().unwrap_or_else(|| "Unknown Linux".to_string());
    let version = System::os_version().unwrap_or_else(|| "0.0".to_string());

    // Use tokio::process::Command so these shell calls don't block the async runtime.
    #[cfg(target_os = "windows")]
    let gpu = tokio::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-WmiObject Win32_VideoController | Select-Object -First 1 -ExpandProperty Name",
        ])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Unknown GPU".to_string());

    #[cfg(not(target_os = "windows"))]
    let gpu = tokio::process::Command::new("sh")
        .arg("-c")
        .arg("lspci | grep -E 'VGA|3D' | cut -d ':' -f3 | sed 's/^[ \\t]*//'")
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Unknown GPU".to_string());

    #[cfg(target_os = "windows")]
    let package_manager = tokio::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "if (Get-Command winget -EA SilentlyContinue) { 'winget' } \
             elseif (Get-Command choco -EA SilentlyContinue) { 'chocolatey' } \
             elseif (Get-Command scoop -EA SilentlyContinue) { 'scoop' } \
             else { 'none' }",
        ])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "none".to_string());

    #[cfg(not(target_os = "windows"))]
    let package_manager = tokio::process::Command::new("sh")
        .arg("-c")
        .arg("which pacman || which apt || which dnf || which zypper || echo 'unknown'")
        .output()
        .await
        .map(|o| {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
            path.split('/').last().unwrap_or("unknown").to_string()
        })
        .unwrap_or_else(|_| "unknown".to_string());

    SystemInfo {
        cpu,
        gpu: if gpu.is_empty() {
            "Unknown GPU".to_string()
        } else {
            gpu
        },
        distro,
        version,
        package_manager,
    }
}

// ============================================================================
// AUDIO STREAM PLAYBACK INTERACTION HANDLERS
// ============================================================================

fn get_player(app_handle: &tauri::AppHandle) -> Result<tauri::State<'_, Arc<AudioPlayer>>, String> {
    app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())
}

/// Start streaming a track. Returns a player ID for subsequent control calls.
/// `replay_gain_db` is the ReplayGain track gain in dB; null means no adjustment.
#[tauri::command]
fn play_stream(app_handle: tauri::AppHandle, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    get_player(&app_handle)?.play_stream(stream_url, track_id.to_string(), replay_gain_db)
}

/// Pre-fetch and decode a track in a paused state for gapless playback.
/// Call resume_playback on the returned player ID to start audio instantly.
#[tauri::command]
fn preload_stream(app_handle: tauri::AppHandle, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    get_player(&app_handle)?.preload_stream(stream_url, track_id.to_string(), replay_gain_db)
}

#[tauri::command]
fn pause_playback(app_handle: tauri::AppHandle, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.pause(player_id)
}

#[tauri::command]
fn resume_playback(app_handle: tauri::AppHandle, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.resume(player_id)
}

#[tauri::command]
fn stop_playback(app_handle: tauri::AppHandle, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.stop(player_id)
}

#[tauri::command]
fn set_volume(app_handle: tauri::AppHandle, player_id: &str, volume: f32) -> Result<(), String> {
    get_player(&app_handle)?.set_volume(player_id, volume)
}

#[tauri::command]
fn get_volume(app_handle: tauri::AppHandle, player_id: &str) -> Result<f32, String> {
    get_player(&app_handle)?.get_volume(player_id)
}

#[tauri::command]
fn get_playback_state(app_handle: tauri::AppHandle, player_id: &str) -> Result<PlaybackState, String> {
    get_player(&app_handle)?.get_state(player_id)
}

#[tauri::command]
fn is_playback_finished(app_handle: tauri::AppHandle, player_id: &str) -> Result<bool, String> {
    get_player(&app_handle)?.is_finished(player_id)
}

#[tauri::command]
fn get_track_duration(app_handle: tauri::AppHandle, player_id: &str) -> Result<Option<f64>, String> {
    get_player(&app_handle)?.get_duration(player_id)
}

#[tauri::command]
fn get_current_position(app_handle: tauri::AppHandle, player_id: &str) -> Result<f64, String> {
    get_player(&app_handle)?.get_current_position(player_id)
}

#[tauri::command]
fn seek_position(app_handle: tauri::AppHandle, player_id: &str, position: f64) -> Result<(), String> {
    get_player(&app_handle)?.seek(player_id, position)
}

#[tauri::command]
fn list_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    Ok(AudioPlayer::list_devices())
}


/// Cross-fade from `old_player_id` into a new stream over `fade_duration_ms` milliseconds.
/// Volume steps run natively in a Rust async task — no IPC round-trips per step.
/// Returns the new player ID so the frontend can track the incoming session.
#[tauri::command]
fn crossfade_to(
    app_handle: tauri::AppHandle,
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize audio player within the setup lifecycle hook.
            // This ensures the Tokio async runtime context is fully running.
            // AppHandle is passed so the player can emit state-change events.
            let audio_player = Arc::new(
                AudioPlayer::new(app.handle().clone()).expect("Failed to initialize audio player"),
            );
            app.manage(audio_player);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            // System info
            get_machine_info,
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
