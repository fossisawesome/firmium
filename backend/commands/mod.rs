// Command modules: plain async/sync fns called from `core::Core` and termium. Callers use
// the full path (e.g. `crate::commands::subsonic::get_albums`).

pub mod themes;
pub mod mappers;
pub mod credentials;
pub mod auth;
pub mod app_info;
pub mod equalizer;
pub mod lyrics;
pub mod cover_cache;
pub mod cover_colors;
pub mod queue;
pub mod subsonic;
pub mod local_library;
pub mod downloads;
pub mod listenbrainz;
pub mod stats;
pub mod playlists;
