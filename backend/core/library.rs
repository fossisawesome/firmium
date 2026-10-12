//! Data-source routing and library-level features. When logged in, reads go to
//! the OpenSubsonic server; otherwise they fall back to the offline library in
//! `~/Music/Firmium`. Also holds Mix / Start Radio / auto-continue seeding.

use std::collections::HashSet;
use std::sync::Arc;

use rand::seq::SliceRandom;

use super::Core;
use crate::commands::lyrics::LyricsResult;
use crate::commands::mappers::{Album, Artist, SimilarMatch, Song};
use crate::commands::subsonic::{self, AlbumTracks, ArtistDetails, ArtistInfo, Genre, SearchResult};
use crate::commands::{cover_cache, local_library as local, queue};
use crate::errors::UserError;
use crate::events::BackendEvent;

type R<T> = Result<T, UserError>;

/// One word of a synced lyric line with its estimated time window (ms).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordTiming {
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TimedLine {
    pub start: i64,
    pub value: String,
    pub words: Vec<WordTiming>,
}

/// Lyrics plus per-word timing for the karaoke fill animation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimedLyrics {
    pub lines: Vec<TimedLine>,
    pub synced: bool,
}

impl TimedLyrics {
    /// Spreads each line's duration (to the next line's start, or the track end
    /// for the last line) across its words in proportion to character length.
    pub fn new(r: LyricsResult, track_ms: f64) -> Self {
        let starts: Vec<i64> = r.lines.iter().map(|l| l.start).collect();
        let lines = r
            .lines
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                let words = if r.synced {
                    let end = starts.get(i + 1).map_or_else(|| track_ms.max(l.start as f64), |&n| n as f64);
                    word_timings(&l.value, l.start as f64, end)
                } else {
                    Vec::new()
                };
                TimedLine { start: l.start, value: l.value, words }
            })
            .collect();
        TimedLyrics { lines, synced: r.synced }
    }
}

fn word_timings(text: &str, start: f64, end: f64) -> Vec<WordTiming> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let n = words.len();
    let weights: Vec<f64> = words.iter().enumerate().map(|(i, w)| (w.chars().count() + usize::from(i + 1 < n)) as f64).collect();
    let total = weights.iter().sum::<f64>().max(1.0);
    let span = (end - start).max(0.0);
    let mut cursor = start;
    words
        .iter()
        .zip(&weights)
        .map(|(w, wt)| {
            let s = cursor;
            cursor = s + wt / total * span;
            WordTiming { text: (*w).to_string(), start_ms: s, end_ms: cursor }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::word_timings;

    #[test]
    fn words_cover_line_span_proportionally() {
        let w = word_timings("aa bbbb", 1000.0, 2000.0);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].start_ms, 1000.0);
        assert!((w[1].end_ms - 2000.0).abs() < 1e-6);
        assert!(w[0].end_ms < w[1].end_ms);
    }
}

fn local_err(e: String) -> UserError {
    UserError::classify(&e)
}

const RADIO_BATCH: usize = 10;
const BPM_TOLERANCE: f64 = 15.0;
const POOL_SIZE: i32 = 500;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Energy {
    Chill,
    Mid,
    High,
}

/// Keep songs whose BPM falls in the energy band; fall back to the whole pool
/// when too few carry a BPM tag. Caps at 60 tracks.
fn filter_energy(songs: Vec<Song>, e: Energy) -> Vec<Song> {
    let in_band = |s: &Song| match s.bpm {
        Some(b) => match e {
            Energy::Chill => b < 95.0,
            Energy::Mid => (95.0..130.0).contains(&b),
            Energy::High => b >= 130.0,
        },
        None => false,
    };
    let filtered: Vec<Song> = songs.iter().filter(|s| in_band(s)).cloned().collect();
    let mut out = if filtered.len() >= 10 { filtered } else { songs };
    out.truncate(60);
    out
}

