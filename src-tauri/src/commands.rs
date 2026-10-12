//! Tauri command layer. Every function here is a thin forwarder to
//! `firmium_backend` (mostly `Core`); no app logic lives in this file.

use std::sync::Arc;

use firmium_backend::commands::equalizer::{self, BandSpec, EqState};
use firmium_backend::commands::mappers::{Album, Artist, SimilarMatch, Song, Starred};
use firmium_backend::commands::subsonic::{self, AlbumTracks, ArtistDetails, ArtistInfo, Genre, PlaylistTracks, RemotePlayQueue, SearchResult, StarKind};
use firmium_backend::commands::themes::{self, ThemeEntry};
use firmium_backend::commands::{cover_cache, downloads, local_library, queue};
use firmium_backend::core::library::{Energy, TimedLyrics};
use firmium_backend::core::playlists::{Playlist, PlaylistLists};
use firmium_backend::core::{AuthState, Core, Settings};
use firmium_backend::db::{PlayHistorySummary, RecapStats, RecentPlay};
use firmium_backend::errors::UserError;
use firmium_backend::podcasts::{PodcastChannel, PodcastEpisode};
use firmium_backend::AudioDevice;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

type S<'a> = State<'a, Arc<Core>>;

/// Error shape sent to the UI: a stable `kind` plus a user-facing `message`.
#[derive(Serialize)]
pub struct CmdError {
    kind: String,
    message: String,
}

impl From<UserError> for CmdError {
    fn from(e: UserError) -> Self {
        let kind = format!("{e:?}").split([' ', '{']).next().unwrap_or("Unknown").to_string();
        CmdError { kind, message: e.message() }
    }
}

impl From<String> for CmdError {
    fn from(message: String) -> Self {
        CmdError { kind: "Unknown".into(), message }
    }
}

type R<T> = Result<T, CmdError>;

// ── Session, settings, themes ───────────────────────────────────────────────

#[tauri::command]
pub fn get_app_version() -> &'static str {
    firmium_backend::commands::app_info::get_app_version()
}

#[tauri::command]
pub fn list_themes() -> Vec<ThemeEntry> {
    themes::list_themes()
}

#[tauri::command]
pub fn get_auth_state(core: S) -> AuthState {
    core.auth_state()
}

#[tauri::command]
pub async fn auto_login(core: S<'_>) -> R<Option<AuthState>> {
    Ok(core.auto_login().await?)
}

#[tauri::command]
pub async fn connect(core: S<'_>, server: String, username: String, password: String, save_password: bool) -> R<AuthState> {
    Ok(core.connect(server, username, password, save_password).await?)
}

#[tauri::command]
pub async fn switch_account(core: S<'_>, server: String, username: String) -> R<AuthState> {
    Ok(core.switch_account(server, username).await?)
}

#[tauri::command]
pub fn logout(core: S) -> AuthState {
    core.logout()
}

#[tauri::command]
pub fn remove_account(core: S, server: String, username: String) -> AuthState {
    core.remove_account(&server, &username)
}

#[tauri::command]
pub fn get_settings(core: S) -> Settings {
    core.settings()
}

#[tauri::command]
pub fn set_setting(core: S, key: String, value: serde_json::Value) -> R<Settings> {
    Ok(core.set_setting(&key, value)?)
}

#[tauri::command]
pub fn reset_settings(core: S) -> Settings {
    core.reset_settings()
}

#[tauri::command]
pub fn get_server_extensions(core: S) -> Vec<String> {
    subsonic::get_open_subsonic_extensions(&core.backend.app_state)
}

// ── Library (routed server / offline) ───────────────────────────────────────

#[tauri::command]
pub async fn get_albums(core: S<'_>) -> R<Vec<Album>> {
    Ok(core.albums().await?)
}

#[tauri::command]
pub async fn get_artists(core: S<'_>) -> R<Vec<Artist>> {
    Ok(core.artists().await?)
}

#[tauri::command]
pub async fn get_album_tracks(core: S<'_>, id: String) -> R<AlbumTracks> {
    Ok(core.album_tracks(id).await?)
}

#[tauri::command]
pub async fn get_artist_details(core: S<'_>, id: String) -> R<ArtistDetails> {
    Ok(core.artist_details(id).await?)
}

