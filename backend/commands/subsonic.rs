// ============================================================================
// OPENSUBSONIC API
// ============================================================================
// Read-only OpenSubsonic endpoints. Connection details (server/username/
// password) are held in `AppState.connection` and set via `set_connection`.
//
// These were Tauri commands; in the iced app they are plain async fns called
// from `App::update` via `iced::Task::perform`. Async fns take an owned
// `Arc<AppState>` so the returned future is `'static`; sync fns take `&AppState`.

use crate::commands::auth::generate_auth_params;
use crate::commands::lyrics::{fetch_lrclib_lyrics, LyricLine, LyricsResult};
use crate::commands::mappers::{map_albums, map_artists, map_similar_matches, map_songs, Album, Artist, SimilarMatch, Song};
use crate::events::BackendEvent;
use crate::state::{AppState, ConnectionState};
use std::sync::Arc;

/// Performs an authenticated GET against the connected OpenSubsonic server and
/// returns the parsed `subsonic-response` body. On HTTP 401 or OpenSubsonic
/// error codes 40/41, emits `BackendEvent::SessionExpired` (unless `silent`) and
/// returns `Err(UserError::SessionExpired)`.
async fn subsonic_request(
    state: &AppState,
    action: &str,
    params: &[(&str, String)],
    silent: bool,
) -> Result<serde_json::Value, crate::errors::UserError> {
    use crate::errors::UserError;

    let (server, username, password) = {
        let conn = state.connection.read();
        (
            conn.server.clone().ok_or(UserError::Network)?,
            conn.username.clone().unwrap_or_default(),
            conn.password.clone().unwrap_or_default(),
        )
    };

    let auth = generate_auth_params(username, password);
    let mut url = reqwest::Url::parse(&format!("{server}/rest/{action}"))
        .map_err(|_| UserError::Unknown)?;
    {
        let mut query = url.query_pairs_mut();
        for key in ["u", "t", "s", "v", "c", "f"] {
            query.append_pair(key, auth[key].as_str().unwrap_or(""));
        }
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }

    let res = state.http.get(url).send().await?;
    if res.status() == reqwest::StatusCode::UNAUTHORIZED {
        if !silent {
            state.bus.emit(BackendEvent::SessionExpired);
        }
        return Err(UserError::SessionExpired);
    }
    if !res.status().is_success() {
        return Err(UserError::Server { code: res.status().as_u16() });
    }

    let json: serde_json::Value = res.json().await?;
    let body = json.get("subsonic-response").ok_or(UserError::Unknown)?.clone();

    if let Some(ext) = body.get("openSubsonicExtensions") {
        state.connection.write().open_subsonic_extensions = ext.as_array().map(|arr| {
            arr.iter().filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string)).collect()
        });
    }

    if body.get("status").and_then(|v| v.as_str()) == Some("failed") {
        let code = body.get("error").and_then(|e| e.get("code")).and_then(|c| c.as_u64());
        if code == Some(40) || code == Some(41) {
            if !silent {
                state.bus.emit(BackendEvent::SessionExpired);
            }
            return Err(UserError::SessionExpired);
        }
        return Err(UserError::Server { code: code.unwrap_or(0) as u16 });
    }

    Ok(body)
}

/// Checks whether the connected server has advertised the given OpenSubsonic extension.
fn has_extension(state: &AppState, name: &str) -> bool {
    state.connection.read().open_subsonic_extensions.as_deref().unwrap_or(&[]).iter().any(|e| e == name)
}

