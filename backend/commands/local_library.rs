#![allow(dead_code)]

// ============================================================================
// LOCAL LIBRARY
// ============================================================================
// Scans `~/Music/Firmium` (the same folder used for downloads and drag-and-drop
// imports) and maps the contents into the same Album/Artist/Song shapes used by
// the OpenSubsonic API, so the existing UI works unchanged when the user isn't
// connected to a server.
//
// Local ids are `local:<md5 of a stable key>` (relative path for songs, lowercased
// artist/album name for artists/albums). The scan result is cached in
// `AppState.local_library` until `invalidate_local_library` is called (after a
// download or import completes).

use crate::commands::downloads::sanitize_path_component;
use crate::commands::mappers::{format_track_info, infer_release_type, Album, Artist, Song};
use crate::state::AppState;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "opus", "wav", "m4a", "aac", "alac", "aiff"];

/// Returns (and creates) `~/Music/Firmium`, the local library / download / import folder.
pub fn local_library_dir() -> PathBuf {
    let dir = crate::paths::audio_dir().join("Firmium");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn local_id(seed: &str) -> String {
    format!("local:{:x}", md5::compute(seed.as_bytes()))
}

/// Cached scan results, keyed for fast lookup by the `get_local_*` functions.
pub struct LocalLibraryCache {
    albums: Vec<Album>,
    artists: Vec<Artist>,
    songs_by_album: HashMap<String, Vec<Song>>,
    albums_by_artist: HashMap<String, Vec<Album>>,
    album_meta: HashMap<String, (String, String)>,
    artist_names: HashMap<String, String>,
    all_songs: Vec<Song>,
    album_mtime: HashMap<String, SystemTime>,
    /// Maps local song ids (also used as cover-art ids) to their file path.
    paths: HashMap<String, PathBuf>,
}

fn extension_lower(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "m4a" | "alac" => "audio/mp4",
        "aac" => "audio/aac",
        "aiff" => "audio/aiff",
        _ => "application/octet-stream",
    }
}

struct RawTrack {
    path: PathBuf,
    title: String,
    artist: String,
    album: String,
    album_artist: String,
    track_number: Option<u32>,
    year: Option<u32>,
    genre: Option<String>,
    duration: f64,
    bit_rate: Option<u32>,
    sampling_rate: Option<u32>,
    bit_depth: Option<u32>,
    has_picture: bool,
    suffix: String,
}

fn read_track(path: &Path) -> Option<RawTrack> {
    let tagged = lofty::read_from_path(path).ok()?;
    let properties = tagged.properties();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown Track").to_string();
    let title = tag.and_then(|t| t.title()).map(|s| s.to_string()).filter(|s| !s.is_empty()).unwrap_or(file_stem);
    let artist = tag.and_then(|t| t.artist()).map(|s| s.to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Unknown Artist".to_string());
    let album = tag.and_then(|t| t.album()).map(|s| s.to_string()).filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).map(str::to_string).unwrap_or_else(|| "Unknown Album".to_string()));
    let album_artist = tag.and_then(|t| t.get_string(ItemKey::AlbumArtist)).map(|s| s.to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| artist.clone());
    let track_number = tag.and_then(|t| t.track());
    let year = tag.and_then(|t| t.date()).map(|d| d.year as u32);
    let genre = tag.and_then(|t| t.genre()).map(|s| s.to_string()).filter(|s| !s.is_empty());
    let has_picture = tag.map(|t| !t.pictures().is_empty()).unwrap_or(false);

    Some(RawTrack {
        path: path.to_path_buf(),
        title,
        artist,
        album,
        album_artist,
        track_number,
        year,
        genre,
        duration: properties.duration().as_secs_f64(),
        bit_rate: properties.audio_bitrate(),
        sampling_rate: properties.sample_rate(),
        bit_depth: properties.bit_depth().map(|b| b as u32),
        has_picture,
        suffix: extension_lower(path),
    })
}

fn walk(dir: &Path, out: &mut Vec<RawTrack>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if AUDIO_EXTENSIONS.contains(&extension_lower(&path).as_str()) {
            if let Some(track) = read_track(&path) {
                out.push(track);
            }
        }
    }
}

