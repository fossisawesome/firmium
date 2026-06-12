pub mod themes;
pub mod mappers;
pub mod credentials;
pub mod auth;
pub mod app_info;
pub mod playback;
pub mod lyrics;
pub mod cover_cache;
pub mod subsonic;

pub use themes::list_themes;
pub use mappers::{map_albums, map_artists, map_songs};
pub use credentials::{save_password, get_password, delete_password};
pub use auth::generate_auth_params;
pub use app_info::get_app_version;
pub use lyrics::{parse_lrc, fetch_lrclib_lyrics};
pub use cover_cache::{get_cover_art, clear_cover_cache};
pub use subsonic::{
    set_connection, validate_connection, get_albums, get_artists, get_album_tracks,
    get_artist_details, get_artist_info, search, get_recent_albums, get_random_albums,
    get_newest_albums, get_genres_list, get_playlists, get_playlist_tracks, create_playlist,
    update_playlist, delete_playlist, scrobble, get_song_lyrics,
};
pub use playback::{
    play_stream, preload_stream, pause_playback, resume_playback, stop_playback,
    set_volume, get_volume, get_playback_state, is_playback_finished, get_track_duration,
    get_current_position, seek_position, list_audio_devices, crossfade_to,
    set_bit_perfect_enabled,
};
