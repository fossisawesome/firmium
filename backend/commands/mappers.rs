// ============================================================================
// OPENSUBSONIC DATA MAPPERS
// ============================================================================
// These structs mirror the shapes that the JS API layer produces from raw
// Subsonic JSON. Mapping on the Rust side gives us exhaustive pattern matching
// for release-type inference and keeps transform logic out of JS.

/// Mapped album.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub name: String,
    pub album_artist: String,
    pub artist_id: Option<String>,
    pub cover_art_id: Option<String>,
    pub song_count: Option<u32>,
    pub release_type: String,
    pub genres: Option<serde_json::Value>,
    pub year: Option<u32>,
    pub is_compilation: bool,
    pub starred: bool,
}

/// Mapped artist.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub album_count: u32,
}

/// Mapped song.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<String>,
    pub album: String,
    pub album_id: Option<String>,
    pub duration: f64,
    pub track_number: Option<u32>,
    pub cover_art_id: Option<String>,
    pub replay_gain: Option<serde_json::Value>,
    pub bpm: Option<f64>,
    pub comment: Option<String>,
    pub genres: Option<serde_json::Value>,
    pub bit_rate: Option<u32>,
    pub sampling_rate: Option<u32>,
    pub bit_depth: Option<u32>,
    pub suffix: Option<String>,
    pub content_type: Option<String>,
    pub track_info: Option<String>,
    pub user_rating: Option<u32>,
    pub average_rating: Option<f32>,
    pub starred: bool,
}

