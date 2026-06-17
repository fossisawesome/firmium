// ============================================================================
// OPENSUBSONIC API COMMANDS
// ============================================================================
// Read-only OpenSubsonic endpoints. Connection details (server/username/
// password) are held in `AppState.connection` and set via `set_connection`.

use crate::commands::auth::generate_auth_params;
use crate::commands::lyrics::{fetch_lrclib_lyrics, LyricLine, LyricsResult};
use crate::commands::mappers::{map_albums, map_artists, map_similar_matches, map_songs, Album, Artist, SimilarMatch, Song};
use crate::state::{AppState, ConnectionState};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

const SESSION_EXPIRED: &str = "SESSION_EXPIRED";

/// Performs an authenticated GET against the connected OpenSubsonic server and
/// returns the parsed `subsonic-response` body. On HTTP 401 or OpenSubsonic
/// error codes 40/41, emits `firmium:session-expired` (unless `silent`) and
/// returns `Err(SESSION_EXPIRED)`.
async fn subsonic_request(
    app: &AppHandle,
    state: &AppState,
    action: &str,
    params: &[(&str, String)],
    silent: bool,
) -> Result<serde_json::Value, String> {
    let (server, username, password) = {
        let conn = state.connection.read();
        (
            conn.server.clone().ok_or("Not connected")?,
            conn.username.clone().unwrap_or_default(),
            conn.password.clone().unwrap_or_default(),
        )
    };

    let auth = generate_auth_params(username, password);
    let mut url = reqwest::Url::parse(&format!("{server}/rest/{action}")).map_err(|e| e.to_string())?;
    {
        let mut query = url.query_pairs_mut();
        for key in ["u", "t", "s", "v", "c", "f"] {
            query.append_pair(key, auth[key].as_str().unwrap_or(""));
        }
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }

    let res = state.http.get(url).send().await.map_err(|e| e.to_string())?;
    if res.status() == reqwest::StatusCode::UNAUTHORIZED {
        if !silent {
            let _ = app.emit("firmium:session-expired", ());
        }
        return Err(SESSION_EXPIRED.to_string());
    }
    if !res.status().is_success() {
        return Err(format!("HTTP Error {}", res.status()));
    }

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let body = json.get("subsonic-response").ok_or("Malformed API response")?.clone();

    if let Some(ext) = body.get("openSubsonicExtensions") {
        state.connection.write().open_subsonic_extensions = ext.as_array().map(|arr| {
            arr.iter().filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string)).collect()
        });
    }

    if body.get("status").and_then(|v| v.as_str()) == Some("failed") {
        let code = body.get("error").and_then(|e| e.get("code")).and_then(|c| c.as_u64());
        if code == Some(40) || code == Some(41) {
            if !silent {
                let _ = app.emit("firmium:session-expired", ());
            }
            return Err(SESSION_EXPIRED.to_string());
        }
        let msg = body.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or("Engine error");
        return Err(msg.to_string());
    }

    Ok(body)
}

/// Checks whether the connected server has advertised the given OpenSubsonic extension.
fn has_extension(state: &AppState, name: &str) -> bool {
    state.connection.read().open_subsonic_extensions.as_deref().unwrap_or(&[]).iter().any(|e| e == name)
}

/// Returns the OpenSubsonic extensions advertised by the connected server, as
/// detected from the most recent API response.
#[tauri::command]
pub fn get_open_subsonic_extensions(state: State<'_, Arc<AppState>>) -> Vec<String> {
    state.connection.read().open_subsonic_extensions.clone().unwrap_or_default()
}

fn array_field(body: &serde_json::Value, path: &[&str]) -> Vec<serde_json::Value> {
    let mut cur = body;
    for key in path {
        match cur.get(key) {
            Some(v) => cur = v,
            None => return Vec::new(),
        }
    }
    // Some servers return a single object instead of a one-element array when
    // a collection (e.g. playlists, playlist entries) contains exactly one item.
    match cur {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(_) => vec![cur.clone()],
        _ => Vec::new(),
    }
}

// ── Connection ───────────────────────────────────────────────────────────────

