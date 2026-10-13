//! Local-first playlists persisted at `<config>/playlists.json`. Each playlist
//! is created locally and best-effort synced to the OpenSubsonic server.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::commands::mappers::Song;

/// Max server-create retry attempts before auto-retry stops for a local playlist.
pub const CREATE_ATTEMPT_CAP: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    /// Absolute path of a user-chosen cover image, copied under `<config>/playlist_covers`.
    #[serde(default)]
    pub custom_cover: Option<String>,
}

fn default_true() -> bool {
    true
}

fn playlists_path() -> PathBuf {
    crate::paths::config_dir().join("playlists.json")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Local playlists plus server playlists not linked to any local one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistLists {
    pub local: Vec<Playlist>,
    pub server_only: Vec<serde_json::Value>,
}

pub fn load() -> Vec<Playlist> {
    crate::paths::read_or_backup(&playlists_path(), |s| serde_json::from_str(s).ok()).unwrap_or_default()
}

pub fn save(list: &[Playlist]) {
    if let Ok(s) = serde_json::to_string_pretty(list) {
        if let Err(e) = crate::paths::write_atomic(&playlists_path(), &s) {
            eprintln!("Playlists save failed: {e}");
        }
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
        custom_cover: None,
    }
}

/// Appends `songs` to playlist `id`, skipping duplicates by song id.
/// Returns the ids actually added (for the server push).
pub fn add_tracks(list: &mut [Playlist], id: &str, songs: Vec<Song>) -> Vec<String> {
    let Some(p) = list.iter_mut().find(|p| p.id == id) else {
        return Vec::new();
    };
    let mut seen: std::collections::HashSet<String> = p.tracks.iter().map(|s| s.id.clone()).collect();
    let new_songs: Vec<Song> = songs.into_iter().filter(|s| seen.insert(s.id.clone())).collect();
    let new_ids = new_songs.iter().map(|s| s.id.clone()).collect();
    p.tracks.extend(new_songs);
    new_ids
}

/// Moves a track; returns the new ordered id list when a move happened.
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

/// Removes a track by song id. Returns its former index.
pub fn remove_track(list: &mut [Playlist], id: &str, track_id: &str) -> Option<usize> {
    let p = list.iter_mut().find(|p| p.id == id)?;
    let idx = p.tracks.iter().position(|s| s.id == track_id)?;
    p.tracks.remove(idx);
    Some(idx)
}

// ── Core operations: mutate the local list, persist, then push to the server
// best-effort. The UI gets the updated list back from each call.

use std::sync::Arc;

use super::Core;
use crate::commands::playlists as sync;

impl Core {
    pub fn local_playlists(&self) -> Vec<Playlist> {
        self.playlists.lock().clone()
    }

    fn server_id_of(&self, local_id: &str) -> Option<String> {
        self.playlists.lock().iter().find(|p| p.id == local_id).and_then(|p| p.server_id.clone())
    }

    async fn sync_new(self: &Arc<Self>, local_id: String, name: String, track_ids: Vec<String>) {
        let res = sync::sync_create(self.backend.app_state.clone(), name, track_ids).await;
        let mut list = self.playlists.lock();
        if let Some(p) = list.iter_mut().find(|p| p.id == local_id) {
            match res {
                Ok(pl) => {
                    p.server_id = pl.get("id").and_then(|v| v.as_str()).map(String::from);
                    p.create_pending = false;
                }
                Err(e) => {
                    eprintln!("playlist create sync failed: {e:?}");
                    p.create_attempts += 1;
                    p.create_pending = p.create_attempts < CREATE_ATTEMPT_CAP;
                }
            }
        }
        save(&list);
    }

    pub async fn create_playlist(self: &Arc<Self>, name: String, first_track: Option<Song>) -> Vec<Playlist> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return self.local_playlists();
        }
        let mut p = new_local(name.clone());
        if let Some(s) = first_track {
            p.tracks.push(s);
        }
        let id = p.id.clone();
        let ids: Vec<String> = p.tracks.iter().map(|s| s.id.clone()).collect();
        {
            let mut list = self.playlists.lock();
            list.insert(0, p);
            save(&list);
        }
        self.sync_new(id, name, ids).await;
        self.local_playlists()
    }

    /// Adopts same-named server playlists for unsynced locals, retries the rest,
    /// and returns the local list plus server playlists no local entry claims.
    pub async fn reconcile_playlists(self: &Arc<Self>, server: Vec<serde_json::Value>) -> PlaylistLists {
        let mut retry = Vec::new();
        {
            let mut list = self.playlists.lock();
            for local in list.iter_mut() {
                if local.server_id.is_some() || !local.create_pending || local.create_attempts >= CREATE_ATTEMPT_CAP {
                    continue;
                }
                let same = server.iter().find(|sp| sp.get("name").and_then(|v| v.as_str()) == Some(local.name.as_str()));
                if let Some(sp) = same {
                    local.server_id = sp.get("id").and_then(|v| v.as_str()).map(String::from);
                    local.create_pending = false;
                } else {
                    retry.push((local.id.clone(), local.name.clone(), local.tracks.iter().map(|s| s.id.clone()).collect::<Vec<_>>()));
                }
            }
            save(&list);
        }
        for (id, name, ids) in retry {
            self.sync_new(id, name, ids).await;
        }
        let local = self.local_playlists();
        let server_only = server
            .into_iter()
            .filter(|sp| {
                let id = sp.get("id").and_then(|v| v.as_str());
                !local.iter().any(|l| l.server_id.as_deref() == id)
            })
            .collect();
        PlaylistLists { local, server_only }
    }

    pub async fn add_to_server_playlist(&self, server_id: String, song_ids: Vec<String>) {
        sync::push_add(self.backend.app_state.clone(), server_id, song_ids).await;
    }

    pub async fn reorder_server_playlist(&self, server_id: String, ordered_ids: Vec<String>) {
        sync::push_reorder(self.backend.app_state.clone(), server_id, ordered_ids).await;
    }

    pub async fn remove_from_server_playlist(&self, server_id: String, index: u32) {
        sync::push_remove(self.backend.app_state.clone(), server_id, index).await;
    }

    pub async fn sync_playlist_now(self: &Arc<Self>, local_id: String) -> Vec<Playlist> {
        let job = self.playlists.lock().iter().find(|p| p.id == local_id && p.server_id.is_none()).map(|p| {
            (p.name.clone(), p.tracks.iter().map(|s| s.id.clone()).collect::<Vec<_>>())
        });
        if let Some((name, ids)) = job {
            self.sync_new(local_id, name, ids).await;
        }
        self.local_playlists()
    }

    pub async fn delete_local_playlist(&self, local_id: &str) -> Vec<Playlist> {
        let sid = self.server_id_of(local_id);
        {
            let mut list = self.playlists.lock();
            list.retain(|p| p.id != local_id);
            save(&list);
        }
        if let Some(sid) = sid {
            sync::push_delete(self.backend.app_state.clone(), sid).await;
        }
        self.local_playlists()
    }

    pub async fn rename_local_playlist(&self, local_id: &str, name: String) -> Vec<Playlist> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return self.local_playlists();
        }
        let sid = {
            let mut list = self.playlists.lock();
            let sid = list.iter_mut().find(|p| p.id == local_id).map(|p| {
                p.name = name.clone();
                p.server_id.clone()
            });
            save(&list);
            sid.flatten()
        };
        if let Some(sid) = sid {
            sync::push_rename(self.backend.app_state.clone(), sid, name).await;
        }
        self.local_playlists()
    }

    /// Copies `src` into the config dir as this playlist's cover; `None` clears it.
    pub fn set_local_playlist_cover(&self, local_id: &str, src: Option<PathBuf>) -> Result<Vec<Playlist>, String> {
        let dest = match src {
            Some(src) => {
                let dir = crate::paths::config_dir().join("playlist_covers");
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png");
                // Unique name per pick so the webview never serves a cached older image.
                let dest = dir.join(format!("{local_id}-{}.{ext}", now_ms()));
                std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
                Some(dest.to_string_lossy().into_owned())
            }
            None => None,
        };
        let mut list = self.playlists.lock();
        if let Some(p) = list.iter_mut().find(|p| p.id == local_id) {
            if let Some(old) = p.custom_cover.take() {
                let _ = std::fs::remove_file(old);
            }
            p.custom_cover = dest;
        }
        save(&list);
        Ok(list.clone())
    }

    pub async fn add_to_local_playlist(&self, local_id: &str, song: Song) -> Vec<Playlist> {
        let added = {
            let mut list = self.playlists.lock();
            let added = add_tracks(&mut list, local_id, vec![song]);
            save(&list);
            added
        };
        if let Some(sid) = self.server_id_of(local_id) {
            sync::push_add(self.backend.app_state.clone(), sid, added).await;
        }
        self.local_playlists()
    }

    pub async fn move_local_track(&self, local_id: &str, from: usize, to: usize) -> Vec<Playlist> {
        let ordered = {
            let mut list = self.playlists.lock();
            let o = move_track(&mut list, local_id, from, to);
            save(&list);
            o
        };
        if let (Some(sid), Some(ids)) = (self.server_id_of(local_id), ordered) {
            sync::push_reorder(self.backend.app_state.clone(), sid, ids).await;
        }
        self.local_playlists()
    }

    pub async fn remove_local_track(&self, local_id: &str, track_id: &str) -> Vec<Playlist> {
        let idx = {
            let mut list = self.playlists.lock();
            let i = remove_track(&mut list, local_id, track_id);
            save(&list);
            i
        };
        if let (Some(sid), Some(i)) = (self.server_id_of(local_id), idx) {
            sync::push_remove(self.backend.app_state.clone(), sid, i as u32).await;
        }
        self.local_playlists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(id: &str) -> Song {
        serde_json::from_value(serde_json::json!({ "id": id, "title": "", "artist": "", "album": "", "duration": 0.0, "starred": false })).unwrap_or_else(|_| panic!("song fixture"))
    }

    #[test]
    fn add_skips_duplicates_and_move_reorders() {
        let mut list = vec![new_local("a".into())];
        let id = list[0].id.clone();
        let added = add_tracks(&mut list, &id, vec![song("1"), song("2"), song("1")]);
        assert_eq!(added, vec!["1", "2"]);
        assert_eq!(move_track(&mut list, &id, 0, 1), Some(vec!["2".into(), "1".into()]));
        assert_eq!(remove_track(&mut list, &id, "2"), Some(0));
    }
}