/// Returns the OpenSubsonic extensions advertised by the connected server, as
/// detected from the most recent API response.
#[allow(dead_code)]
pub fn get_open_subsonic_extensions(state: &AppState) -> Vec<String> {
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

/// Stores the active connection's credentials. Called on login/logout/session expiry.
pub fn set_connection(state: &AppState, server: Option<String>, username: Option<String>, password: Option<String>) {
    let mut conn = state.connection.write();
    conn.server = server;
    conn.username = username;
    conn.password = password;
    conn.open_subsonic_extensions = None;
}

/// Validates credentials with a minimal request, used during the initial
/// login flow. Does not emit `SessionExpired` on failure, since a rejected
/// login isn't an expired session.
pub async fn validate_connection(state: Arc<AppState>) -> Result<(), crate::errors::UserError> {
    match subsonic_request(&state, "getAlbumList2", &[("type", "alphabeticalByName".to_string()), ("size", "1".to_string())], true).await {
        Ok(_) => {
            // openSubsonicExtensions is only included on this dedicated endpoint,
            // not on regular responses like getAlbumList2.
            let _ = subsonic_request(&state, "getOpenSubsonicExtensions", &[], true).await;
            Ok(())
        }
        Err(e) => {
            *state.connection.write() = ConnectionState::default();
            // A rejection during login means bad credentials, not an expired
            // session — surface it as Auth so the user sees a toast (SessionExpired
            // is suppressed by the UI in favour of the re-login event flow).
            let e = if matches!(e, crate::errors::UserError::SessionExpired) {
                crate::errors::UserError::Auth
            } else {
                e
            };
            Err(e)
        }
    }
}

// ── Reads ────────────────────────────────────────────────────────────────────

const API_PAGE_SIZE: &str = "500";

pub async fn get_albums(state: Arc<AppState>) -> Result<Vec<Album>, crate::errors::UserError> {
    let page_size: u32 = API_PAGE_SIZE.parse().unwrap_or(500);
    let mut all = Vec::new();
    let mut offset: u32 = 0;
    loop {
        let body = subsonic_request(
            &state,
            "getAlbumList2",
            &[("type", "alphabeticalByName".to_string()), ("size", page_size.to_string()), ("offset", offset.to_string())],
            false,
        )
        .await?;
        let page = array_field(&body, &["albumList2", "album"]);
        let page_len = page.len();
        all.extend(page);
        if page_len < page_size as usize {
            break;
        }
        offset += page_size;
    }
    Ok(map_albums(all))
}

pub async fn get_artists(state: Arc<AppState>) -> Result<Vec<Artist>, crate::errors::UserError> {
    let body = subsonic_request(&state, "getArtists", &[], false).await?;
    let mut raw = Vec::new();
    for group in array_field(&body, &["artists", "index"]) {
        if let Some(artists) = group.get("artist").and_then(|v| v.as_array()) {
            raw.extend(artists.iter().cloned());
        }
    }
    Ok(map_artists(raw))
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumTracks {
    pub tracks: Vec<Song>,
    pub album_name: String,
    pub album_artist: String,
    pub cover_art_id: Option<String>,
}

pub async fn get_album_tracks(state: Arc<AppState>, id: String) -> Result<AlbumTracks, crate::errors::UserError> {
    let body = subsonic_request(&state, "getAlbum", &[("id", id)], false).await?;
    let album = body.get("album").cloned().unwrap_or(serde_json::Value::Null);
    let tracks = map_songs(array_field(&album, &["song"]));
    Ok(AlbumTracks {
        tracks,
        album_name: album.get("name").or_else(|| album.get("title")).and_then(|v| v.as_str()).unwrap_or("Unknown Album").to_string(),
        album_artist: album.get("displayArtist").or_else(|| album.get("artist")).and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        cover_art_id: album.get("coverArt").and_then(|v| v.as_str()).map(str::to_string),
    })
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetails {
    pub name: String,
    pub albums: Vec<Album>,
}

pub async fn get_artist_details(state: Arc<AppState>, id: String) -> Result<ArtistDetails, crate::errors::UserError> {
    let body = subsonic_request(&state, "getArtist", &[("id", id)], false).await?;
    let artist = body.get("artist").cloned().unwrap_or(serde_json::Value::Null);
    Ok(ArtistDetails {
        name: artist.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        albums: map_albums(array_field(&artist, &["album"])),
    })
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo {
    pub image: Option<String>,
    pub bio: Option<String>,
}

/// Returns artist info (bio + image) from the server's getArtistInfo2 endpoint.
/// When a user-supplied Last.fm API key and the artist name are provided, the
/// bio (and a non-placeholder image, if any) are overlaid from Last.fm's
/// artist.getInfo, which is richer than many servers' cached copies.
pub async fn get_artist_info(
    state: Arc<AppState>,
    id: String,
    lastfm_key: String,
    artist_name: String,
) -> Result<Option<ArtistInfo>, crate::errors::UserError> {
    let mut info = match subsonic_request(&state, "getArtistInfo2", &[("id", id)], false).await {
        Ok(body) => {
            let raw = body.get("artistInfo2").cloned().unwrap_or(serde_json::Value::Null);
            let image = ["largeImageUrl", "mediumImageUrl", "smallImageUrl"]
                .iter()
                .find_map(|k| raw.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_string));
            let bio = raw.get("biography").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_string);
            ArtistInfo { image, bio }
        }
        Err(_) => ArtistInfo { image: None, bio: None },
    };

    if !lastfm_key.trim().is_empty() && !artist_name.trim().is_empty() {
        if let Some(lf) = fetch_lastfm_artist_info(&state.http, lastfm_key.trim(), artist_name.trim()).await {
            if lf.bio.is_some() {
                info.bio = lf.bio;
            }
            if lf.image.is_some() {
                info.image = lf.image;
            }
        }
    }

    Ok(Some(info))
}

/// Fetches artist bio (and a best-effort image) directly from Last.fm's
/// `artist.getInfo` using the user's own API key. Returns `None` on any
/// network/parse failure so the server baseline is kept.
async fn fetch_lastfm_artist_info(
    http: &reqwest::Client,
    api_key: &str,
    artist: &str,
) -> Option<ArtistInfo> {
    // Last.fm artist images have been a shared placeholder ("star") since 2019;
    // skip that known hash so we don't show a blank graphic.
    const PLACEHOLDER: &str = "2a96cbd8b46e442fc41c2b86b821562f";

    let mut url = reqwest::Url::parse("https://ws.audioscrobbler.com/2.0/").ok()?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("method", "artist.getinfo");
        query.append_pair("artist", artist);
        query.append_pair("api_key", api_key);
        query.append_pair("format", "json");
        query.append_pair("autocorrect", "1");
    }
    let resp = http.get(url).send().await.ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;
    let a = body.get("artist")?;

    let bio = a
        .pointer("/bio/content")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let image = a.get("image").and_then(|v| v.as_array()).and_then(|arr| {
        arr.iter().rev().find_map(|im| {
            im.get("#text")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty() && !s.contains(PLACEHOLDER))
                .map(str::to_string)
        })
    });

    if bio.is_none() && image.is_none() {
        return None;
    }
    Some(ArtistInfo { image, bio })
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub songs: Vec<Song>,
    pub albums: Vec<Album>,
}