#[tauri::command]
pub async fn get_artist_info(core: S<'_>, id: String, artist_name: String) -> R<Option<ArtistInfo>> {
    Ok(core.artist_info(id, artist_name).await?)
}

#[tauri::command]
pub async fn get_artist_image(core: S<'_>, id: String) -> R<Option<String>> {
    Ok(core.artist_image(id).await)
}

#[tauri::command]
pub async fn play_artist(core: S<'_>, id: String, shuffle: bool) -> R<usize> {
    Ok(core.play_artist(id, shuffle).await?)
}

#[tauri::command]
pub async fn get_artist_recommendations(core: S<'_>, id: String, name: String) -> R<Vec<Artist>> {
    Ok(core.artist_recommendations(id, name).await)
}

#[tauri::command]
pub async fn get_similar_artists(core: S<'_>, id: String, count: Option<i32>) -> R<Vec<String>> {
    Ok(subsonic::get_similar_artists(core.backend.app_state.clone(), id, count).await?)
}

#[tauri::command]
pub async fn search(core: S<'_>, query: String) -> R<SearchResult> {
    Ok(core.search(query).await?)
}

/// `kind`: "recent" | "random" | "newest".
#[tauri::command]
pub async fn get_album_list(core: S<'_>, kind: String, size: u32) -> R<Vec<Album>> {
    Ok(core.album_list(&kind, size).await?)
}

#[tauri::command]
pub async fn get_genres(core: S<'_>) -> R<Vec<Genre>> {
    Ok(core.genres().await?)
}

#[tauri::command]
pub async fn get_songs_by_genre(core: S<'_>, genre: String, count: Option<i32>) -> R<Vec<Song>> {
    Ok(subsonic::get_songs_by_genre(core.backend.app_state.clone(), genre, count).await?)
}

#[tauri::command]
pub async fn get_lyrics(core: S<'_>, song: Song) -> R<Option<TimedLyrics>> {
    Ok(core.lyrics(&song).await?)
}

#[tauri::command]
pub async fn get_similar_tracks(core: S<'_>, song: Song, count: Option<usize>) -> R<Vec<SimilarMatch>> {
    let st = core.backend.app_state.clone();
    let n = count.map(|c| c as i32);
    let sonic = subsonic::get_open_subsonic_extensions(&st).iter().any(|e| e == "sonicSimilarity");
    Ok(if sonic {
        subsonic::get_sonic_similar_tracks(st, song.id, n).await?
    } else {
        let genre = song.genres.as_ref().and_then(|g| g.get(0)).and_then(|g| g.as_str().or_else(|| g.get("name").and_then(|n| n.as_str()))).map(String::from);
        subsonic::get_similar_tracks_fallback(st, song.id, song.artist_id, genre, n).await?
    })
}

/// Cover art as a local file path (the UI converts it with `convertFileSrc`).
#[tauri::command]
pub async fn get_cover(core: S<'_>, cover_id: String) -> R<String> {
    Ok(core.cover_path(cover_id).await?)
}

#[tauri::command]
pub async fn get_cover_colors(core: S<'_>, cover_id: String) -> R<firmium_backend::commands::cover_colors::CoverColorsResult> {
    Ok(core.cover_colors(cover_id).await?)
}

#[tauri::command]
pub fn clear_cover_cache() -> R<()> {
    Ok(cover_cache::clear_cover_cache()?)
}

// ── Favorites & ratings ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn set_starred(core: S<'_>, id: String, kind: StarKind, starred: bool) -> R<()> {
    let st = core.backend.app_state.clone();
    if starred { subsonic::star_item(st, id, kind).await? } else { subsonic::unstar_item(st, id, kind).await? }
    Ok(())
}

#[tauri::command]
pub async fn get_starred(core: S<'_>) -> R<Starred> {
    Ok(subsonic::get_starred(core.backend.app_state.clone()).await?)
}

#[tauri::command]
pub async fn set_rating(core: S<'_>, id: String, rating: u32) -> R<()> {
    subsonic::set_rating(core.backend.app_state.clone(), id, rating).await;
    Ok(())
}

// ── Playback queue & transport ──────────────────────────────────────────────

macro_rules! qargs {
    ($core:expr) => {
        ($core.backend.queue_state.clone(), $core.backend.app_state.clone(), $core.backend.audio_player.clone())
    };
}

