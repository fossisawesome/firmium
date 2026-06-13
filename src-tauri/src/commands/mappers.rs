// ============================================================================
// OPENSUBSONIC DATA MAPPERS
// ============================================================================
// These structs mirror the shapes that the JS API layer produces from raw
// Subsonic JSON. Mapping on the Rust side gives us exhaustive pattern matching
// for release-type inference and keeps transform logic out of JS.

/// Mapped album, returned to JS in camelCase via serde.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    id: String,
    name: String,
    album_artist: String,
    artist_id: Option<String>,
    cover_art_id: Option<String>,
    song_count: Option<u32>,
    release_type: String,
    genres: Option<serde_json::Value>,
    year: Option<u32>,
    is_compilation: bool,
}

/// Mapped artist, returned to JS in camelCase via serde.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    id: String,
    name: String,
    album_count: u32,
}

/// Mapped song, returned to JS in camelCase via serde.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    id: String,
    title: String,
    artist: String,
    album: String,
    album_id: Option<String>,
    duration: f64,
    track_number: Option<u32>,
    cover_art_id: Option<String>,
    replay_gain: Option<serde_json::Value>,
    bpm: Option<f64>,
    comment: Option<String>,
    genres: Option<serde_json::Value>,
    bit_rate: Option<u32>,
    sampling_rate: Option<u32>,
    bit_depth: Option<u32>,
    suffix: Option<String>,
    content_type: Option<String>,
    track_info: Option<String>,
}

/// Formats a "FLAC · 44.1 kHz · 16-bit · 1234 kbps"-style summary of a song's
/// audio format, for display in the player bar.
fn format_track_info(s: &serde_json::Value) -> Option<String> {
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
fn infer_release_type(a: &serde_json::Value) -> String {
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
    }
}

/// Map a batch of raw Subsonic album objects to typed Album structs.
#[tauri::command]
pub fn map_albums(albums: Vec<serde_json::Value>) -> Vec<Album> {
    albums.iter().map(map_album).collect()
}

/// Map a batch of raw Subsonic artist objects to typed Artist structs.
#[tauri::command]
pub fn map_artists(artists: Vec<serde_json::Value>) -> Vec<Artist> {
    artists.iter().map(map_artist).collect()
}

/// Map a batch of raw Subsonic song objects to typed Song structs.
#[tauri::command]
pub fn map_songs(songs: Vec<serde_json::Value>) -> Vec<Song> {
    songs.iter().map(map_song).collect()
}

/// A song paired with a similarity score, returned by the `sonicSimilarity`
/// OpenSubsonic extension (`getSonicSimilarTracks`/`findSonicPath`).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarMatch {
    song: Song,
    similarity: f64,
}

/// Map a batch of raw `sonicMatches` entries (`{entry, similarity}`) to typed SimilarMatch structs.
pub fn map_similar_matches(matches: Vec<serde_json::Value>) -> Vec<SimilarMatch> {
    matches
        .iter()
        .map(|m| SimilarMatch {
            song: map_song(m.get("entry").unwrap_or(&serde_json::Value::Null)),
            similarity: m.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0),
        })
        .collect()
}