pub async fn search(state: Arc<AppState>, query: String) -> Result<SearchResult, crate::errors::UserError> {
    let body = subsonic_request(&state, "search3", &[("query", query), ("albumCount", "40".to_string()), ("songCount", "100".to_string())], false).await?;
    Ok(SearchResult {
        songs: map_songs(array_field(&body, &["searchResult3", "song"])),
        albums: map_albums(array_field(&body, &["searchResult3", "album"])),
    })
}

async fn fetch_album_list(state: &AppState, list_type: &str, size: u32) -> Result<Vec<Album>, crate::errors::UserError> {
    let body = subsonic_request(state, "getAlbumList2", &[("type", list_type.to_string()), ("size", size.to_string())], false).await?;
    Ok(map_albums(array_field(&body, &["albumList2", "album"])))
}

pub async fn get_recent_albums(state: Arc<AppState>, size: u32) -> Result<Vec<Album>, crate::errors::UserError> {
    fetch_album_list(&state, "recent", size).await
}

pub async fn get_random_albums(state: Arc<AppState>, size: u32) -> Result<Vec<Album>, crate::errors::UserError> {
    fetch_album_list(&state, "random", size).await
}

pub async fn get_newest_albums(state: Arc<AppState>, size: u32) -> Result<Vec<Album>, crate::errors::UserError> {
    fetch_album_list(&state, "newest", size).await
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub name: String,
    pub album_count: u32,
    pub song_count: u32,
}