#[tauri::command]
pub async fn set_queue(core: S<'_>, songs: Vec<Song>, start_idx: usize) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::set_queue(q, a, p, songs, start_idx).await?)
}

#[tauri::command]
pub async fn set_queue_seamless(core: S<'_>, songs: Vec<Song>, start_idx: usize) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::set_queue_seamless(q, a, p, songs, start_idx).await?)
}

#[tauri::command]
pub async fn append_and_play(core: S<'_>, songs: Vec<Song>) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::append_and_play(q, a, p, songs).await?)
}

#[tauri::command]
pub async fn shuffle_and_play(core: S<'_>, songs: Vec<Song>) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::shuffle_and_play(q, a, p, songs).await?)
}

#[tauri::command]
pub async fn play_queue_index(core: S<'_>, idx: usize) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::play_queue_index(q, a, p, idx).await?)
}

#[tauri::command]
pub async fn queue_next(core: S<'_>) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::queue_next(q, a, p).await?)
}

#[tauri::command]
pub async fn queue_prev(core: S<'_>) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::queue_prev(q, a, p).await?)
}

#[tauri::command]
pub async fn toggle_play(core: S<'_>) -> R<()> {
    let (q, a, p) = qargs!(core);
    Ok(queue::toggle_play(q, a, p).await?)
}

#[tauri::command]
pub fn seek_queue(core: S, position: f64) -> R<()> {
    Ok(queue::seek_queue(&core.backend.queue_state, &core.backend.audio_player, position)?)
}

#[tauri::command]
pub fn toggle_shuffle(core: S) {
    queue::toggle_shuffle(&core.backend.bus, &core.backend.queue_state);
}

#[tauri::command]
pub fn set_repeat_mode(core: S, repeat_one: bool, repeat_all: bool) {
    queue::set_repeat_mode(&core.backend.bus, &core.backend.queue_state, repeat_one, repeat_all);
}

/// Current queue snapshot, for hydrating the UI on load.
#[tauri::command]
pub fn get_queue_state(core: S) -> firmium_backend::queue_state::QueueStateSnapshot {
    core.backend.queue_state.inner.lock().snapshot()
}

#[tauri::command]
pub async fn generate_mix(core: S<'_>, energy: Energy, genre: Option<String>) -> R<usize> {
    Ok(core.generate_mix(energy, genre).await?)
}

#[tauri::command]
pub async fn start_radio(core: S<'_>, seed: Song) -> R<()> {
    Ok(core.start_radio(seed).await?)
}

#[tauri::command]
pub async fn get_remote_play_queue(core: S<'_>) -> R<Option<RemotePlayQueue>> {
    Ok(subsonic::get_play_queue(core.backend.app_state.clone()).await?)
}

/// Fetches the saved server queue, plays it and seeks to its saved offset.
#[tauri::command]
pub async fn resume_remote_queue(core: S<'_>) -> R<()> {
    let Some(queue_data) = subsonic::get_play_queue(core.backend.app_state.clone()).await? else { return Ok(()) };
    let start = queue_data.current.as_deref().and_then(|c| queue_data.entries.iter().position(|s| s.id == c)).unwrap_or(0);
    let pos = queue_data.position_ms.unwrap_or(0).max(0) as f64 / 1000.0;
    let (q, a, p) = qargs!(core);
    queue::set_queue(q, a, p, queue_data.entries, start).await?;
    Ok(queue::seek_queue(&core.backend.queue_state, &core.backend.audio_player, pos)?)
}

#[tauri::command]
pub fn list_audio_devices() -> Vec<AudioDevice> {
    firmium_backend::audio::AudioPlayer::list_devices()
}

// ── Equalizer ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_eq_state() -> EqState {
    equalizer::get_eq_state()
}

#[tauri::command]
pub fn save_eq_profile(core: S, name: String, kind: String, bands: Vec<BandSpec>) -> R<()> {
    Ok(equalizer::save_eq_profile(&core.backend.audio_player, name, kind, bands)?)
}

#[tauri::command]
pub fn delete_eq_profile(core: S, name: String) -> R<()> {
    Ok(equalizer::delete_eq_profile(&core.backend.audio_player, name)?)
}

#[tauri::command]
pub fn set_eq_active_profile(core: S, device: String, profile: String) -> R<()> {
    Ok(equalizer::set_eq_active_profile(&core.backend.audio_player, device, profile)?)
}