impl Song {
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Formats a "FLAC · 44.1 kHz · 16-bit · 1234 kbps"-style summary of a song's
/// audio format, for display in the player bar.
pub(crate) fn format_track_info(s: &serde_json::Value) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(suffix) = s.get("suffix").and_then(|v| v.as_str()) {
        if !suffix.is_empty() {
            parts.push(suffix.to_uppercase());
        }
    }
    if let Some(rate) = s.get("samplingRate").and_then(|v| v.as_f64()) {
        if rate != 0.0 {
            let khz = rate / 1000.0;
            let formatted = format!("{khz:.1}");
            let formatted = formatted.strip_suffix(".0").unwrap_or(&formatted);
            parts.push(format!("{formatted} kHz"));
        }
    }
    if let Some(depth) = s.get("bitDepth").and_then(|v| v.as_u64()) {
        if depth != 0 {
            parts.push(format!("{depth}-bit"));
        }
    }
    if let Some(rate) = s.get("bitRate").and_then(|v| v.as_u64()) {
        if rate != 0 {
            parts.push(format!("{rate} kbps"));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

/// Infer release type from explicit server fields, title keywords, and song count.
/// Equivalent to the release-type inference originally in the frontend's api.ts (now
/// superseded; see ApiClient.kt for the Android equivalent, which uses a different taxonomy).
pub(crate) fn infer_release_type(a: &serde_json::Value) -> String {
    // Prefer explicit releaseTypes[] array, then releaseType string.
    let explicit = a.get("releaseTypes")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .or_else(|| a.get("releaseType").and_then(|v| v.as_str()));

    if let Some(t) = explicit {
        return t.to_lowercase();
    }

    let title = a.get("name").or_else(|| a.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    if title.contains(" - single") || title.ends_with("(single)") || title.ends_with("- single") {
        return "single".to_string();
    }
    if title.contains(" - ep") || title.ends_with("(ep)") || title.ends_with("- ep") {
        return "ep".to_string();
    }

    let count = a.get("songCount").and_then(|v| v.as_u64()).unwrap_or(0);
    match count {
        1 | 2 => "single".to_string(),
        3..=6 => "ep".to_string(),
        _ => "album".to_string(),
    }
}

fn map_album(a: &serde_json::Value) -> Album {
    Album {
        id: a.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        name: a.get("name").or_else(|| a.get("title"))
            .and_then(|v| v.as_str()).unwrap_or("Unknown Album").to_string(),
        album_artist: a.get("displayArtist").or_else(|| a.get("artist"))
            .and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        artist_id: a.get("artistId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        cover_art_id: a.get("coverArt").and_then(|v| v.as_str()).map(|s| s.to_string()),
        song_count: a.get("songCount").and_then(|v| v.as_u64()).map(|n| n as u32),
        release_type: infer_release_type(a),
        genres: a.get("genres").cloned(),
        year: a.get("year").and_then(|v| v.as_u64()).map(|n| n as u32),
        is_compilation: a.get("isCompilation").and_then(|v| v.as_bool()).unwrap_or(false),
        starred: a.get("starred").is_some_and(|v| !v.is_null()),
    }
}

fn map_artist(a: &serde_json::Value) -> Artist {
    Artist {
        id: a.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        name: a.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        album_count: a.get("albumCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    }
}

fn map_song(s: &serde_json::Value) -> Song {
    Song {
        id: s.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        title: s.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown Track").to_string(),
        artist: s.get("displayArtist").or_else(|| s.get("artist"))
            .and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string(),
        artist_id: s.get("artistId").and_then(|v| v.as_str()).map(str::to_string),
        album: s.get("album").and_then(|v| v.as_str()).unwrap_or("Unknown Album").to_string(),
        album_id: s.get("albumId").and_then(|v| v.as_str()).map(|v| v.to_string()),
        duration: s.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0),
        track_number: s.get("track").and_then(|v| v.as_u64()).map(|n| n as u32),
        cover_art_id: s.get("coverArt").and_then(|v| v.as_str()).map(|v| v.to_string()),
        replay_gain: s.get("replayGain").cloned(),
        bpm: s.get("bpm").and_then(|v| v.as_f64()),
        comment: s.get("comment").and_then(|v| v.as_str()).map(|v| v.to_string()),
        genres: s.get("genres").cloned(),
        bit_rate: s.get("bitRate").and_then(|v| v.as_u64()).map(|n| n as u32),
        sampling_rate: s.get("samplingRate").and_then(|v| v.as_u64()).map(|n| n as u32),
        bit_depth: s.get("bitDepth").and_then(|v| v.as_u64()).map(|n| n as u32),
        suffix: s.get("suffix").and_then(|v| v.as_str()).map(|v| v.to_string()),
        content_type: s.get("contentType").and_then(|v| v.as_str()).map(|v| v.to_string()),
        track_info: format_track_info(s),
        user_rating: s.get("userRating").and_then(|v| v.as_u64()).map(|n| n as u32),
        average_rating: s.get("averageRating").and_then(|v| v.as_f64()).map(|n| n as f32).filter(|&n| n > 0.0),
        starred: s.get("starred").is_some_and(|v| !v.is_null()),
    }
}

/// Map a batch of raw Subsonic album objects to typed Album structs.
pub fn map_albums(albums: Vec<serde_json::Value>) -> Vec<Album> {
    albums.iter().map(map_album).collect()
}

/// Map a batch of raw Subsonic artist objects to typed Artist structs.
pub fn map_artists(artists: Vec<serde_json::Value>) -> Vec<Artist> {
    artists.iter().map(map_artist).collect()
}

/// Map a batch of raw Subsonic song objects to typed Song structs.
pub fn map_songs(songs: Vec<serde_json::Value>) -> Vec<Song> {
    songs.iter().map(map_song).collect()
}

/// A song paired with a similarity score, returned by the `sonicSimilarity`
/// OpenSubsonic extension (`getSonicSimilarTracks`/`findSonicPath`).
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimilarMatch {
    pub song: Song,
    pub similarity: f64,
}

impl SimilarMatch {
    pub fn new(song: Song, similarity: f64) -> Self {
        SimilarMatch { song, similarity }
    }
}

/// Map a batch of raw `sonicMatches` entries (`{entry, similarity}`) to typed SimilarMatch structs.
#[allow(dead_code)]
pub fn map_similar_matches(matches: Vec<serde_json::Value>) -> Vec<SimilarMatch> {
    matches
        .iter()
        .map(|m| SimilarMatch {
            song: map_song(m.get("entry").unwrap_or(&serde_json::Value::Null)),
            similarity: m.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0),
        })
        .collect()
}

/// All items the user has starred, as returned by `getStarred2`.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Starred {
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub songs: Vec<Song>,
}

/// Maps a raw `getStarred2` response body (the `subsonic-response` object) into a `Starred`.
pub fn map_starred(body: &serde_json::Value) -> Starred {
    let root = body.get("starred2");
    let list = |key: &str| -> Vec<serde_json::Value> {
        root.and_then(|r| r.get(key))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };
    Starred {
        artists: map_artists(list("artist")),
        albums: map_albums(list("album")),
        songs: map_songs(list("song")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- infer_release_type ---

    #[test]
    fn release_type_prefers_release_types_array() {
        let a = json!({ "releaseTypes": ["Album"], "releaseType": "EP", "songCount": 1 });
        assert_eq!(infer_release_type(&a), "album");
    }

    #[test]
    fn release_type_falls_back_to_release_type_string() {
        let a = json!({ "releaseType": "Single" });
        assert_eq!(infer_release_type(&a), "single");
    }

    #[test]
    fn release_type_empty_release_types_array_falls_through_to_title() {
        let a = json!({ "releaseTypes": [], "name": "Some Track - Single" });
        assert_eq!(infer_release_type(&a), "single");
    }

    #[test]
    fn release_type_detects_single_from_title_suffix() {
        for name in ["Foo - Single", "Foo (Single)", "Foo- Single"] {
            let a = json!({ "name": name });
            assert_eq!(infer_release_type(&a), "single", "name={name}");
        }
    }

    #[test]
    fn release_type_detects_ep_from_title_suffix() {
        for name in ["Foo - EP", "Foo (EP)", "Foo- EP"] {
            let a = json!({ "name": name });
            assert_eq!(infer_release_type(&a), "ep", "name={name}");
        }
    }

    #[test]
    fn release_type_uses_title_field_when_name_absent() {
        let a = json!({ "title": "Foo - Single" });
        assert_eq!(infer_release_type(&a), "single");
    }

    #[test]
    fn release_type_song_count_buckets() {
        assert_eq!(infer_release_type(&json!({ "songCount": 0 })), "album");
        assert_eq!(infer_release_type(&json!({ "songCount": 1 })), "single");
        assert_eq!(infer_release_type(&json!({ "songCount": 2 })), "single");
        assert_eq!(infer_release_type(&json!({ "songCount": 3 })), "ep");
        assert_eq!(infer_release_type(&json!({ "songCount": 6 })), "ep");
        assert_eq!(infer_release_type(&json!({ "songCount": 7 })), "album");
    }

    #[test]
    fn release_type_missing_fields_defaults_to_album() {
        assert_eq!(infer_release_type(&json!({})), "album");
    }

    // --- format_track_info ---

    #[test]
    fn track_info_joins_all_present_parts() {
        let s = json!({
            "suffix": "flac",
            "samplingRate": 44100.0,
            "bitDepth": 16,
            "bitRate": 1234,
        });
        assert_eq!(
            format_track_info(&s),
            Some("FLAC · 44.1 kHz · 16-bit · 1234 kbps".to_string())
        );
    }

    #[test]
    fn track_info_strips_trailing_zero_decimal_on_khz() {
        let s = json!({ "samplingRate": 48000.0 });
        assert_eq!(format_track_info(&s), Some("48 kHz".to_string()));
    }

    #[test]
    fn track_info_skips_zero_and_absent_fields() {
        let s = json!({ "suffix": "", "samplingRate": 0.0, "bitDepth": 0, "bitRate": 0 });
        assert_eq!(format_track_info(&s), None);
    }

    #[test]
    fn track_info_empty_object_returns_none() {
        assert_eq!(format_track_info(&json!({})), None);
    }

    #[test]
    fn track_info_uppercases_suffix() {
        let s = json!({ "suffix": "mp3" });
        assert_eq!(format_track_info(&s), Some("MP3".to_string()));
    }

    // --- map_albums / map_artists / map_songs ---

    #[test]
    fn map_albums_applies_defaults_for_missing_fields() {
        let albums = map_albums(vec![json!({})]);
        assert_eq!(albums.len(), 1);
        let a = &albums[0];
        assert_eq!(a.id, "");
        assert_eq!(a.name, "Unknown Album");
        assert_eq!(a.album_artist, "Unknown Artist");
        assert_eq!(a.artist_id, None);
        assert_eq!(a.song_count, None);
        assert_eq!(a.release_type, "album");
        assert_eq!(a.year, None);
        assert!(!a.is_compilation);
    }

    #[test]
    fn map_albums_prefers_display_artist_over_artist() {
        let albums = map_albums(vec![json!({ "displayArtist": "DA", "artist": "A" })]);
        assert_eq!(albums[0].album_artist, "DA");
    }

    #[test]
    fn map_albums_falls_back_to_artist_when_no_display_artist() {
        let albums = map_albums(vec![json!({ "artist": "A" })]);
        assert_eq!(albums[0].album_artist, "A");
    }

    #[test]
    fn map_albums_preserves_all_real_values() {
        let raw = json!({
            "id": "a1",
            "name": "Album Name",
            "displayArtist": "Artist Name",
            "artistId": "ar1",
            "coverArt": "cov1",
            "songCount": 10,
            "releaseType": "Album",
            "year": 2024,
            "isCompilation": true,
        });
        let albums = map_albums(vec![raw]);
        let a = &albums[0];
        assert_eq!(a.id, "a1");
        assert_eq!(a.name, "Album Name");
        assert_eq!(a.album_artist, "Artist Name");
        assert_eq!(a.artist_id, Some("ar1".to_string()));
        assert_eq!(a.cover_art_id, Some("cov1".to_string()));
        assert_eq!(a.song_count, Some(10));
        assert_eq!(a.release_type, "album");
        assert_eq!(a.year, Some(2024));
        assert!(a.is_compilation);
    }

    #[test]
    fn map_artists_applies_defaults() {
        let artists = map_artists(vec![json!({})]);
        assert_eq!(artists[0].id, "");
        assert_eq!(artists[0].name, "Unknown Artist");
        assert_eq!(artists[0].album_count, 0);
    }

    #[test]
    fn map_songs_applies_defaults_for_missing_fields() {
        let songs = map_songs(vec![json!({})]);
        let s = &songs[0];
        assert_eq!(s.id, "");
        assert_eq!(s.title, "Unknown Track");
        assert_eq!(s.artist, "Unknown Artist");
        assert_eq!(s.album, "Unknown Album");
        assert_eq!(s.duration, 0.0);
        assert_eq!(s.track_info, None);
        assert_eq!(s.average_rating, None);
    }

    #[test]
    fn map_songs_average_rating_filters_non_positive() {
        let songs = map_songs(vec![json!({ "averageRating": 0.0 })]);
        assert_eq!(songs[0].average_rating, None);
        let songs = map_songs(vec![json!({ "averageRating": 3.5 })]);
        assert_eq!(songs[0].average_rating, Some(3.5));
    }

    #[test]
    fn map_songs_id_accessor_matches_field() {
        let songs = map_songs(vec![json!({ "id": "song-1" })]);
        assert_eq!(songs[0].id(), "song-1");
    }

    // --- map_similar_matches ---

    #[test]
    fn map_similar_matches_extracts_entry_and_similarity() {
        let raw = vec![json!({ "entry": { "id": "s1", "title": "T" }, "similarity": 0.87 })];
        let matches = map_similar_matches(raw);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].song.id, "s1");
        assert_eq!(matches[0].similarity, 0.87);
    }

    #[test]
    fn map_similar_matches_missing_fields_default_to_zero_and_empty_song() {
        let matches = map_similar_matches(vec![json!({})]);
        assert_eq!(matches[0].similarity, 0.0);
        assert_eq!(matches[0].song.id, "");
    }

    // --- starred ---

    #[test]
    fn map_albums_reads_starred_flag() {
        let starred = map_albums(vec![json!({ "starred": "2024-01-01T00:00:00.000Z" })]);
        assert!(starred[0].starred);
        let not_starred = map_albums(vec![json!({})]);
        assert!(!not_starred[0].starred);
    }

    #[test]
    fn map_songs_reads_starred_flag() {
        let starred = map_songs(vec![json!({ "starred": "2024-01-01T00:00:00.000Z" })]);
        assert!(starred[0].starred);
        let not_starred = map_songs(vec![json!({})]);
        assert!(!not_starred[0].starred);
    }

    #[test]
    fn map_starred_parses_artists_albums_songs() {
        let body = json!({
            "starred2": {
                "artist": [{ "id": "ar1", "name": "Artist" }],
                "album": [{ "id": "al1", "name": "Album" }],
                "song": [{ "id": "s1", "title": "Song" }],
            }
        });
        let starred = map_starred(&body);
        assert_eq!(starred.artists.len(), 1);
        assert_eq!(starred.artists[0].id, "ar1");
        assert_eq!(starred.albums.len(), 1);
        assert_eq!(starred.albums[0].id, "al1");
        assert_eq!(starred.songs.len(), 1);
        assert_eq!(starred.songs[0].id, "s1");
    }

    #[test]
    fn map_starred_missing_starred2_returns_empty() {
        let starred = map_starred(&json!({}));
        assert!(starred.artists.is_empty());
        assert!(starred.albums.is_empty());
        assert!(starred.songs.is_empty());
    }
}
