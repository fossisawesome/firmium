pub mod cli;
mod commands;
mod events;

use commands::*;
use firmium_backend::core::Core;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Enter (not block_on) the runtime: the backend spawns tasks, but its
            // blocking HTTP client must not be built inside an async context.
            let _guard = tauri::async_runtime::handle().inner().enter();
            let core = Core::new()?;
            events::start(app.handle().clone(), &core.backend.bus);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_decorations(core.settings().window_decorations);
            }
            app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version, list_themes, get_auth_state, auto_login, connect, switch_account, logout, remove_account,
            get_settings, set_setting, reset_settings, get_server_extensions,
            get_albums, get_artists, get_album_tracks, get_artist_details, get_artist_info, get_similar_artists, get_artist_image, play_artist, get_artist_recommendations, search,
            get_album_list, get_genres, get_songs_by_genre, get_lyrics, get_similar_tracks, get_cover, get_cover_colors,
            clear_cover_cache, set_starred, get_starred, set_rating,
            set_queue, set_queue_seamless, append_and_play, shuffle_and_play, play_queue_index, queue_next, queue_prev,
            toggle_play, seek_queue, toggle_shuffle, set_repeat_mode, get_queue_state, generate_mix, start_radio,
            get_remote_play_queue, resume_remote_queue, list_audio_devices,
            get_eq_state, save_eq_profile, delete_eq_profile, set_eq_active_profile, set_eq_bands, set_eq_enabled,
            get_local_playlists, get_server_playlists, get_server_playlist_tracks, add_to_server_playlist, reorder_server_playlist, remove_from_server_playlist, create_playlist, sync_playlist_now,
            delete_playlist, rename_playlist, add_to_playlist, move_playlist_track, remove_playlist_track,
            download_track, download_album, get_local_album_track_keys, import_local_files, pick_and_import,
            get_recap, get_history_summary, weekly_recap_due, get_recent_plays, export_history, save_image,
            get_podcast_channels, get_podcast_episodes, add_podcast, refresh_podcast, unsubscribe_podcast, play_podcast_episode,
            set_window_decorations,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