#[tauri::command]
pub fn set_eq_bands(core: S, profile: String, bands: Vec<BandSpec>) -> R<()> {
    Ok(equalizer::set_eq_bands(&core.backend.audio_player, profile, bands)?)
}

#[tauri::command]
pub fn set_eq_enabled(core: S, enabled: bool) -> R<()> {
    Ok(equalizer::set_eq_enabled(&core.backend.audio_player, enabled)?)
}

// ── Playlists (local-first, synced to the server) ───────────────────────────

#[tauri::command]
pub fn get_local_playlists(core: S) -> Vec<Playlist> {
    core.local_playlists()
}

#[tauri::command]
pub async fn get_server_playlists(core: S<'_>) -> R<PlaylistLists> {
    let server = subsonic::get_playlists(core.backend.app_state.clone()).await?;
    Ok(core.inner().reconcile_playlists(server).await)
}

#[tauri::command]
pub async fn add_to_server_playlist(core: S<'_>, server_id: String, song_ids: Vec<String>) -> R<()> {
    core.add_to_server_playlist(server_id, song_ids).await;
    Ok(())
}

#[tauri::command]
pub async fn reorder_server_playlist(core: S<'_>, server_id: String, ordered_ids: Vec<String>) -> R<()> {
    core.reorder_server_playlist(server_id, ordered_ids).await;
    Ok(())
}

#[tauri::command]
pub async fn remove_from_server_playlist(core: S<'_>, server_id: String, index: u32) -> R<()> {
    core.remove_from_server_playlist(server_id, index).await;
    Ok(())
}

#[tauri::command]
pub async fn get_server_playlist_tracks(core: S<'_>, id: String) -> R<PlaylistTracks> {
    Ok(subsonic::get_playlist_tracks(core.backend.app_state.clone(), id).await?)
}

#[tauri::command]
pub async fn create_playlist(core: S<'_>, name: String, first_track: Option<Song>) -> R<Vec<Playlist>> {
    Ok(core.inner().create_playlist(name, first_track).await)
}

#[tauri::command]
pub async fn sync_playlist_now(core: S<'_>, local_id: String) -> R<Vec<Playlist>> {
    Ok(core.inner().sync_playlist_now(local_id).await)
}

#[tauri::command]
pub async fn delete_playlist(core: S<'_>, local_id: String) -> R<Vec<Playlist>> {
    Ok(core.delete_local_playlist(&local_id).await)
}

#[tauri::command]
pub async fn rename_playlist(core: S<'_>, local_id: String, name: String) -> R<Vec<Playlist>> {
    Ok(core.rename_local_playlist(&local_id, name).await)
}

#[tauri::command]
pub async fn add_to_playlist(core: S<'_>, local_id: String, song: Song) -> R<Vec<Playlist>> {
    Ok(core.add_to_local_playlist(&local_id, song).await)
}

#[tauri::command]
pub async fn move_playlist_track(core: S<'_>, local_id: String, from: usize, to: usize) -> R<Vec<Playlist>> {
    Ok(core.move_local_track(&local_id, from, to).await)
}

#[tauri::command]
pub async fn remove_playlist_track(core: S<'_>, local_id: String, track_id: String) -> R<Vec<Playlist>> {
    Ok(core.remove_local_track(&local_id, &track_id).await)
}

// ── Downloads & local library ───────────────────────────────────────────────

#[tauri::command]
pub async fn download_track(core: S<'_>, song: Song, album_artist: Option<String>) -> R<()> {
    let format = core.settings().download_format;
    let artist = album_artist.unwrap_or_else(|| song.artist.clone());
    downloads::download_track(core.backend.app_state.clone(), song.id, format, artist, song.album, song.title, song.track_number, song.suffix).await?;
    Ok(())
}

#[tauri::command]
pub async fn download_album(core: S<'_>, album_id: String) -> R<()> {
    let format = core.settings().download_format;
    Ok(downloads::download_album(core.backend.app_state.clone(), album_id, format).await?)
}

#[tauri::command]
pub fn get_local_album_track_keys(core: S, album_artist: String, album: String) -> R<Vec<local_library::LocalTrackKey>> {
    Ok(local_library::get_local_album_track_keys(&core.backend.app_state, album_artist, album)?)
}

