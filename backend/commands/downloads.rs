// ============================================================================
// DOWNLOADS
// ============================================================================
// Downloads tracks/albums from the connected OpenSubsonic server into the
// local library folder (`local_library_dir`), using the same folder layout
// as local-library imports: `<AlbumArtist>/<Album>/<TrackNum> - <Title>.<ext>`.
// After a successful download, invalidates the local-library scan cache so
// the new file shows up next time the local library is read.

use crate::commands::auth::generate_auth_params;
use crate::commands::local_library::{invalidate_local_library, local_library_dir};
use crate::commands::subsonic::get_album_tracks;
use crate::state::AppState;
use std::sync::Arc;

pub(crate) fn sanitize_path_component(name: &str) -> String {
    let cleaned: String = name.chars().map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() { "Unknown".to_string() } else { trimmed.to_string() }
}

#[allow(clippy::too_many_arguments)]
pub async fn download_track(
    state: Arc<AppState>,
    song_id: String,
    format: String,
    album_artist: String,
    album: String,
    title: String,
    track_number: Option<u32>,
    suffix: Option<String>,
) -> Result<(), String> {
    {
        let cache = state.local_library.read();
        if let Some(cache) = cache.as_ref() {
            if cache.has_local_match(&title, &album) {
                return Ok(());
            }
        }
    }

    let (server, username, password) = {
        let conn = state.connection.read();
        (
            conn.server.clone().ok_or("Not connected")?,
            conn.username.clone().unwrap_or_default(),
            conn.password.clone().unwrap_or_default(),
        )
    };

    let auth = generate_auth_params(username, password);
    let mut url = reqwest::Url::parse(&format!("{server}/rest/stream")).map_err(|e| e.to_string())?;
    {
        let mut query = url.query_pairs_mut();
        for key in ["u", "t", "s", "v", "c", "f"] {
            query.append_pair(key, auth[key].as_str().unwrap_or(""));
        }
        query.append_pair("id", &song_id);
        query.append_pair("format", &format);
    }

    let res = state.http.get(url).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("HTTP error {}", res.status()));
    }

    let is_json = res.headers().get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.contains("application/json"));

    if is_json {
        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let msg = json.get("subsonic-response")
            .and_then(|b| b.get("error"))
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Download failed");
        return Err(msg.to_string());
    }

    let ext = if format == "raw" { suffix.unwrap_or_else(|| "mp3".to_string()) } else { format };
    let file_stem = match track_number {
        Some(n) => format!("{n:02} - {title}"),
        None => title,
    };
    let file_name = format!("{}.{}", sanitize_path_component(&file_stem), ext);

    let dir = local_library_dir()
        .join(sanitize_path_component(&album_artist))
        .join(sanitize_path_component(&album));
    let path = dir.join(file_name);
    eprintln!("Downloading track to {}", path.display());

    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&path, &bytes)
    }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;

    invalidate_local_library(&state);
    Ok(())
}

pub async fn download_album(state: Arc<AppState>, album_id: String, format: String) -> Result<(), String> {
    let album_tracks = get_album_tracks(state.clone(), album_id).await?;
    for track in album_tracks.tracks {
        download_track(
            state.clone(),
            track.id,
            format.clone(),
            album_tracks.album_artist.clone(),
            album_tracks.album_name.clone(),
            track.title,
            track.track_number,
            track.suffix,
        ).await?;
    }
    Ok(())
}