fn scan() -> Result<LocalLibraryCache, String> {
    let root = local_library_dir();
    let mut raw = Vec::new();
    walk(&root, &mut raw);

    let mut songs_by_album: HashMap<String, Vec<Song>> = HashMap::new();
    let mut album_meta: HashMap<String, (String, String)> = HashMap::new();
    let mut artist_names: HashMap<String, String> = HashMap::new();
    let mut album_mtime: HashMap<String, SystemTime> = HashMap::new();
    let mut paths: HashMap<String, PathBuf> = HashMap::new();
    let mut album_song_count: HashMap<String, u32> = HashMap::new();
    let mut album_cover: HashMap<String, String> = HashMap::new();
    let mut album_year: HashMap<String, Option<u32>> = HashMap::new();
    let mut album_genres: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_songs: Vec<Song> = Vec::new();

    for raw_track in raw {
        let rel = raw_track.path.strip_prefix(&root).unwrap_or(&raw_track.path);
        let song_id = local_id(&rel.to_string_lossy());
        let artist_id = local_id(&format!("artist:{}", raw_track.album_artist.to_lowercase()));
        let album_id = local_id(&format!("album:{}|{}", raw_track.album_artist.to_lowercase(), raw_track.album.to_lowercase()));

        paths.insert(song_id.clone(), raw_track.path.clone());

        if raw_track.has_picture {
            album_cover.entry(album_id.clone()).or_insert_with(|| song_id.clone());
        }
        *album_song_count.entry(album_id.clone()).or_insert(0) += 1;
        album_year.entry(album_id.clone()).or_insert(raw_track.year);
        if let Some(g) = &raw_track.genre {
            album_genres.entry(album_id.clone()).or_default().insert(g.clone());
        }
        artist_names.entry(artist_id.clone()).or_insert_with(|| raw_track.album_artist.clone());
        album_meta.entry(album_id.clone()).or_insert_with(|| (raw_track.album.clone(), raw_track.album_artist.clone()));

        if let Some(parent) = raw_track.path.parent() {
            if let Ok(mtime) = std::fs::metadata(parent).and_then(|m| m.modified()) {
                album_mtime.entry(album_id.clone())
                    .and_modify(|m| if mtime > *m { *m = mtime })
                    .or_insert(mtime);
            }
        }

        let genres_json = raw_track.genre.as_ref().map(|g| serde_json::json!([{ "name": g }]));
        let track_info_json = serde_json::json!({
            "suffix": raw_track.suffix,
            "samplingRate": raw_track.sampling_rate,
            "bitDepth": raw_track.bit_depth,
            "bitRate": raw_track.bit_rate,
        });

        let song = Song {
            id: song_id.clone(),
            title: raw_track.title,
            artist: raw_track.artist,
            artist_id: Some(artist_id),
            album: raw_track.album,
            album_id: Some(album_id.clone()),
            duration: raw_track.duration,
            track_number: raw_track.track_number,
            cover_art_id: if raw_track.has_picture { Some(song_id) } else { None },
            replay_gain: None,
            bpm: None,
            comment: None,
            genres: genres_json,
            bit_rate: raw_track.bit_rate,
            sampling_rate: raw_track.sampling_rate,
            bit_depth: raw_track.bit_depth,
            suffix: Some(raw_track.suffix.clone()),
            content_type: Some(content_type_for(&raw_track.suffix).to_string()),
            track_info: format_track_info(&track_info_json),
            user_rating: None,
        };

        songs_by_album.entry(album_id).or_default().push(song.clone());
        all_songs.push(song);
    }

    for tracks in songs_by_album.values_mut() {
        tracks.sort_by_key(|s| s.track_number.unwrap_or(u32::MAX));
    }

    let mut albums: Vec<Album> = Vec::new();
    let mut albums_by_artist: HashMap<String, Vec<Album>> = HashMap::new();
    for (album_id, (name, album_artist)) in &album_meta {
        let artist_id = local_id(&format!("artist:{}", album_artist.to_lowercase()));
        let song_count = album_song_count.get(album_id).copied();
        let genres = album_genres.get(album_id).map(|set| {
            serde_json::Value::Array(set.iter().map(|g| serde_json::json!({ "name": g })).collect())
        });
        let album = Album {
            id: album_id.clone(),
            name: name.clone(),
            album_artist: album_artist.clone(),
            artist_id: Some(artist_id.clone()),
            cover_art_id: album_cover.get(album_id).cloned(),
            song_count,
            release_type: infer_release_type(&serde_json::json!({ "songCount": song_count.unwrap_or(0) })),
            genres,
            year: album_year.get(album_id).copied().flatten(),
            is_compilation: false,
        };
        albums_by_artist.entry(artist_id).or_default().push(album.clone());
        albums.push(album);
    }
    albums.sort_by_key(|a| a.name.to_lowercase());
    for albums in albums_by_artist.values_mut() {
        albums.sort_by_key(|a| a.name.to_lowercase());
    }

    let mut artists: Vec<Artist> = artist_names.iter().map(|(id, name)| {
        let album_count = albums_by_artist.get(id).map(|v| v.len() as u32).unwrap_or(0);
        Artist { id: id.clone(), name: name.clone(), album_count }
    }).collect();
    artists.sort_by_key(|a| a.name.to_lowercase());

    Ok(LocalLibraryCache {
        albums,
        artists,
        songs_by_album,
        albums_by_artist,
        album_meta,
        artist_names,
        all_songs,
        album_mtime,
        paths,
    })
}