/// Stores the active connection's credentials. Called from `stores.ts`
/// whenever `setAuth`/`clearAuth` runs (login, logout, session expiry).
#[tauri::command]
pub fn set_connection(state: State<'_, Arc<AppState>>, server: Option<String>, username: Option<String>, password: Option<String>) {
    let mut conn = state.connection.write();
    conn.server = server;
    conn.username = username;
    conn.password = password;
    conn.open_subsonic_extensions = None;
}

/// Validates credentials with a minimal request, used during the initial
/// login flow. Does not emit `firmium:session-expired` on failure, since a
/// rejected login isn't an expired session.
#[tauri::command]
pub async fn validate_connection(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    match subsonic_request(&app, &state, "getAlbumList2", &[("type", "alphabeticalByName".to_string()), ("size", "1".to_string())], true).await {
        Ok(_) => {
            // openSubsonicExtensions is only included on this dedicated endpoint,
            // not on regular responses like getAlbumList2.
            let _ = subsonic_request(&app, &state, "getOpenSubsonicExtensions", &[], true).await;
            Ok(())
        }
        Err(e) => {
            *state.connection.write() = ConnectionState::default();
            Err(e)
        }
    }
}

// ── Reads ────────────────────────────────────────────────────────────────────

const API_PAGE_SIZE: &str = "500";

