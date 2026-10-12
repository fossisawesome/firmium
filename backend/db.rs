// ============================================================================
// LOCAL PLAY HISTORY (SQLite)
// ============================================================================
// Records one row per completed play, written from the same scrobble-completion
// path as Subsonic scrobbling (`queue_manager.rs`). Local only — no server calls.
// Powers the Stats Export page. Name columns are denormalized so
// exports render fully offline without resolving IDs against the server.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::commands::mappers::Song;

/// Holds the play-history SQLite connection. Owned by the queue manager and App.
pub struct PlayHistory {
    conn: Mutex<Connection>,
}

/// Fire-and-forget play-history write, mirroring `fire_scrobble` /
/// `fire_listenbrainz_listen`. No-op when the DB failed to init (`None`);
/// errors are logged only so a DB problem never blocks playback.
pub(crate) fn fire_record_play(history: Option<&PlayHistory>, song: &Song, duration_played: i64) {
    let Some(history) = history else { return };
    if let Err(e) = history.record(song, duration_played) {
        eprintln!("Play history record failed: {e}");
    }
}

fn db_path() -> PathBuf {
    // data_dir (not cache) so history survives a cover-cache clear.
    crate::paths::data_dir().join("play_history.db")
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS plays (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  track_id TEXT NOT NULL,
  track_title TEXT NOT NULL,
  artist_id TEXT,
  artist_name TEXT,
  album_id TEXT,
  album_name TEXT,
  cover_art_id TEXT,
  genre TEXT,
  bpm INTEGER,
  timestamp INTEGER NOT NULL,
  duration_played INTEGER
);
CREATE INDEX IF NOT EXISTS idx_plays_timestamp ON plays(timestamp);
";

/// Pulls the first genre name out of the raw Subsonic `genres` JSON array
/// (`[{ \"name\": \"Rock\" }, ...]`). Mirrors the frontend `extractGenres`.
fn first_genre(genres: &Option<serde_json::Value>) -> Option<String> {
    let arr = genres.as_ref()?.as_array()?;
    for g in arr {
        if let Some(name) = g.get("name").and_then(|v| v.as_str()) {
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

impl PlayHistory {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Inserts one play row. `timestamp` is unix seconds (now); `duration_played`
    /// is seconds actually played.
    pub fn record(&self, song: &Song, duration_played: i64) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let genre = first_genre(&song.genres);
        let bpm = song.bpm.map(|b| b.round() as i64);
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO plays
             (track_id, track_title, artist_id, artist_name, album_id, album_name,
              cover_art_id, genre, bpm, timestamp, duration_played)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                song.id,
                song.title,
                song.artist_id,
                song.artist,
                song.album_id,
                song.album,
                song.cover_art_id,
                genre,
                bpm,
                now,
                duration_played,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn summary(&self) -> Result<PlayHistorySummary, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT
                COUNT(*),
                COALESCE(SUM(duration_played), 0),
                COUNT(DISTINCT track_id),
                COUNT(DISTINCT artist_id),
                COUNT(DISTINCT album_id),
                MIN(timestamp),
                MAX(timestamp)
             FROM plays",
            [],
            |r| {
                Ok(PlayHistorySummary {
                    total_plays: r.get(0)?,
                    total_seconds: r.get(1)?,
                    unique_tracks: r.get(2)?,
                    unique_artists: r.get(3)?,
                    unique_albums: r.get(4)?,
                    first_play: r.get(5)?,
                    last_play: r.get(6)?,
                })
            },
        )
        .map_err(|e| e.to_string())
    }

    /// Returns every play row (newest first) as CSV or pretty JSON for export.
    pub fn export(&self, format: &str) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT track_id, track_title, artist_id, artist_name, album_id, album_name,
                        genre, bpm, timestamp, duration_played
                 FROM plays ORDER BY timestamp DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(PlayRow {
                    track_id: r.get(0)?,
                    track_title: r.get(1)?,
                    artist_id: r.get(2)?,
                    artist_name: r.get(3)?,
                    album_id: r.get(4)?,
                    album_name: r.get(5)?,
                    genre: r.get(6)?,
                    bpm: r.get(7)?,
                    timestamp: r.get(8)?,
                    duration_played: r.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        match format {
            "json" => serde_json::to_string_pretty(&rows).map_err(|e| e.to_string()),
            _ => Ok(rows_to_csv(&rows)),
        }
    }

    pub fn recent_plays(&self, limit: usize) -> Result<Vec<RecentPlay>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT track_id, track_title, artist_id, artist_name, album_id, album_name, cover_art_id
                 FROM plays GROUP BY track_id ORDER BY MAX(timestamp) DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit as i64], |r| {
                Ok(RecentPlay {
                    track_id: r.get(0)?,
                    track_title: r.get(1)?,
                    artist_id: r.get(2)?,
                    artist_name: r.get(3)?,
                    album_id: r.get(4)?,
                    album_name: r.get(5)?,
                    cover_art_id: r.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}

fn rows_to_csv(rows: &[PlayRow]) -> String {
    let mut out = String::from(
        "track_id,track_title,artist_id,artist_name,album_id,album_name,genre,bpm,timestamp,duration_played\n",
    );
    for r in rows {
        let line = [
            csv_field(Some(&r.track_id)),
            csv_field(Some(&r.track_title)),
            csv_field(r.artist_id.as_deref()),
            csv_field(r.artist_name.as_deref()),
            csv_field(r.album_id.as_deref()),
            csv_field(r.album_name.as_deref()),
            csv_field(r.genre.as_deref()),
            r.bpm.map(|b| b.to_string()).unwrap_or_default(),
            r.timestamp.to_string(),
            r.duration_played.map(|d| d.to_string()).unwrap_or_default(),
        ]
        .join(",");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Quotes a CSV field when it contains a comma, quote, or newline (RFC 4180).
/// Text starting with `= + - @` is prefixed with `'` so spreadsheets don't run it as a formula.
fn csv_field(v: Option<&str>) -> String {
    let raw = v.unwrap_or("");
    let guarded;
    let s = if raw.starts_with(['=', '+', '-', '@']) {
        guarded = format!("'{raw}");
        guarded.as_str()
    } else {
        raw
    };
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod csv_tests {
    use super::csv_field;

    #[test]
    fn formula_prefix_is_neutralised() {
        assert_eq!(csv_field(Some("=SUM(A1)")), "'=SUM(A1)");
        assert_eq!(csv_field(Some("a,b")), "\"a,b\"");
        assert_eq!(csv_field(None), "");
    }
}

// ── Serializable result types (camelCase to match frontend) ──────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentPlay {
    pub track_id: String,
    pub track_title: String,
    pub artist_id: Option<String>,
    pub artist_name: Option<String>,
    pub album_id: Option<String>,
    pub album_name: Option<String>,
    pub cover_art_id: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayHistorySummary {
    pub total_plays: i64,
    pub total_seconds: i64,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
    pub first_play: Option<i64>,
    pub last_play: Option<i64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayRow {
    track_id: String,
    track_title: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    album_id: Option<String>,
    album_name: Option<String>,
    genre: Option<String>,
    bpm: Option<i64>,
    timestamp: i64,
    duration_played: Option<i64>,
}