/// First genre name of a song's raw `genres` field (strings, `{name}` objects or one string).
fn genre_of(song: &Song) -> Option<String> {
    match song.genres.as_ref()? {
        serde_json::Value::Array(a) => a.first().and_then(|g| match g {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(o) => o.get("name").and_then(|n| n.as_str()).map(String::from),
            _ => None,
        }),
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

impl Core {
    pub fn is_authed(&self) -> bool {
        self.auth_state().authed
    }

    fn st(&self) -> Arc<crate::state::AppState> {
        self.backend.app_state.clone()
    }

    pub async fn albums(&self) -> R<Vec<Album>> {
        if self.is_authed() { return subsonic::get_albums(self.st()).await; }
        local::ensure_scanned_async(self.st()).await.map_err(local_err)?;
        local::get_local_albums(&self.st()).map_err(local_err)
    }

    pub async fn artists(&self) -> R<Vec<Artist>> {
        if self.is_authed() { return subsonic::get_artists(self.st()).await; }
        local::ensure_scanned_async(self.st()).await.map_err(local_err)?;
        local::get_local_artists(&self.st()).map_err(local_err)
    }

    pub async fn album_tracks(&self, id: String) -> R<AlbumTracks> {
        if self.is_authed() {
            return subsonic::get_album_tracks(self.st(), id).await;
        }
        local::ensure_scanned_async(self.st()).await.map_err(local_err)?;
        let a = local::get_local_album_tracks(&self.st(), id).map_err(local_err)?;
        Ok(AlbumTracks { tracks: a.tracks, album_name: a.album_name, album_artist: a.album_artist, cover_art_id: a.cover_art_id, starred: false })
    }

    pub async fn artist_details(&self, id: String) -> R<ArtistDetails> {
        if self.is_authed() {
            return subsonic::get_artist_details(self.st(), id).await;
        }
        local::ensure_scanned_async(self.st()).await.map_err(local_err)?;
        let a = local::get_local_artist_details(&self.st(), id).map_err(local_err)?;
        Ok(ArtistDetails { name: a.name, albums: a.albums })
    }

    /// Bio + image. Uses the stored Last.fm key when set; none offline.
    pub async fn artist_info(&self, id: String, artist_name: String) -> R<Option<ArtistInfo>> {
        if !self.is_authed() {
            return Ok(None);
        }
        subsonic::get_artist_info(self.st(), id, self.settings().lastfm_key, artist_name).await
    }

    pub async fn search(&self, query: String) -> R<SearchResult> {
        if self.is_authed() {
            return subsonic::search(self.st(), query).await;
        }
        local::ensure_scanned_async(self.st()).await.map_err(local_err)?;
        let r = local::search_local(&self.st(), query).map_err(local_err)?;
        Ok(SearchResult { songs: r.songs, albums: r.albums })
    }

    /// `kind`: "recent" | "random" | "newest".
    pub async fn album_list(&self, kind: &str, size: u32) -> R<Vec<Album>> {
        let st = self.st();
        if !self.is_authed() {
            local::ensure_scanned_async(st.clone()).await.map_err(local_err)?;
        }
        match (self.is_authed(), kind) {
            (true, "recent") => subsonic::get_recent_albums(st, size).await,
            (true, "random") => subsonic::get_random_albums(st, size).await,
            (true, "newest") => subsonic::get_newest_albums(st, size).await,
            (false, "recent") => local::get_local_recent_albums(&st, size).map_err(local_err),
            (false, "random") => local::get_local_random_albums(&st, size).map_err(local_err),
            (false, "newest") => local::get_local_newest_albums(&st, size).map_err(local_err),
            _ => Err(UserError::Unknown),
        }
    }

    pub async fn genres(&self) -> R<Vec<Genre>> {
        if self.is_authed() {
            return subsonic::get_genres_list(self.st()).await;
        }
        local::ensure_scanned_async(self.st()).await.map_err(local_err)?;
        let g = local::get_local_genres_list(&self.st()).map_err(local_err)?;
        Ok(g.into_iter().map(|g| Genre { name: g.name, album_count: g.album_count, song_count: g.song_count }).collect())
    }

    pub async fn lyrics(&self, song: &Song) -> R<Option<TimedLyrics>> {
        if !self.is_authed() {
            return Ok(None);
        }
        let res = subsonic::get_song_lyrics(self.st(), song.id.clone(), song.artist.clone(), song.title.clone(), song.duration, self.settings().lrclib_enabled).await?;
        Ok(res.map(|r| TimedLyrics::new(r, song.duration * 1000.0)))
    }

    /// Cover art file path. `local:` ids resolve from embedded art; server ids
    /// build the URL here and go through the disk cache.
    pub async fn cover_path(&self, cover_id: String) -> Result<String, String> {
        if cover_id.starts_with("local:") {
            return local::get_local_cover_art_async(self.st(), cover_id).await;
        }
        // Podcast artwork is a plain image URL rather than a server cover id.
        if cover_id.starts_with("http") {
            let key = format!("ext-{:x}", md5::compute(&cover_id));
            return cover_cache::get_cover_art(key, cover_id).await;
        }
        let url = subsonic::build_cover_url(&self.st(), &cover_id, 300)?;
        cover_cache::get_cover_art(self.server_cover_key(&cover_id), url).await
    }

    /// Cache key for a server cover id. Scoped by server URL so two saved
    /// servers that reuse the same cover id don't share cached art.
    fn server_cover_key(&self, cover_id: &str) -> String {
        let server = self.st().connection.read().server.clone().unwrap_or_default();
        format!("{:x}-{cover_id}", md5::compute(server.as_bytes()))
    }

    pub async fn cover_colors(&self, cover_id: String) -> Result<crate::commands::cover_colors::CoverColorsResult, String> {
        if cover_id.starts_with("local:") {
            let path = self.cover_path(cover_id).await?;
            return crate::commands::cover_colors::extract_cover_colors_from_path(path).await;
        }
        let url = subsonic::build_cover_url(&self.st(), &cover_id, 300)?;
        crate::commands::cover_colors::extract_cover_colors(self.server_cover_key(&cover_id), url).await
    }

    // ── Mix / Radio ───────────────────────────────────────────────────────────

    async fn library_pool(&self, genre: Option<String>) -> Vec<Song> {
        let st = self.st();
        let res = match genre {
            Some(g) => subsonic::get_songs_by_genre(st, g, Some(POOL_SIZE)).await,
            None => subsonic::get_random_songs(st, Some(POOL_SIZE), None).await,
        };
        res.unwrap_or_default()
    }

    /// Seeding cascade for radio/auto-continue: server similar tracks first,
    /// then the library filtered by genre + BPM of the seed.
    pub async fn seed_from(&self, seed: &Song, exclude: &HashSet<String>, count: usize) -> Vec<Song> {
        let mut skip: HashSet<String> = exclude.clone();
        skip.insert(seed.id.clone());
        let mut out: Vec<Song> = Vec::new();
        let push = |s: Song, skip: &mut HashSet<String>, out: &mut Vec<Song>| {
            if skip.insert(s.id.clone()) {
                out.push(s);
            }
        };

        let st = self.st();
        let sonic = subsonic::get_open_subsonic_extensions(&st).iter().any(|e| e == "sonicSimilarity");
        let n = Some((count * 2) as i32);
        let matches: Vec<SimilarMatch> = if sonic {
            subsonic::get_sonic_similar_tracks(st, seed.id.clone(), n).await
        } else {
            subsonic::get_similar_tracks_fallback(st, seed.id.clone(), seed.artist_id.clone(), genre_of(seed), n).await
        }
        .unwrap_or_default();
        for m in matches {
            push(m.song, &mut skip, &mut out);
        }

        if out.len() < count {
            let mut pool = self.library_pool(genre_of(seed)).await;
            pool.retain(|s| match seed.bpm {
                None => true,
                Some(b) => s.bpm.is_some_and(|x| (x - b).abs() <= BPM_TOLERANCE),
            });
            pool.shuffle(&mut rand::rng());
            for s in pool {
                push(s, &mut skip, &mut out);
            }
        }
        out.truncate(count);
        out
    }

    /// Mood Mix: random library tracks in an energy band, queued and played.
    pub async fn generate_mix(&self, energy: Energy, genre: Option<String>) -> Result<usize, String> {
        let pool = self.library_pool(genre).await;
        let mix = filter_energy(pool, energy);
        let n = mix.len();
        if n == 0 {
            return Ok(0);
        }
        let b = &self.backend;
        queue::set_queue(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone(), mix, 0).await?;
        Ok(n)
    }

    /// Shuffle All: a random sample of the whole library, played immediately. Returns the
    /// queue length (0 when the library is empty or unreachable).
    pub async fn shuffle_all(&self) -> Result<usize, String> {
        let mut songs = self.library_pool(None).await;
        songs.shuffle(&mut rand::rng());
        let n = songs.len();
        if n == 0 {
            return Ok(0);
        }
        let b = &self.backend;
        queue::set_queue(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone(), songs, 0).await?;
        Ok(n)
    }

    /// Start Radio: seed track plus similar tracks, played immediately.
    pub async fn start_radio(&self, seed: Song) -> Result<(), String> {
        let seeded = self.seed_from(&seed, &HashSet::new(), RADIO_BATCH).await;
        let mut tracks = vec![seed];
        tracks.extend(seeded);
        let b = &self.backend;
        queue::set_queue(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone(), tracks, 0).await
    }

    /// Background task: appends similar tracks when the queue ends and
    /// auto-continue is on. Tracks played this session are excluded.
    pub fn start_auto_continue(self: &Arc<Self>) {
        let core = Arc::clone(self);
        let mut rx = core.backend.bus.subscribe();
        tokio::spawn(async move {
            let mut played: HashSet<String> = HashSet::new();
            let mut current: Option<String> = None;
            let mut last_saved = std::time::Instant::now();
            loop {
                match rx.recv().await {
                    Ok(BackendEvent::QueueStateChanged(s)) => {
                        current = usize::try_from(s.queue_idx).ok().and_then(|i| s.queue.get(i)).map(|t| t.id.clone());
                        if let Some(id) = &current {
                            played.insert(id.clone());
                        }
                    }
                    // Persist podcast resume position every ~5 s.
                    Ok(BackendEvent::PlaybackPosition { position, .. }) => {
                        if let (Some(ep), Some(store)) = (current.as_deref().and_then(|c| c.strip_prefix("podcast:")), core.backend.podcasts.as_ref()) {
                            if last_saved.elapsed() >= std::time::Duration::from_secs(5) {
                                last_saved = std::time::Instant::now();
                                let _ = store.update_position(ep, (position * 1000.0) as i64);
                            }
                        }
                    }
                    Ok(BackendEvent::QueueExhausted(seed)) => {
                        let seeded = core.seed_from(&seed, &played, RADIO_BATCH).await;
                        if seeded.is_empty() {
                            continue;
                        }
                        let b = &core.backend;
                        let mut all = b.queue_state.inner.lock().queue.clone();
                        let start = all.len();
                        all.extend(seeded);
                        if let Err(e) = queue::set_queue(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone(), all, start).await {
                            eprintln!("auto-continue failed: {e}");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    _ => {}
                }
            }
        });
    }
}

// ── Podcasts ─────────────────────────────────────────────────────────────────
use crate::podcasts::{self, PodcastChannel, PodcastEpisode};

impl Core {
    fn podcast_store(&self) -> Result<Arc<podcasts::PodcastStore>, String> {
        self.backend.podcasts.clone().ok_or_else(|| "Podcast database unavailable".to_string())
    }

    pub fn podcast_channels(&self) -> Result<Vec<PodcastChannel>, String> {
        podcasts::list_channels(self.podcast_store()?)
    }

    pub fn podcast_episodes(&self, channel_id: String) -> Result<Vec<PodcastEpisode>, String> {
        podcasts::list_episodes(self.podcast_store()?, channel_id)
    }

    pub async fn add_podcast(&self, feed_url: String) -> Result<PodcastChannel, String> {
        podcasts::add_channel(self.st(), self.podcast_store()?, feed_url.trim().to_string()).await
    }

    pub async fn refresh_podcast(&self, channel_id: String, feed_url: String) -> Result<usize, String> {
        podcasts::refresh_channel(self.st(), self.podcast_store()?, channel_id, feed_url).await
    }

    pub fn unsubscribe_podcast(&self, channel_id: String) -> Result<(), String> {
        podcasts::unsubscribe(self.podcast_store()?, channel_id)
    }

    /// Plays an episode through the main queue so the player bar and transport
    /// controls treat it like any track. Resumes from the saved position.
    pub async fn play_podcast_episode(&self, episode: PodcastEpisode, channel_title: String, cover_url: Option<String>) -> Result<(), String> {
        let id = format!("podcast:{}", episode.id);
        self.backend.app_state.external_streams.write().insert(id.clone(), episode.audio_url.clone());
        let song: Song = serde_json::from_value(serde_json::json!({
            "id": id,
            "title": episode.title,
            "artist": channel_title,
            "album": channel_title,
            "duration": episode.duration_seconds.unwrap_or(0) as f64,
            "coverArtId": cover_url,
            "starred": false,
        }))
        .map_err(|e| e.to_string())?;
        let b = &self.backend;
        queue::set_queue(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone(), vec![song], 0).await?;
        if episode.position_ms > 0 {
            queue::seek_queue(&b.queue_state, &b.audio_player, episode.position_ms as f64 / 1000.0)?;
        }
        Ok(())
    }
}

// ── Artists ──────────────────────────────────────────────────────────────────
impl Core {
    /// Artist photo URL, cached for the session. `None` offline or when the server has none.
    pub async fn artist_image(&self, id: String) -> Option<String> {
        if !self.is_authed() {
            return None;
        }
        if let Some(hit) = self.artist_images.lock().get(&id) {
            return hit.clone();
        }
        let image = subsonic::get_artist_info(self.st(), id.clone(), String::new(), String::new())
            .await
            .ok()
            .flatten()
            .and_then(|i| i.image);
        self.artist_images.lock().insert(id, image.clone());
        image
    }

    /// Plays every track across all of an artist's albums. Returns the track count.
    pub async fn play_artist(&self, id: String, shuffle: bool) -> Result<usize, String> {
        let details = self.artist_details(id).await.map_err(|e| e.message())?;
        let mut tracks: Vec<Song> = Vec::new();
        for album in &details.albums {
            tracks.extend(self.album_tracks(album.id.clone()).await.map_err(|e| e.message())?.tracks);
        }
        let n = tracks.len();
        if n == 0 {
            return Ok(0);
        }
        let b = &self.backend;
        if shuffle {
            queue::shuffle_and_play(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone(), tracks).await?;
        } else {
            queue::set_queue_seamless(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone(), tracks, 0).await?;
        }
        Ok(n)
    }

    /// Similar artists (server first, Last.fm fallback) that exist in the user's library.
    pub async fn artist_recommendations(&self, id: String, name: String) -> Vec<Artist> {
        if !self.is_authed() {
            return Vec::new();
        }
        let mut names = subsonic::get_similar_artists(self.st(), id.clone(), None).await.unwrap_or_default();
        if names.is_empty() {
            let key = self.settings().lastfm_key;
            if !key.is_empty() {
                names = self.lastfm_similar(&name, &key).await;
            }
        }
        if names.is_empty() {
            return Vec::new();
        }
        let library = self.artists().await.unwrap_or_default();
        let by_name: std::collections::HashMap<String, &Artist> = library.iter().map(|a| (a.name.to_lowercase(), a)).collect();
        let mut seen: HashSet<String> = HashSet::from([id]);
        let mut out = Vec::new();
        for n in names {
            if let Some(hit) = by_name.get(&n.to_lowercase()) {
                if seen.insert(hit.id.clone()) {
                    out.push((*hit).clone());
                }
            }
        }
        out.truncate(12);
        out
    }

    async fn lastfm_similar(&self, artist: &str, key: &str) -> Vec<String> {
        let url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json&limit=40",
            urlencode(artist),
            urlencode(key)
        );
        let Ok(res) = self.backend.app_state.http.get(url).send().await else { return Vec::new() };
        let Ok(body) = res.json::<serde_json::Value>().await else { return Vec::new() };
        match body.pointer("/similarartists/artist") {
            Some(serde_json::Value::Array(a)) => a.iter().filter_map(|x| x.get("name").and_then(|n| n.as_str()).map(String::from)).collect(),
            Some(o @ serde_json::Value::Object(_)) => o.get("name").and_then(|n| n.as_str()).map(String::from).into_iter().collect(),
            _ => Vec::new(),
        }
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
