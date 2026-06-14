use std::sync::Arc;

use tauri::Manager;

use crate::audio::AudioPlayer;
use crate::{AudioDevice, PlaybackState};

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
pub fn play_stream<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    let player = get_player(&app_handle)?;
    AudioPlayer::play_stream(&player, stream_url, track_id.to_string(), replay_gain_db)
}

#[tauri::command]
pub fn preload_stream<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, stream_url: &str, track_id: &str, replay_gain_db: Option<f32>) -> Result<String, String> {
    let player = get_player(&app_handle)?;
    AudioPlayer::preload_stream(&player, stream_url, track_id.to_string(), replay_gain_db)
}

#[tauri::command]
pub fn set_bit_perfect_enabled<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, enabled: bool) -> Result<(), String> {
    get_player(&app_handle)?.set_bit_perfect_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub fn set_visualizer_enabled<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, enabled: bool) -> Result<(), String> {
    get_player(&app_handle)?.set_visualizer_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub fn pause_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.pause(player_id)
}

#[tauri::command]
pub fn resume_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.resume(player_id)
}

#[tauri::command]
pub fn stop_playback<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<(), String> {
    get_player(&app_handle)?.stop(player_id)
}

#[tauri::command]
pub fn set_volume<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str, volume: f32) -> Result<(), String> {
    get_player(&app_handle)?.set_volume(player_id, volume)
}

#[tauri::command]
pub fn get_volume<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<f32, String> {
    get_player(&app_handle)?.get_volume(player_id)
}

#[tauri::command]
pub fn get_playback_state<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<PlaybackState, String> {
    get_player(&app_handle)?.get_state(player_id)
}

#[tauri::command]
pub fn is_playback_finished<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<bool, String> {
    get_player(&app_handle)?.is_finished(player_id)
}

#[tauri::command]
pub fn get_track_duration<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<Option<f64>, String> {
    get_player(&app_handle)?.get_duration(player_id)
}

#[tauri::command]
pub fn get_current_position<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str) -> Result<f64, String> {
    get_player(&app_handle)?.get_current_position(player_id)
}

#[tauri::command]
pub fn seek_position<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, player_id: &str, position: f64) -> Result<(), String> {
    get_player(&app_handle)?.seek(player_id, position)
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    Ok(AudioPlayer::list_devices())
}

#[tauri::command]
pub fn crossfade_to<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    old_player_id: &str,
    stream_url: &str,
    track_id: &str,
    fade_duration_ms: u64,
    target_volume: f32,
    replay_gain_db: Option<f32>,
) -> Result<String, String> {
    let player = get_player(&app_handle)?;
    AudioPlayer::crossfade_to(&player, old_player_id, stream_url, track_id.to_string(), fade_duration_ms, target_volume, replay_gain_db)
}