#[tauri::command]
pub async fn get_albums(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Vec<Album>, String> {
    let body = subsonic_request(&app, &state, "getAlbumList2", &[("type", "alphabeticalByName".to_string()), ("size", API_PAGE_SIZE.to_string())], false).await?;
    Ok(map_albums(array_field(&body, &["albumList2", "album"])))
}

#[tauri::command]
pub async fn get_artists(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Vec<Artist>, String> {
    let body = subsonic_request(&app, &state, "getArtists", &[], false).await?;
    let mut raw = Vec::new();
    for group in array_field(&body, &["artists", "index"]) {
        if let Some(artists) = group.get("artist").and_then(|v| v.as_array()) {
            raw.extend(artists.iter().cloned());
        }
    }
    Ok(map_artists(raw))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumTracks {
    pub tracks: Vec<Song>,
    pub album_name: String,
    pub album_artist: String,
    pub cover_art_id: Option<String>,
}

#[tauri::command]
pub async fn get_album_tracks(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Result<AlbumTracks, String> {
    let body = subsonic_request(&app, &state, "getAlbum", &[("id", id)], false).await?;
    let album = body.get("album").cloned().unwrap_or(serde_json::Value::Null);
    let tracks = map_songs(array_field(&album, &["song"]));
    Ok(AlbumTracks {
        tracks,
        album_name: album.get("name").or_else(|| album.get("title")).and_then(|v| v.as_str()).unwrap_or("Unknown Album").to_string(),
        album_artist: album.get("displayArtist").or_else(|| album.get("artist")).and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        cover_art_id: album.get("coverArt").and_then(|v| v.as_str()).map(str::to_string),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetails {
    name: String,
    albums: Vec<Album>,
}

#[tauri::command]
pub async fn get_artist_details(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Result<ArtistDetails, String> {
    let body = subsonic_request(&app, &state, "getArtist", &[("id", id)], false).await?;
    let artist = body.get("artist").cloned().unwrap_or(serde_json::Value::Null);
    Ok(ArtistDetails {
        name: artist.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        albums: map_albums(array_field(&artist, &["album"])),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo {
    image: Option<String>,
    bio: Option<String>,
}

/// Returns artist info (bio + image) from Last.fm/MusicBrainz via the
/// server's getArtistInfo2 endpoint, or `None` if the server has no info.
#[tauri::command]
pub async fn get_artist_info(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Result<Option<ArtistInfo>, String> {
    let body = match subsonic_request(&app, &state, "getArtistInfo2", &[("id", id)], false).await {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let info = body.get("artistInfo2").cloned().unwrap_or(serde_json::Value::Null);
    let image = ["largeImageUrl", "mediumImageUrl", "smallImageUrl"]
        .iter()
        .find_map(|k| info.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_string));
    let bio = info.get("biography").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_string);
    Ok(Some(ArtistInfo { image, bio }))
}

#[tauri::command]
pub async fn search(app: tauri::AppHandle, state: State<'_, Arc<AppState>>, query: String) -> Result<SearchResult, String> {
    let body = subsonic_request(&app, &state, "search3", &[("query", query), ("albumCount", "40".to_string()), ("songCount", "100".to_string())], false).await?;
    Ok(SearchResult {
        songs: map_songs(array_field(&body, &["searchResult3", "song"])),
        albums: map_albums(array_field(&body, &["searchResult3", "album"])),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    songs: Vec<Song>,
    albums: Vec<Album>,
}

async fn fetch_album_list(app: &AppHandle, state: &AppState, list_type: &str, size: u32) -> Result<Vec<Album>, String> {
    let body = subsonic_request(app, state, "getAlbumList2", &[("type", list_type.to_string()), ("size", size.to_string())], false).await?;
    Ok(map_albums(array_field(&body, &["albumList2", "album"])))
}

#[tauri::command]
pub async fn get_recent_albums(app: AppHandle, state: State<'_, Arc<AppState>>, size: u32) -> Result<Vec<Album>, String> {
    fetch_album_list(&app, &state, "recent", size).await
}

#[tauri::command]
pub async fn get_random_albums(app: AppHandle, state: State<'_, Arc<AppState>>, size: u32) -> Result<Vec<Album>, String> {
    fetch_album_list(&app, &state, "random", size).await
}

#[tauri::command]
pub async fn get_newest_albums(app: AppHandle, state: State<'_, Arc<AppState>>, size: u32) -> Result<Vec<Album>, String> {
    fetch_album_list(&app, &state, "newest", size).await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    name: String,
    album_count: u32,
    song_count: u32,
}

#[tauri::command]
pub async fn get_genres_list(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Vec<Genre>, String> {
    let body = subsonic_request(&app, &state, "getGenres", &[], false).await?;
    let mut genres: Vec<Genre> = array_field(&body, &["genres", "genre"])
        .iter()
        .filter_map(|g| {
            let name = g.get("value").or_else(|| g.get("name")).and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                return None;
            }
            Some(Genre {
                name: name.to_string(),
                album_count: g.get("albumCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                song_count: g.get("songCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            })
        })
        .collect();
    genres.sort_by_key(|g| std::cmp::Reverse(g.album_count));
    Ok(genres)
}

// ── Playlists ────────────────────────────────────────────────────────────────

/// Returns all playlists visible to the current user.
#[tauri::command]
pub async fn get_playlists(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Vec<serde_json::Value>, String> {
    let body = subsonic_request(&app, &state, "getPlaylists", &[], false).await?;
    Ok(array_field(&body, &["playlists", "playlist"]))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTracks {
    id: String,
    name: String,
    comment: String,
    song_count: Option<u32>,
    tracks: Vec<Song>,
}

/// Fetches a playlist's full track list from the server.
#[tauri::command]
pub async fn get_playlist_tracks(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Result<PlaylistTracks, String> {
    let body = subsonic_request(&app, &state, "getPlaylist", &[("id", id)], false).await?;
    let playlist = body.get("playlist").cloned().unwrap_or(serde_json::Value::Null);
    Ok(PlaylistTracks {
        id: playlist.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        name: playlist.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        comment: playlist.get("comment").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        song_count: playlist.get("songCount").and_then(|v| v.as_u64()).map(|n| n as u32),
        tracks: map_songs(array_field(&playlist, &["entry"])),
    })
}

/// Creates a new playlist on the server and returns the created playlist object.
#[tauri::command]
pub async fn create_playlist(app: AppHandle, state: State<'_, Arc<AppState>>, name: String) -> Result<serde_json::Value, String> {
    let body = subsonic_request(&app, &state, "createPlaylist", &[("name", name)], false).await?;
    Ok(body.get("playlist").cloned().unwrap_or(serde_json::Value::Null))
}

/// Updates playlist metadata and/or adds/removes tracks by server-side ID/index.
#[tauri::command]
pub async fn update_playlist(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
    name: Option<String>,
    comment: Option<String>,
    song_ids_to_add: Vec<String>,
    song_indices_to_remove: Vec<u32>,
) -> Result<(), String> {
    let mut params = vec![("playlistId", id)];
    if let Some(name) = name {
        params.push(("name", name));
    }
    if let Some(comment) = comment {
        params.push(("comment", comment));
    }
    for song_id in song_ids_to_add {
        params.push(("songIdToAdd", song_id));
    }
    for index in song_indices_to_remove {
        params.push(("songIndexToRemove", index.to_string()));
    }
    subsonic_request(&app, &state, "updatePlaylist", &params, false).await?;
    Ok(())
}

/// Deletes a playlist from the server.
#[tauri::command]
pub async fn delete_playlist(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    subsonic_request(&app, &state, "deletePlaylist", &[("id", id)], false).await?;
    Ok(())
}

// ── Scrobble ─────────────────────────────────────────────────────────────────

// ── Internal helpers for queue_manager / queue commands ─────────────────────

/// Builds an authenticated `stream` URL for a given track ID without making
/// an HTTP request. Used by the Rust queue manager when starting playback.
pub(crate) fn build_stream_url(state: &AppState, track_id: &str) -> Result<String, String> {
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
        query.append_pair("id", track_id);
    }
    Ok(url.to_string())
}

/// Fire-and-forget scrobble, callable from Rust without going through the Tauri command layer.
pub(crate) fn fire_scrobble(app: AppHandle, state: Arc<AppState>, id: String, submission: bool) {
    tauri::async_runtime::spawn(async move {
        let params = [("id", id), ("submission", submission.to_string()), ("time", "0".to_string())];
        if let Err(e) = subsonic_request(&app, &state, "scrobble", &params, true).await {
            eprintln!("Scrobble failed: {e}");
        }
    });
}

/// Fire-and-forget playback report, callable from Rust without going through the Tauri command layer.
pub(crate) fn fire_report_playback(app: AppHandle, state: Arc<AppState>, media_id: String, position_ms: i64, playback_state: String) {
    if !has_extension(&state, "playbackReport") { return; }
    tauri::async_runtime::spawn(async move {
        let params = [
            ("mediaId", media_id),
            ("mediaType", "song".to_string()),
            ("positionMs", position_ms.to_string()),
            ("state", playback_state),
        ];
        if let Err(e) = subsonic_request(&app, &state, "reportPlayback", &params, true).await {
            eprintln!("Report playback failed: {e}");
        }
    });
}

/// Fire-and-forget save-play-queue, callable from Rust without going through the Tauri command layer.
pub(crate) fn fire_save_play_queue(app: AppHandle, state: Arc<AppState>, ids: Vec<String>, current: Option<String>, position_ms: Option<i64>) {
    tauri::async_runtime::spawn(async move {
        let mut params: Vec<(&str, String)> = ids.into_iter().map(|id| ("id", id)).collect();
        if let Some(c) = current { params.push(("current", c)); }
        if let Some(p) = position_ms { params.push(("position", p.to_string())); }
        if let Err(e) = subsonic_request(&app, &state, "savePlayQueue", &params, true).await {
            eprintln!("Save play queue failed: {e}");
        }
    });
}

/// Reports playback progress to the server. Fire-and-forget: errors are logged,
/// not surfaced, and never trigger a session-expiry prompt.
#[tauri::command]
pub fn scrobble(app: AppHandle, state: State<'_, Arc<AppState>>, id: String, submission: bool, time: i64) {
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let params = [("id", id), ("submission", submission.to_string()), ("time", time.to_string())];
        if let Err(e) = subsonic_request(&app, &state, "scrobble", &params, true).await {
            eprintln!("Scrobble failed: {e}");
        }
    });
}

/// Reports playback state/position via the `playbackReport` OpenSubsonic extension
/// (`reportPlayback`). No-op if the server hasn't advertised the extension.
/// Fire-and-forget, like `scrobble`.
#[tauri::command]
pub fn report_playback(app: AppHandle, state: State<'_, Arc<AppState>>, media_id: String, position_ms: i64, playback_state: String) {
    let state = state.inner().clone();
    if has_extension(&state, "playbackReport") {
        tauri::async_runtime::spawn(async move {
            let params = [
                ("mediaId", media_id),
                ("mediaType", "song".to_string()),
                ("positionMs", position_ms.to_string()),
                ("state", playback_state),
            ];
            if let Err(e) = subsonic_request(&app, &state, "reportPlayback", &params, true).await {
                eprintln!("Report playback failed: {e}");
            }
        });
    }
}

// ── Play Queue (cross-device continue) ──────────────────────────────────────

/// Saves the current queue/position to the server via `savePlayQueue`, so it can be
/// resumed on another device. Fire-and-forget, like `scrobble`.
#[tauri::command]
pub fn save_play_queue(app: AppHandle, state: State<'_, Arc<AppState>>, ids: Vec<String>, current: Option<String>, position_ms: Option<i64>) {
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let mut params: Vec<(&str, String)> = ids.into_iter().map(|id| ("id", id)).collect();
        if let Some(c) = current { params.push(("current", c)); }
        if let Some(p) = position_ms { params.push(("position", p.to_string())); }
        if let Err(e) = subsonic_request(&app, &state, "savePlayQueue", &params, true).await {
            eprintln!("Save play queue failed: {e}");
        }
    });
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePlayQueue {
    pub entries: Vec<Song>,
    pub current: Option<String>,
    pub position_ms: Option<i64>,
    pub changed_by: Option<String>,
}

/// Fetches the last saved play queue from the server via `getPlayQueue`, for
/// resuming playback that was started on another device. Returns `None` if no
/// queue has been saved.
#[tauri::command]
pub async fn get_play_queue(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Option<RemotePlayQueue>, String> {
    let body = subsonic_request(&app, &state, "getPlayQueue", &[], true).await?;
    let queue = body.get("playQueue").cloned();
    let Some(queue) = queue else { return Ok(None) };

    let entries = map_songs(array_field(&queue, &["entry"]));
    if entries.is_empty() {
        return Ok(None);
    }

    Ok(Some(RemotePlayQueue {
        entries,
        current: queue.get("current").and_then(|v| v.as_str()).map(str::to_string),
        position_ms: queue.get("position").and_then(|v| v.as_i64()),
        changed_by: queue.get("changedBy").and_then(|v| v.as_str()).map(str::to_string),
    }))
}

// ── Sonic Similarity ─────────────────────────────────────────────────────────

/// Fetches audio-similar tracks for a song via the `sonicSimilarity` OpenSubsonic
/// extension (`getSonicSimilarTracks`). Errors if the server hasn't advertised
/// the extension, so the frontend can hide the feature.
#[tauri::command]
pub async fn get_sonic_similar_tracks(app: AppHandle, state: State<'_, Arc<AppState>>, id: String, count: Option<i32>) -> Result<Vec<SimilarMatch>, String> {
    if !has_extension(&state, "sonicSimilarity") {
        return Err("sonicSimilarity not supported".to_string());
    }
    let mut params = vec![("id", id)];
    if let Some(count) = count {
        params.push(("count", count.to_string()));
    }
    let body = subsonic_request(&app, &state, "getSonicSimilarTracks", &params, false).await?;
    Ok(map_similar_matches(array_field(&body, &["sonicMatch"])))
}

/// Finds a transition path of audio-similar tracks between two songs via the
/// `sonicSimilarity` OpenSubsonic extension (`findSonicPath`).
#[tauri::command]
pub async fn find_sonic_path(app: AppHandle, state: State<'_, Arc<AppState>>, start_song_id: String, end_song_id: String, count: Option<i32>) -> Result<Vec<SimilarMatch>, String> {
    if !has_extension(&state, "sonicSimilarity") {
        return Err("sonicSimilarity not supported".to_string());
    }
    let mut params = vec![("startSongId", start_song_id), ("endSongId", end_song_id)];
    if let Some(count) = count {
        params.push(("count", count.to_string()));
    }
    let body = subsonic_request(&app, &state, "findSonicPath", &params, false).await?;
    Ok(map_similar_matches(array_field(&body, &["sonicMatch"])))
}

/// Fallback "similar tracks" for servers without `sonicSimilarity`: combines
/// genre-matched songs (`getSongsByGenre`) and tracks by Last.fm-similar artists
/// (`getArtistInfo2` → `getTopSongs`), with synthetic similarity scores so the
/// existing Similar Tracks UI keeps working unchanged.
#[tauri::command]
pub async fn get_similar_tracks_fallback(app: AppHandle, state: State<'_, Arc<AppState>>, song_id: String, artist_id: Option<String>, genre: Option<String>, count: Option<i32>) -> Result<Vec<SimilarMatch>, String> {
    use rand::seq::SliceRandom;

    let count = count.unwrap_or(10).max(1) as usize;
    let mut matches: Vec<SimilarMatch> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(song_id.clone());

    if let Some(genre) = genre {
        if let Ok(body) = subsonic_request(&app, &state, "getSongsByGenre", &[("genre", genre), ("count", (count * 2).to_string())], true).await {
            for song in map_songs(array_field(&body, &["songsByGenre", "song"])) {
                if seen.insert(song.id().to_string()) {
                    matches.push(SimilarMatch::new(song, 0.55));
                }
            }
        }
    }

    if let Some(artist_id) = artist_id {
        if let Ok(body) = subsonic_request(&app, &state, "getArtistInfo2", &[("id", artist_id), ("count", "5".to_string())], true).await {
            for similar in array_field(&body, &["artistInfo2", "similarArtist"]).iter().take(3) {
                let Some(name) = similar.get("name").and_then(|v| v.as_str()) else { continue };
                if let Ok(top_body) = subsonic_request(&app, &state, "getTopSongs", &[("artist", name.to_string()), ("count", "2".to_string())], true).await {
                    for song in map_songs(array_field(&top_body, &["topSongs", "song"])) {
                        if seen.insert(song.id().to_string()) {
                            matches.push(SimilarMatch::new(song, 0.45));
                        }
                    }
                }
            }
        }
    }

    matches.shuffle(&mut rand::rng());
    matches.truncate(count);
    Ok(matches)
}

// ── Lyrics ───────────────────────────────────────────────────────────────────

/// Full lyrics lookup cascade: OpenSubsonic structured lyrics (synced
/// preferred), then legacy plain-text lyrics, then optionally LRCLIB.
#[tauri::command]
pub async fn get_song_lyrics(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    song_id: String,
    artist: String,
    title: String,
    duration: f64,
    use_lrclib_fallback: bool,
) -> Result<Option<LyricsResult>, String> {
    // 1. OpenSubsonic structured lyrics (synced preferred)
    if let Ok(body) = subsonic_request(&app, &state, "getLyricsBySongId", &[("id", song_id)], false).await {
        let list = array_field(&body, &["lyricsList", "structuredLyrics"]);
        let best = list.iter().find(|l| l.get("synced").and_then(|v| v.as_bool()) == Some(true)).or_else(|| list.first());
        if let Some(best) = best {
            if let Some(line) = best.get("line").and_then(|v| v.as_array()) {
                if !line.is_empty() {
                    let offset = best.get("offset").and_then(|v| v.as_i64()).unwrap_or(0);
                    let lines = line.iter().map(|l| LyricLine {
                        start: l.get("start").and_then(|v| v.as_i64()).unwrap_or(0) + offset,
                        value: l.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    }).collect();
                    return Ok(Some(LyricsResult { lines, synced: best.get("synced").and_then(|v| v.as_bool()).unwrap_or(false) }));
                }
            }
        }
    }
    // 2. Legacy getLyrics (plain text)
    if let Ok(body) = subsonic_request(&app, &state, "getLyrics", &[("artist", artist.clone()), ("title", title.clone())], false).await {
        if let Some(value) = body.get("lyrics").and_then(|l| l.get("value")).and_then(|v| v.as_str()) {
            if !value.trim().is_empty() {
                let lines = value.lines().map(|v| LyricLine { start: 0, value: v.to_string() }).collect();
                return Ok(Some(LyricsResult { lines, synced: false }));
            }
        }
    }
    // 3. LRCLIB external fallback
    if use_lrclib_fallback {
        if let Ok(Some(result)) = fetch_lrclib_lyrics(artist, title, duration).await {
            return Ok(Some(result));
        }
    }
    Ok(None)
}
