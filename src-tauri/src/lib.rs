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
mod visualizer;
mod visualizer_gpu;
mod commands;
use commands::*;
mod state;
use state::AppState;
mod queue_state;
use queue_state::QueueState;
mod queue_manager;
mod db;
use db::PlayHistory;

// ============================================================================
// APPLICATION ENTRY POINT
// ============================================================================

/// App entry point.
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init());

    builder
        .setup(move |_app| {
            let audio_player = Arc::new(
                AudioPlayer::new(_app.handle().clone()).expect("Failed to initialize audio player"),
            );
            let app_state = Arc::new(AppState::new());
            let queue_state = Arc::new(QueueState::new());

            queue_manager::start(
                _app.handle().clone(),
                Arc::clone(&queue_state),
                Arc::clone(&app_state),
                Arc::clone(&audio_player),
            );

            // Local play-history store. A failure here (e.g. unwritable data dir)
            // must not crash the app, so log and skip — stats features no-op without it.
            match PlayHistory::new(_app.handle()) {
                Ok(history) => { _app.manage(history); }
                Err(e) => eprintln!("Play history DB init failed: {e}"),
            }

            _app.manage(audio_player);
            _app.manage(app_state);
            _app.manage(queue_state);

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
            set_visualizer_enabled,
            set_visualizer_mode,
            set_visualizer_palette,
            start_visualizer_renderer,
            stop_visualizer_renderer,
            set_bit_perfect_mode,
            // Equalizer
            get_eq_state,
            save_eq_profile,
            delete_eq_profile,
            set_eq_active_profile,
            set_eq_bands,
            set_eq_enabled,
            // Lyrics
            parse_lrc,
            fetch_lrclib_lyrics,
            // Cover art cache
            get_cover_art,
            clear_cover_cache,
            extract_cover_colors,
            extract_cover_colors_from_path,
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
            set_rating,
            report_playback,
            save_play_queue,
            get_play_queue,
            get_sonic_similar_tracks,
            find_sonic_path,
            get_similar_tracks_fallback,
            get_songs_by_genre,
            get_random_songs,
            get_similar_artists,
            get_song_lyrics,
            // Local library
            get_local_albums,
            get_local_artists,
            get_local_album_tracks,
            get_local_album_track_keys,
            get_local_artist_details,
            get_local_cover_art,
            get_local_track_path,
            search_local,
            get_local_recent_albums,
            get_local_random_albums,
            get_local_newest_albums,
            get_local_genres_list,
            import_local_files,
            find_local_match,
            prewarm_local_library,
            // Downloads
            download_track,
            download_album,
            // Play history stats
            get_recap_stats,
            get_play_history_summary,
            export_play_history,
            save_text_file,
            save_binary_file,
            // Queue management
            init_playback_settings,
            set_queue,
            set_queue_seamless,
            append_and_play,
            shuffle_and_play,
            play_queue_index,
            queue_next,
            queue_prev,
            toggle_play,
            seek_queue,
            set_queue_volume,
            set_repeat_mode,
            toggle_shuffle,
            set_crossfade_settings,
            set_crossfade_curve,
            set_gapless_enabled,
            set_replay_gain_enabled,
            set_auto_continue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
