pub mod themes;
pub mod mappers;
pub mod credentials;
pub mod auth;
pub mod app_info;
pub mod playback;
pub mod equalizer;
pub mod lyrics;
pub mod cover_cache;
pub mod cover_colors;
pub mod queue;
pub mod subsonic;
pub mod local_library;
pub mod downloads;
pub mod listenbrainz;

pub use themes::list_themes;
pub use mappers::{map_albums, map_artists, map_songs};
pub use credentials::{save_password, get_password, delete_password};
pub use auth::generate_auth_params;
pub use app_info::get_app_version;
pub use lyrics::{parse_lrc, fetch_lrclib_lyrics};
pub use cover_cache::{get_cover_art, clear_cover_cache};
pub use cover_colors::{extract_cover_colors, extract_cover_colors_from_path};
pub use queue::{
    init_playback_settings, set_queue, set_queue_seamless, shuffle_and_play, play_queue_index,
    queue_next, queue_prev, toggle_play, seek_queue, set_queue_volume,
    set_repeat_mode, toggle_shuffle, set_crossfade_settings, set_gapless_enabled, set_replay_gain_enabled,
    set_auto_continue,
};
pub use subsonic::{
    set_connection, validate_connection, get_open_subsonic_extensions, get_albums, get_artists, get_album_tracks,
    get_artist_details, get_artist_info, search, get_recent_albums, get_random_albums,
    get_newest_albums, get_genres_list, get_playlists, get_playlist_tracks, create_playlist,
    update_playlist, delete_playlist, scrobble, report_playback,
    save_play_queue, get_play_queue,
    get_sonic_similar_tracks, find_sonic_path, get_similar_tracks_fallback, get_song_lyrics,
    get_songs_by_genre, get_random_songs, get_similar_artists,
};
pub use playback::{
    play_stream, preload_stream, pause_playback, resume_playback, stop_playback,
    set_volume, get_volume, get_playback_state, is_playback_finished, get_track_duration,
    get_current_position, seek_position, list_audio_devices, crossfade_to,
    set_visualizer_enabled, set_bit_perfect_mode,
};
pub use local_library::{
    get_local_albums, get_local_artists, get_local_album_tracks, get_local_album_track_keys, get_local_artist_details,
    get_local_cover_art, get_local_track_path, search_local, get_local_recent_albums,
    get_local_random_albums, get_local_newest_albums, get_local_genres_list,
    import_local_files, find_local_match, prewarm_local_library,
};
pub use downloads::{download_track, download_album};
pub use equalizer::{
    get_eq_state, save_eq_profile, delete_eq_profile, set_eq_active_profile,
    set_eq_bands, set_eq_enabled,
};
