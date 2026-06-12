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
mod commands;
use commands::*;

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
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

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