impl LocalLibraryCache {
    pub fn has_local_match(&self, title: &str, album: &str) -> bool {
        let title_lc = title.to_lowercase();
        let album_lc = album.to_lowercase();
        self.all_songs.iter().any(|s| {
            s.title.to_lowercase() == title_lc && s.album.to_lowercase() == album_lc
        })
    }
}

fn ensure_scanned(state: &AppState) -> Result<(), String> {
    if state.local_library.read().is_some() {
        return Ok(());
    }
    let cache = scan()?;
    *state.local_library.write() = Some(cache);
    Ok(())
}

/// Forces a rescan on next access. Called after downloads/imports change the folder contents.
pub fn invalidate_local_library(state: &AppState) {
    *state.local_library.write() = None;
}

/// Returns a destination path under `dir` for `file_name`, appending " (1)", " (2)", etc.
/// if a file with that name already exists.
fn unique_dest(dir: &Path, file_name: &str) -> PathBuf {
    let dest = dir.join(file_name);
    if !dest.exists() {
        return dest;
    }
    let stem = Path::new(file_name).file_stem().and_then(|s| s.to_str()).unwrap_or(file_name);
    let ext = Path::new(file_name).extension().and_then(|e| e.to_str());
    let mut i = 1;
    loop {
        let candidate = match ext {
            Some(ext) => format!("{stem} ({i}).{ext}"),
            None => format!("{stem} ({i})"),
        };
        let dest = dir.join(candidate);
        if !dest.exists() {
            return dest;
        }
        i += 1;
    }
}

/// Copies one audio file into `root/<AlbumArtist>/<Album>/<filename>`, reading tags to
/// determine the destination subfolder. Returns `false` (without copying) if the file
/// can't be read as audio (e.g. not a supported format).
fn import_file(root: &Path, src: &Path) -> Result<bool, String> {
    let Some(track) = read_track(src) else { return Ok(false) };
    let dir = root
        .join(sanitize_path_component(&track.album_artist))
        .join(sanitize_path_component(&track.album));
    let file_name = src.file_name().and_then(|n| n.to_str()).unwrap_or("track").to_string();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = unique_dest(&dir, &file_name);
    std::fs::copy(src, dest).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Imports dropped files/folders into the local library, copying audio files into
/// `<AlbumArtist>/<Album>/<filename>` (handling name collisions). Non-audio files and
/// folders without recognized audio are skipped. Returns the number of files imported.
pub async fn import_local_files(state: Arc<AppState>, paths: Vec<String>) -> Result<usize, String> {
    let root = local_library_dir();
    let imported = tokio::task::spawn_blocking(move || -> Result<usize, String> {
        let mut count = 0;
        for p in paths {
            let path = PathBuf::from(p);
            if path.is_dir() {
                let mut raw = Vec::new();
                walk(&path, &mut raw);
                for track in raw {
                    if import_file(&root, &track.path)? {
                        count += 1;
                    }
                }
            } else if AUDIO_EXTENSIONS.contains(&extension_lower(&path).as_str()) && import_file(&root, &path)? {
                count += 1;
            }
        }
        Ok(count)
    }).await.map_err(|e| e.to_string())??;

    if imported > 0 {
        invalidate_local_library(&state);
    }
    Ok(imported)
}

pub fn get_local_albums(state: &AppState) -> Result<Vec<Album>, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    Ok(cache.as_ref().ok_or("Local library not loaded")?.albums.clone())
}

pub fn get_local_artists(state: &AppState) -> Result<Vec<Artist>, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    Ok(cache.as_ref().ok_or("Local library not loaded")?.artists.clone())
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LocalAlbumTracks {
    pub tracks: Vec<Song>,
    pub album_name: String,
    pub album_artist: String,
    pub cover_art_id: Option<String>,
}