pub async fn get_genres_list(state: Arc<AppState>) -> Result<Vec<Genre>, crate::errors::UserError> {
    let body = subsonic_request(&state, "getGenres", &[], false).await?;
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

/// Returns all playlists visible to the current user (raw JSON objects).
pub async fn get_playlists(state: Arc<AppState>) -> Result<Vec<serde_json::Value>, crate::errors::UserError> {
    let body = subsonic_request(&state, "getPlaylists", &[], false).await?;
    Ok(array_field(&body, &["playlists", "playlist"]))
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTracks {
    pub id: String,
    pub name: String,
    pub comment: String,
    pub song_count: Option<u32>,
    pub tracks: Vec<Song>,
}

/// Fetches a playlist's full track list from the server.
pub async fn get_playlist_tracks(state: Arc<AppState>, id: String) -> Result<PlaylistTracks, crate::errors::UserError> {
    let body = subsonic_request(&state, "getPlaylist", &[("id", id)], false).await?;
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
pub async fn create_playlist(state: Arc<AppState>, name: String) -> Result<serde_json::Value, crate::errors::UserError> {
    let body = subsonic_request(&state, "createPlaylist", &[("name", name)], false).await?;
    Ok(body.get("playlist").cloned().unwrap_or(serde_json::Value::Null))
}

/// Updates playlist metadata and/or adds/removes tracks by server-side ID/index.
#[allow(clippy::too_many_arguments)]
pub async fn update_playlist(
    state: Arc<AppState>,
    id: String,
    name: Option<String>,
    comment: Option<String>,
    song_ids_to_add: Vec<String>,
    song_indices_to_remove: Vec<u32>,
) -> Result<(), crate::errors::UserError> {
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
    subsonic_request(&state, "updatePlaylist", &params, false).await?;
    Ok(())
}

/// Deletes a playlist from the server.
pub async fn delete_playlist(state: Arc<AppState>, id: String) -> Result<(), crate::errors::UserError> {
    subsonic_request(&state, "deletePlaylist", &[("id", id)], false).await?;
    Ok(())
}

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

/// Builds an authenticated `getCoverArt` URL for a cover id. Passed to
/// `cover_cache::get_cover_art`, which downloads + caches it on disk.
pub fn build_cover_url(state: &AppState, cover_id: &str, size: u32) -> Result<String, String> {
    let (server, username, password) = {
        let conn = state.connection.read();
        (
            conn.server.clone().ok_or("Not connected")?,
            conn.username.clone().unwrap_or_default(),
            conn.password.clone().unwrap_or_default(),
        )
    };
    let auth = generate_auth_params(username, password);
    let mut url = reqwest::Url::parse(&format!("{server}/rest/getCoverArt")).map_err(|e| e.to_string())?;
    {
        let mut query = url.query_pairs_mut();
        for key in ["u", "t", "s", "v", "c", "f"] {
            query.append_pair(key, auth[key].as_str().unwrap_or(""));
        }
        query.append_pair("id", cover_id);
        query.append_pair("size", &size.to_string());
    }
    Ok(url.to_string())
}

/// Fire-and-forget scrobble, spawned from the (tokio) queue manager.
pub(crate) fn fire_scrobble(state: Arc<AppState>, id: String, submission: bool) {
    tokio::spawn(async move {
        let params = [("id", id), ("submission", submission.to_string()), ("time", "0".to_string())];
        if let Err(e) = subsonic_request(&state, "scrobble", &params, true).await {
            eprintln!("Scrobble failed: {}", e.message());
        }
    });
}

/// Fire-and-forget playback report, spawned from the (tokio) queue manager.
pub(crate) fn fire_report_playback(state: Arc<AppState>, media_id: String, position_ms: i64, playback_state: String) {
    if !has_extension(&state, "playbackReport") { return; }
    tokio::spawn(async move {
        let params = [
            ("mediaId", media_id),
            ("mediaType", "song".to_string()),
            ("positionMs", position_ms.to_string()),
            ("state", playback_state),
        ];
        if let Err(e) = subsonic_request(&state, "reportPlayback", &params, true).await {
            eprintln!("Report playback failed: {}", e.message());
        }
    });
}

/// Fire-and-forget save-play-queue, spawned from the (tokio) queue manager.
pub(crate) fn fire_save_play_queue(state: Arc<AppState>, ids: Vec<String>, current: Option<String>, position_ms: Option<i64>) {
    tokio::spawn(async move {
        let mut params: Vec<(&str, String)> = ids.into_iter().map(|id| ("id", id)).collect();
        if let Some(c) = current { params.push(("current", c)); }
        if let Some(p) = position_ms { params.push(("position", p.to_string())); }
        if let Err(e) = subsonic_request(&state, "savePlayQueue", &params, true).await {
            eprintln!("Save play queue failed: {}", e.message());
        }
    });
}

/// Reports playback progress to the server. Errors are logged, not surfaced,
/// and never trigger a session-expiry prompt.
#[allow(dead_code)]
pub async fn scrobble(state: Arc<AppState>, id: String, submission: bool, time: i64) {
    let params = [("id", id), ("submission", submission.to_string()), ("time", time.to_string())];
    if let Err(e) = subsonic_request(&state, "scrobble", &params, true).await {
        eprintln!("Scrobble failed: {}", e.message());
    }
}

/// Sets a 1–5 star rating on a track (0 clears it). Logged-only on error.
pub async fn set_rating(state: Arc<AppState>, id: String, rating: u32) {
    let params = [("id", id), ("rating", rating.to_string())];
    if let Err(e) = subsonic_request(&state, "setRating", &params, true).await {
        eprintln!("Set rating failed: {}", e.message());
    }
}

/// Reports playback state/position via the `playbackReport` extension. No-op if
/// the server hasn't advertised it. Logged-only on error.
#[allow(dead_code)]
pub async fn report_playback(state: Arc<AppState>, media_id: String, position_ms: i64, playback_state: String) {
    if !has_extension(&state, "playbackReport") {
        return;
    }
    let params = [
        ("mediaId", media_id),
        ("mediaType", "song".to_string()),
        ("positionMs", position_ms.to_string()),
        ("state", playback_state),
    ];
    if let Err(e) = subsonic_request(&state, "reportPlayback", &params, true).await {
        eprintln!("Report playback failed: {}", e.message());
    }
}

// ── Play Queue (cross-device continue) ──────────────────────────────────────

/// Saves the current queue/position to the server via `savePlayQueue`, so it can
/// be resumed on another device. Logged-only on error.
#[allow(dead_code)]
pub async fn save_play_queue(state: Arc<AppState>, ids: Vec<String>, current: Option<String>, position_ms: Option<i64>) {
    let mut params: Vec<(&str, String)> = ids.into_iter().map(|id| ("id", id)).collect();
    if let Some(c) = current { params.push(("current", c)); }
    if let Some(p) = position_ms { params.push(("position", p.to_string())); }
    if let Err(e) = subsonic_request(&state, "savePlayQueue", &params, true).await {
        eprintln!("Save play queue failed: {}", e.message());
    }
}

#[derive(serde::Serialize, Clone, Debug)]
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
pub async fn get_play_queue(state: Arc<AppState>) -> Result<Option<RemotePlayQueue>, crate::errors::UserError> {
    let body = subsonic_request(&state, "getPlayQueue", &[], true).await?;
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
/// the extension, so the UI can hide the feature.
#[allow(dead_code)]
pub async fn get_sonic_similar_tracks(state: Arc<AppState>, id: String, count: Option<i32>) -> Result<Vec<SimilarMatch>, crate::errors::UserError> {
    if !has_extension(&state, "sonicSimilarity") {
        return Err(crate::errors::UserError::Unknown);
    }
    let mut params = vec![("id", id)];
    if let Some(count) = count {
        params.push(("count", count.to_string()));
    }
    let body = subsonic_request(&state, "getSonicSimilarTracks", &params, false).await?;
    Ok(map_similar_matches(array_field(&body, &["sonicMatch"])))
}

/// Finds a transition path of audio-similar tracks between two songs via the
/// `sonicSimilarity` OpenSubsonic extension (`findSonicPath`).
#[allow(dead_code)]
pub async fn find_sonic_path(state: Arc<AppState>, start_song_id: String, end_song_id: String, count: Option<i32>) -> Result<Vec<SimilarMatch>, crate::errors::UserError> {
    if !has_extension(&state, "sonicSimilarity") {
        return Err(crate::errors::UserError::Unknown);
    }
    let mut params = vec![("startSongId", start_song_id), ("endSongId", end_song_id)];
    if let Some(count) = count {
        params.push(("count", count.to_string()));
    }
    let body = subsonic_request(&state, "findSonicPath", &params, false).await?;
    Ok(map_similar_matches(array_field(&body, &["sonicMatch"])))
}

/// Fallback "similar tracks" for servers without `sonicSimilarity`: combines
/// genre-matched songs (`getSongsByGenre`) and tracks by Last.fm-similar artists
/// (`getArtistInfo2` → `getTopSongs`), with synthetic similarity scores.
pub async fn get_similar_tracks_fallback(state: Arc<AppState>, song_id: String, artist_id: Option<String>, genre: Option<String>, count: Option<i32>) -> Result<Vec<SimilarMatch>, crate::errors::UserError> {
    use rand::seq::SliceRandom;

    let count = count.unwrap_or(10).max(1) as usize;
    let mut matches: Vec<SimilarMatch> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(song_id.clone());

    if let Some(genre) = genre {
        if let Ok(body) = subsonic_request(&state, "getSongsByGenre", &[("genre", genre), ("count", (count * 2).to_string())], true).await {
            for song in map_songs(array_field(&body, &["songsByGenre", "song"])) {
                if seen.insert(song.id().to_string()) {
                    matches.push(SimilarMatch::new(song, 0.55));
                }
            }
        }
    }

    if let Some(artist_id) = artist_id {
        if let Ok(body) = subsonic_request(&state, "getArtistInfo2", &[("id", artist_id), ("count", "5".to_string())], true).await {
            for similar in array_field(&body, &["artistInfo2", "similarArtist"]).iter().take(3) {
                let Some(name) = similar.get("name").and_then(|v| v.as_str()) else { continue };
                if let Ok(top_body) = subsonic_request(&state, "getTopSongs", &[("artist", name.to_string()), ("count", "2".to_string())], true).await {
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

// ── Library song enumeration (Radio / Mood Mix seeding) ──────────────────────

/// Returns songs of a given genre via `getSongsByGenre`.
pub async fn get_songs_by_genre(state: Arc<AppState>, genre: String, count: Option<i32>) -> Result<Vec<Song>, crate::errors::UserError> {
    let count = count.unwrap_or(100).clamp(1, 500);
    let body = subsonic_request(&state, "getSongsByGenre", &[("genre", genre), ("count", count.to_string())], false).await?;
    Ok(map_songs(array_field(&body, &["songsByGenre", "song"])))
}

/// Returns a random sample of library songs via `getRandomSongs` (optionally
/// scoped to a genre).
pub async fn get_random_songs(state: Arc<AppState>, count: Option<i32>, genre: Option<String>) -> Result<Vec<Song>, crate::errors::UserError> {
    let size = count.unwrap_or(100).clamp(1, 500);
    let mut params = vec![("size", size.to_string())];
    if let Some(genre) = genre {
        params.push(("genre", genre));
    }
    let body = subsonic_request(&state, "getRandomSongs", &params, false).await?;
    Ok(map_songs(array_field(&body, &["randomSongs", "song"])))
}

/// Returns the names of similar artists from the server's `getArtistInfo2`
/// (`similarArtist[]`), for the artist-page "You might also like" section.
pub async fn get_similar_artists(state: Arc<AppState>, id: String, count: Option<i32>) -> Result<Vec<String>, crate::errors::UserError> {
    let count = count.unwrap_or(20).clamp(1, 100);
    let body = subsonic_request(&state, "getArtistInfo2", &[("id", id), ("count", count.to_string())], false).await?;
    Ok(array_field(&body, &["artistInfo2", "similarArtist"])
        .iter()
        .filter_map(|a| a.get("name").and_then(|v| v.as_str()).map(str::to_string))
        .collect())
}

// ── Lyrics ───────────────────────────────────────────────────────────────────

/// Full lyrics lookup cascade: OpenSubsonic structured lyrics (synced
/// preferred), then legacy plain-text lyrics, then optionally LRCLIB.
pub async fn get_song_lyrics(
    state: Arc<AppState>,
    song_id: String,
    artist: String,
    title: String,
    duration: f64,
    use_lrclib_fallback: bool,
) -> Result<Option<LyricsResult>, crate::errors::UserError> {
    // 1. OpenSubsonic structured lyrics (synced preferred)
    if let Ok(body) = subsonic_request(&state, "getLyricsBySongId", &[("id", song_id)], false).await {
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
    if let Ok(body) = subsonic_request(&state, "getLyrics", &[("artist", artist.clone()), ("title", title.clone())], false).await {
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
