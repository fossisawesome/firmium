//! Best-effort server-sync primitives for local-first playlists. The in-memory
//! `Vec<Playlist>` lives on `App`; these fns only talk to the OpenSubsonic server
//! and are spawned via `iced::Task::perform`. Failures are surfaced to the caller
//! (create) or swallowed/logged (the push_* helpers), never blocking the UI.

use std::sync::Arc;

use crate::commands::subsonic;
use crate::errors::UserError;
use crate::state::AppState;

/// Creates the playlist on the server and adds its current tracks. Returns the
/// raw server playlist object (its `"id"` is the new server id).
pub async fn sync_create(
    state: Arc<AppState>,
    name: String,
    track_ids: Vec<String>,
) -> Result<serde_json::Value, UserError> {
    let pl = subsonic::create_playlist(state.clone(), name).await?;
    if !track_ids.is_empty() {
        if let Some(id) = pl.get("id").and_then(|v| v.as_str()) {
            subsonic::update_playlist(state, id.to_string(), None, None, track_ids, Vec::new()).await?;
        }
    }
    Ok(pl)
}

pub async fn push_rename(state: Arc<AppState>, server_id: String, name: String) {
    if let Err(e) =
        subsonic::update_playlist(state, server_id, Some(name), None, Vec::new(), Vec::new()).await
    {
        eprintln!("playlist rename sync failed (best-effort): {e:?}");
    }
}

pub async fn push_delete(state: Arc<AppState>, server_id: String) {
    if let Err(e) = subsonic::delete_playlist(state, server_id).await {
        eprintln!("playlist delete sync failed (best-effort): {e:?}");
    }
}

pub async fn push_add(state: Arc<AppState>, server_id: String, song_ids: Vec<String>) {
    if song_ids.is_empty() {
        return;
    }
    if let Err(e) =
        subsonic::update_playlist(state, server_id, None, None, song_ids, Vec::new()).await
    {
        eprintln!("playlist add-tracks sync failed (best-effort): {e:?}");
    }
}

pub async fn push_remove(state: Arc<AppState>, server_id: String, index: u32) {
    if let Err(e) =
        subsonic::update_playlist(state, server_id, None, None, Vec::new(), vec![index]).await
    {
        eprintln!("playlist remove-track sync failed (best-effort): {e:?}");
    }
}

/// Re-pushes the full track order: OpenSubsonic has no native move, so remove all
/// original indices and re-add ids in the new order (mirrors Android moveTrack).
pub async fn push_reorder(state: Arc<AppState>, server_id: String, ordered_ids: Vec<String>) {
    let remove: Vec<u32> = (0..ordered_ids.len() as u32).collect();
    if let Err(e) =
        subsonic::update_playlist(state, server_id, None, None, ordered_ids, remove).await
    {
        eprintln!("playlist reorder sync failed (best-effort): {e:?}");
    }
}