pub fn get_local_album_tracks(state: &AppState, id: String) -> Result<LocalAlbumTracks, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    let cache = cache.as_ref().ok_or("Local library not loaded")?;
    let tracks = cache.songs_by_album.get(&id).cloned().unwrap_or_default();
    let (name, album_artist) = cache.album_meta.get(&id).cloned()
        .unwrap_or_else(|| ("Unknown Album".to_string(), "Unknown Artist".to_string()));
    let cover_art_id = cache.albums.iter().find(|a| a.id == id).and_then(|a| a.cover_art_id.clone());
    Ok(LocalAlbumTracks { tracks, album_name: name, album_artist, cover_art_id })
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LocalTrackKey {
    pub track_number: Option<u32>,
    pub title: String,
}

/// Returns the (track number, title) of every locally-present track for the given
/// album/artist, so the UI can mark matching server tracks as already downloaded.
pub fn get_local_album_track_keys(state: &AppState, album_artist: String, album: String) -> Result<Vec<LocalTrackKey>, String> {
    ensure_scanned(state)?;
    let album_id = local_id(&format!("album:{}|{}", album_artist.to_lowercase(), album.to_lowercase()));
    let cache = state.local_library.read();
    let cache = cache.as_ref().ok_or("Local library not loaded")?;
    Ok(cache.songs_by_album.get(&album_id)
        .map(|tracks| tracks.iter().map(|s| LocalTrackKey { track_number: s.track_number, title: s.title.clone() }).collect())
        .unwrap_or_default())
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LocalArtistDetails {
    pub name: String,
    pub albums: Vec<Album>,
}

pub fn get_local_artist_details(state: &AppState, id: String) -> Result<LocalArtistDetails, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    let cache = cache.as_ref().ok_or("Local library not loaded")?;
    let albums = cache.albums_by_artist.get(&id).cloned().unwrap_or_default();
    let name = cache.artist_names.get(&id).cloned().unwrap_or_else(|| "Unknown Artist".to_string());
    Ok(LocalArtistDetails { name, albums })
}

/// Resolves a `local:<hash>` song id to its absolute file path, for playback.
pub fn get_local_track_path(state: &AppState, id: &str) -> Result<String, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    cache.as_ref().ok_or("Local library not loaded")?.paths.get(id)
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or_else(|| "Track not found".to_string())
}

/// Best-effort path lookup for a song matching title + (album OR artist), used by
/// the queue manager to prefer a local copy over streaming.
pub(crate) fn find_local_match_internal(state: &AppState, title: &str, artist: &str, album: &str) -> Option<String> {
    ensure_scanned(state).ok()?;
    let cache = state.local_library.read();
    let cache = cache.as_ref()?;
    let title_lc = title.to_lowercase();
    let artist_lc = artist.to_lowercase();
    let album_lc = album.to_lowercase();
    let found = cache.all_songs.iter().find(|s| {
        s.title.to_lowercase() == title_lc
            && (s.album.to_lowercase() == album_lc || s.artist.to_lowercase() == artist_lc)
    })?;
    cache.paths.get(&found.id).map(|p| p.to_string_lossy().into_owned())
}

fn find_cached(dir: &Path, safe_id: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let prefix = format!("{safe_id}.");
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            return Some(entry.path());
        }
    }
    None
}

/// Extracts the embedded cover picture for a local song/album id, caches it under
/// `<cache_dir>/local_covers/`, and returns the cached file path (mirrors `get_cover_art`).
pub fn get_local_cover_art(state: &AppState, id: String) -> Result<String, String> {
    ensure_scanned(state)?;
    let path = {
        let cache = state.local_library.read();
        cache.as_ref().ok_or("Local library not loaded")?.paths.get(&id).cloned().ok_or_else(|| "Cover not found".to_string())?
    };

    let safe_id = id.replace([':', '/', '\\'], "_");
    let dir = crate::paths::cache_dir().join("local_covers");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    if let Some(cached) = find_cached(&dir, &safe_id) {
        return Ok(cached.to_string_lossy().into_owned());
    }

    let tagged = lofty::read_from_path(&path).map_err(|e| e.to_string())?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag()).ok_or("No tags")?;
    let picture = tag.pictures().first().ok_or("No embedded cover art")?;
    let ext = match picture.mime_type() {
        Some(lofty::picture::MimeType::Png) => "png",
        Some(lofty::picture::MimeType::Gif) => "gif",
        Some(lofty::picture::MimeType::Bmp) => "bmp",
        _ => "jpg",
    };
    let out_path = dir.join(format!("{safe_id}.{ext}"));
    std::fs::write(&out_path, picture.data()).map_err(|e| e.to_string())?;
    Ok(out_path.to_string_lossy().into_owned())
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LocalSearchResult {
    pub songs: Vec<Song>,
    pub albums: Vec<Album>,
}

