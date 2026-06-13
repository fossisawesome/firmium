use std::sync::Arc;
use tauri::Manager;

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
mod state;
use state::AppState;

// ============================================================================
// APPLICATION ENTRY POINT
// ============================================================================

/// App entry point.
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .setup(move |_app| {
            let audio_player = Arc::new(
                AudioPlayer::new(_app.handle().clone()).expect("Failed to initialize audio player"),
            );
            _app.manage(audio_player);
            _app.manage(Arc::new(AppState::new()));

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
            // App info
            get_app_version,
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
            set_bit_perfect_enabled,
            // Lyrics
            parse_lrc,
            fetch_lrclib_lyrics,
            // Cover art cache
            get_cover_art,
            clear_cover_cache,
            // OpenSubsonic API
            set_connection,
            validate_connection,
            get_albums,
            get_artists,
            get_album_tracks,
            get_artist_details,
            get_artist_info,
            search,
            get_recent_albums,
            get_random_albums,
            get_newest_albums,
            get_genres_list,
            get_playlists,
            get_playlist_tracks,
            create_playlist,
            update_playlist,
            delete_playlist,
            get_open_subsonic_extensions,
            scrobble,
            report_playback,
            get_sonic_similar_tracks,
            find_sonic_path,
            get_song_lyrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
