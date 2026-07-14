//! Local-first playlists persisted at `~/.config/<id>/playlists.json`.
//! Mirrors Android's PlaylistRepository data model: each playlist is created
//! locally and best-effort synced to the OpenSubsonic server. The whole list is
//! stored as a JSON array; server pushes happen separately in `App::update`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use firmium_backend::commands::mappers::Song;

/// Max server-create retry attempts before we stop auto-retrying a local playlist.
pub const CREATE_ATTEMPT_CAP: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tracks: Vec<Song>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(default = "default_true")]
    pub create_pending: bool,
    #[serde(default)]
    pub create_attempts: u32,
}

fn default_true() -> bool {
    true
}

fn playlists_path() -> PathBuf {
    firmium_backend::paths::config_dir().join("playlists.json")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn load_playlists() -> Vec<Playlist> {
    std::fs::read_to_string(playlists_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_playlists(list: &[Playlist]) {
    if let Ok(s) = serde_json::to_string_pretty(list) {
        let _ = std::fs::write(playlists_path(), s);
    }
}

pub fn new_local(name: String) -> Playlist {
    Playlist {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        tracks: Vec::new(),
        created_at: now_ms(),
        server_id: None,
        create_pending: true,
        create_attempts: 0,
    }
}

/// Appends `songs` to playlist `id`, skipping duplicates by song id.
/// Returns the ids of songs actually added (for the server push).
pub fn add_tracks(list: &mut [Playlist], id: &str, songs: Vec<Song>) -> Vec<String> {
    let Some(p) = list.iter_mut().find(|p| p.id == id) else {
        return Vec::new();
    };
    let existing: std::collections::HashSet<&str> = p.tracks.iter().map(|s| s.id.as_str()).collect();
    let new_songs: Vec<Song> = songs
        .into_iter()
        .filter(|s| !existing.contains(s.id.as_str()))
        .collect();
    let new_ids: Vec<String> = new_songs.iter().map(|s| s.id.clone()).collect();
    p.tracks.extend(new_songs);
    new_ids
}

/// Moves a track within playlist `id`. Returns the full new ordered id list when
/// a move happened (so the caller can re-push order to the server).
pub fn move_track(list: &mut [Playlist], id: &str, from: usize, to: usize) -> Option<Vec<String>> {
    let p = list.iter_mut().find(|p| p.id == id)?;
    let n = p.tracks.len();
    if from >= n || to >= n || from == to {
        return None;
    }
    let moved = p.tracks.remove(from);
    p.tracks.insert(to, moved);
    Some(p.tracks.iter().map(|s| s.id.clone()).collect())
}

/// Removes a track by song id from playlist `id`. Returns its former index.
pub fn remove_track(list: &mut [Playlist], id: &str, track_id: &str) -> Option<usize> {
    let p = list.iter_mut().find(|p| p.id == id)?;
    let idx = p.tracks.iter().position(|s| s.id == track_id)?;
    p.tracks.remove(idx);
    Some(idx)
}
