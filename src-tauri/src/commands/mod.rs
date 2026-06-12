pub mod themes;
pub mod mappers;
pub mod credentials;
pub mod auth;
pub mod app_info;
pub mod playback;

pub use themes::list_themes;
pub use mappers::{map_albums, map_artists, map_songs};
pub use credentials::{save_password, get_password, delete_password};
pub use auth::generate_auth_params;
pub use app_info::get_app_version;
pub use playback::{
    play_stream, preload_stream, pause_playback, resume_playback, stop_playback,
    set_volume, get_volume, get_playback_state, is_playback_finished, get_track_duration,
    get_current_position, seek_position, list_audio_devices, crossfade_to,
    set_bit_perfect_enabled,
};
