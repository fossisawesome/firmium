// src-tauri/src/main.rs
use keyring::Entry;
use std::fs::{self, File};
use std::io::Write;
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

/// Playback status event sent to frontend
#[derive(serde::Serialize, Clone)]
struct PlaybackStatus {
    player_id: String,
    state: PlaybackState,
    finished: bool,
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
// SUBSONIC AUTH
// ============================================================================

/// Generate Subsonic token-auth query parameters on the Rust side.
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
// COVER ART CACHING
// ============================================================================

/// Fetch and cache cover art locally, avoiding re-downloads on subsequent views.
#[tauri::command]
async fn cache_cover(
    app_handle: tauri::AppHandle,
    id: String,
    server_url: String,
) -> Result<String, String> {
    let mut cache_path = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?;
    cache_path.push("covers");

    fs::create_dir_all(&cache_path).map_err(|e| e.to_string())?;
    cache_path.push(format!("{}.img", id));

    if cache_path.exists() {
        return Ok(cache_path.to_string_lossy().into_owned());
    }

    let response = reqwest::get(&server_url).await.map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    let mut file = File::create(&cache_path).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;

    Ok(cache_path.to_string_lossy().into_owned())
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
    let gpu = tokio::process::Command::new("sh")
        .arg("-c")
        .arg("lspci | grep -E 'VGA|3D' | cut -d ':' -f3 | sed 's/^[ \\t]*//'")
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Unknown GPU".to_string());

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

#[tauri::command]
fn play_stream(
    app_handle: tauri::AppHandle,
    stream_url: &str,
    track_id: &str,
) -> Result<String, String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).play_stream(stream_url, track_id.to_string())
}

#[tauri::command]
fn pause_playback(app_handle: tauri::AppHandle, player_id: &str) -> Result<(), String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).pause(player_id)
}

/// Resume a paused session. Calls AudioPlayer::resume (previously misnamed `play`).
#[tauri::command]
fn resume_playback(app_handle: tauri::AppHandle, player_id: &str) -> Result<(), String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).resume(player_id)
}

#[tauri::command]
fn stop_playback(app_handle: tauri::AppHandle, player_id: &str) -> Result<(), String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).stop(player_id)
}

#[tauri::command]
fn set_volume(
    app_handle: tauri::AppHandle,
    player_id: &str,
    volume: f32,
) -> Result<(), String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).set_volume(player_id, volume)
}

#[tauri::command]
fn get_volume(app_handle: tauri::AppHandle, player_id: &str) -> Result<f32, String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).get_volume(player_id)
}

#[tauri::command]
fn get_playback_state(
    app_handle: tauri::AppHandle,
    player_id: &str,
) -> Result<String, String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    let state = (&*player).get_state(player_id)?;
    Ok(match state {
        PlaybackState::Loading => "loading".to_string(),
        PlaybackState::Playing => "playing".to_string(),
        PlaybackState::Paused => "paused".to_string(),
        PlaybackState::Stopped => "stopped".to_string(),
    })
}

#[tauri::command]
fn is_playback_finished(
    app_handle: tauri::AppHandle,
    player_id: &str,
) -> Result<bool, String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).is_finished(player_id)
}

#[tauri::command]
fn get_track_duration(
    app_handle: tauri::AppHandle,
    player_id: &str,
) -> Result<Option<f64>, String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).get_duration(player_id)
}

#[tauri::command]
fn get_current_position(
    app_handle: tauri::AppHandle,
    player_id: &str,
) -> Result<f64, String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).get_current_position(player_id)
}

#[tauri::command]
fn seek_position(
    app_handle: tauri::AppHandle,
    player_id: &str,
    position: f64,
) -> Result<(), String> {
    let player = app_handle
        .try_state::<Arc<AudioPlayer>>()
        .ok_or_else(|| "Audio Player state not registered".to_string())?;

    (&*player).seek(player_id, position)
}

/// List available audio output devices.
#[tauri::command]
fn list_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    Ok(AudioPlayer::list_devices())
}

// ============================================================================
// APPLICATION ENTRY POINT
// ============================================================================

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize audio player within the setup lifecycle hook.
            // This ensures the Tokio async runtime context is fully running.
            let audio_player = Arc::new(
                AudioPlayer::new().expect("Failed to initialize audio player"),
            );
            app.manage(audio_player);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Credentials
            save_password,
            get_password,
            delete_password,
            // Auth
            generate_auth_params,
            // Cover art
            cache_cover,
            // System info
            get_machine_info,
            // Audio playback
            play_stream,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}