pub fn search_local(state: &AppState, query: String) -> Result<LocalSearchResult, String> {
    ensure_scanned(state)?;
    let q = query.to_lowercase();
    let cache = state.local_library.read();
    let cache = cache.as_ref().ok_or("Local library not loaded")?;
    let songs = cache.all_songs.iter()
        .filter(|s| s.title.to_lowercase().contains(&q) || s.artist.to_lowercase().contains(&q) || s.album.to_lowercase().contains(&q))
        .take(100).cloned().collect();
    let albums = cache.albums.iter()
        .filter(|a| a.name.to_lowercase().contains(&q) || a.album_artist.to_lowercase().contains(&q))
        .take(40).cloned().collect();
    Ok(LocalSearchResult { songs, albums })
}

pub fn get_local_recent_albums(state: &AppState, size: u32) -> Result<Vec<Album>, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    let cache = cache.as_ref().ok_or("Local library not loaded")?;
    let mut albums = cache.albums.clone();
    albums.sort_by(|a, b| cache.album_mtime.get(&b.id).cmp(&cache.album_mtime.get(&a.id)));
    albums.truncate(size as usize);
    Ok(albums)
}

pub fn get_local_random_albums(state: &AppState, size: u32) -> Result<Vec<Album>, String> {
    use rand::seq::SliceRandom;
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    let mut albums = cache.as_ref().ok_or("Local library not loaded")?.albums.clone();
    albums.shuffle(&mut rand::rng());
    albums.truncate(size as usize);
    Ok(albums)
}

pub fn get_local_newest_albums(state: &AppState, size: u32) -> Result<Vec<Album>, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    let mut albums = cache.as_ref().ok_or("Local library not loaded")?.albums.clone();
    albums.sort_by_key(|a| std::cmp::Reverse(a.year));
    albums.truncate(size as usize);
    Ok(albums)
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LocalGenre {
    pub name: String,
    pub album_count: u32,
    pub song_count: u32,
}

/// Returns the absolute file path of a locally-downloaded track that case-insensitively
/// matches title + (album OR artist). Used to prefer the local copy over streaming.
pub fn find_local_match(state: &AppState, title: String, artist: String, album: String) -> Result<Option<String>, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    let cache = cache.as_ref().ok_or("Local library not loaded")?;
    let title_lc = title.to_lowercase();
    let artist_lc = artist.to_lowercase();
    let album_lc = album.to_lowercase();
    let found = cache.all_songs.iter().find(|s| {
        s.title.to_lowercase() == title_lc
            && (s.album.to_lowercase() == album_lc || s.artist.to_lowercase() == artist_lc)
    });
    Ok(found.and_then(|s| cache.paths.get(&s.id)).map(|p| p.to_string_lossy().into_owned()))
}

/// Triggers an eager scan of the local library so subsequent lookups are instant.
pub fn prewarm_local_library(state: &AppState) -> Result<(), String> {
    ensure_scanned(state)
}

pub fn get_local_genres_list(state: &AppState) -> Result<Vec<LocalGenre>, String> {
    ensure_scanned(state)?;
    let cache = state.local_library.read();
    let cache = cache.as_ref().ok_or("Local library not loaded")?;
    let mut song_counts: HashMap<String, u32> = HashMap::new();
    let mut album_genre_sets: HashMap<String, HashSet<String>> = HashMap::new();
    for song in &cache.all_songs {
        let Some(arr) = song.genres.as_ref().and_then(|g| g.as_array()) else { continue };
        for g in arr {
            let Some(name) = g.get("name").and_then(|v| v.as_str()) else { continue };
            *song_counts.entry(name.to_string()).or_insert(0) += 1;
            if let Some(album_id) = &song.album_id {
                album_genre_sets.entry(name.to_string()).or_default().insert(album_id.clone());
            }
        }
    }
    let mut genres: Vec<LocalGenre> = song_counts.into_iter().map(|(name, song_count)| {
        let album_count = album_genre_sets.get(&name).map(|s| s.len() as u32).unwrap_or(0);
        LocalGenre { name, album_count, song_count }
    }).collect();
    genres.sort_by_key(|g| std::cmp::Reverse(g.album_count));
    Ok(genres)
}