/// Copies audio files/folders (picked or dropped) into the local library.
#[tauri::command]
pub async fn import_local_files(core: S<'_>, paths: Vec<String>) -> R<usize> {
    Ok(local_library::import_local_files(core.backend.app_state.clone(), paths).await?)
}

/// Opens a native picker, then imports the chosen files. `None` if cancelled.
#[tauri::command]
pub async fn pick_and_import(app: AppHandle, core: S<'_>) -> R<Option<usize>> {
    let picked = tauri::async_runtime::spawn_blocking(move || app.dialog().file().add_filter("Audio", &["mp3", "flac", "ogg", "opus", "m4a", "aac", "wav"]).blocking_pick_files())
        .await
        .map_err(|e| e.to_string())?;
    let Some(files) = picked else { return Ok(None) };
    let paths = files.into_iter().filter_map(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned()).collect();
    Ok(Some(local_library::import_local_files(core.backend.app_state.clone(), paths).await?))
}

// ── Stats, recap, export ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_recap(core: S, range: String, from: Option<i64>, to: Option<i64>) -> R<RecapStats> {
    Ok(core.recap(&range, from.zip(to))?)
}

#[tauri::command]
pub fn weekly_recap_due(core: S) -> bool {
    core.weekly_recap_due()
}

#[tauri::command]
pub fn get_history_summary(core: S) -> R<PlayHistorySummary> {
    Ok(core.history_summary()?)
}

#[tauri::command]
pub fn get_recent_plays(core: S, limit: usize) -> R<Vec<RecentPlay>> {
    Ok(core.recent_plays(limit)?)
}

/// Serializes play history and asks the user where to save it. `false` if cancelled.
#[tauri::command]
pub async fn export_history(app: AppHandle, core: S<'_>, format: String) -> R<bool> {
    let contents = core.export_history(&format)?;
    let ext = if format == "json" { "json" } else { "csv" };
    save_with_dialog(app, format!("firmium-history.{ext}"), ext, contents.into_bytes()).await
}

/// Saves a PNG (base64 data URL from a rendered Recap card) via a save dialog.
#[tauri::command]
pub async fn save_image(app: AppHandle, default_name: String, png_base64: String) -> R<bool> {
    let b64 = png_base64.split(',').next_back().unwrap_or_default();
    let bytes = decode_base64(b64).ok_or_else(|| "invalid image data".to_string())?;
    save_with_dialog(app, default_name, "png", bytes).await
}

async fn save_with_dialog(app: AppHandle, name: String, ext: &'static str, bytes: Vec<u8>) -> R<bool> {
    let path = tauri::async_runtime::spawn_blocking(move || app.dialog().file().set_file_name(name).add_filter(ext, &[ext]).blocking_save_file())
        .await
        .map_err(|e| e.to_string())?;
    let Some(path) = path.and_then(|p| p.into_path().ok()) else { return Ok(false) };
    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    Ok(true)
}

fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

// ── Podcasts ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_podcast_channels(core: S) -> R<Vec<PodcastChannel>> {
    Ok(core.podcast_channels()?)
}

#[tauri::command]
pub fn get_podcast_episodes(core: S, channel_id: String) -> R<Vec<PodcastEpisode>> {
    Ok(core.podcast_episodes(channel_id)?)
}

#[tauri::command]
pub async fn add_podcast(core: S<'_>, feed_url: String) -> R<PodcastChannel> {
    Ok(core.add_podcast(feed_url).await?)
}

#[tauri::command]
pub async fn refresh_podcast(core: S<'_>, channel_id: String, feed_url: String) -> R<usize> {
    Ok(core.refresh_podcast(channel_id, feed_url).await?)
}

#[tauri::command]
pub fn unsubscribe_podcast(core: S, channel_id: String) -> R<()> {
    Ok(core.unsubscribe_podcast(channel_id)?)
}

#[tauri::command]
pub async fn play_podcast_episode(core: S<'_>, episode: PodcastEpisode, channel_title: String, cover_url: Option<String>) -> R<()> {
    Ok(core.play_podcast_episode(episode, channel_title, cover_url).await?)
}

// ── Window ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_window_decorations(window: tauri::WebviewWindow, enabled: bool) -> R<()> {
    window.set_decorations(enabled).map_err(|e| e.to_string().into())
}
