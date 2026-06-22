// ============================================================================
// LOCAL PLAY HISTORY (SQLite)
// ============================================================================
// Records one row per completed play, written from the same scrobble-completion
// path as Subsonic scrobbling (`queue_manager.rs`). Local only — no server calls.
// Powers the Stats Export page and Firmium Recap. Name columns are denormalized so
// Recap renders fully offline without resolving IDs against the server.

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

    pub fn recap(&self, from: i64, to: i64) -> Result<RecapStats, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (total_plays, total_seconds): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(duration_played), 0)
                 FROM plays WHERE timestamp >= ?1 AND timestamp < ?2",
                [from, to],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let top_tracks = query_top_tracks(&conn, from, to, 10)?;
        let top_artists = query_top_artists(&conn, from, to, 10)?;
        let top_albums = query_top_albums(&conn, from, to, 10)?;
        let top_genre = query_top_genre(&conn, from, to)?;
        let by_time_of_day = query_by_time_of_day(&conn, from, to)?;
        let by_day_of_week = query_by_day_of_week(&conn, from, to)?;
        let biggest_discovery = query_biggest_discovery(&conn, from, to)?;
        let streak = query_streak(&conn, from, to)?;

        Ok(RecapStats {
            from,
            to,
            total_plays,
            total_seconds,
            top_tracks,
            top_artists,
            top_albums,
            top_genre,
            by_time_of_day,
            by_day_of_week,
            biggest_discovery,
            streak,
        })
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
}

fn query_top_tracks(conn: &Connection, from: i64, to: i64, limit: i64) -> Result<Vec<TrackStat>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT track_id, track_title, artist_name, cover_art_id, COUNT(*) AS c
             FROM plays WHERE timestamp >= ?1 AND timestamp < ?2
             GROUP BY track_id ORDER BY c DESC, track_title ASC LIMIT ?3",
        )
        .map_err(|e| e.to_string())?;
    let out = stmt
        .query_map([from, to, limit], |r| {
            Ok(TrackStat {
                track_id: r.get(0)?,
                title: r.get(1)?,
                artist: r.get(2)?,
                cover_art_id: r.get(3)?,
                count: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(out)
}

fn query_top_artists(conn: &Connection, from: i64, to: i64, limit: i64) -> Result<Vec<ArtistStat>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT artist_id, artist_name, COUNT(*) AS c
             FROM plays WHERE timestamp >= ?1 AND timestamp < ?2 AND artist_name IS NOT NULL
             GROUP BY artist_name ORDER BY c DESC, artist_name ASC LIMIT ?3",
        )
        .map_err(|e| e.to_string())?;
    let out = stmt
        .query_map([from, to, limit], |r| {
            Ok(ArtistStat {
                artist_id: r.get(0)?,
                name: r.get(1)?,
                count: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(out)
}

fn query_top_albums(conn: &Connection, from: i64, to: i64, limit: i64) -> Result<Vec<AlbumStat>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT album_id, album_name, artist_name, cover_art_id, COUNT(*) AS c
             FROM plays WHERE timestamp >= ?1 AND timestamp < ?2 AND album_name IS NOT NULL
             GROUP BY album_name ORDER BY c DESC, album_name ASC LIMIT ?3",
        )
        .map_err(|e| e.to_string())?;
    let out = stmt
        .query_map([from, to, limit], |r| {
            Ok(AlbumStat {
                album_id: r.get(0)?,
                name: r.get(1)?,
                artist: r.get(2)?,
                cover_art_id: r.get(3)?,
                count: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(out)
}

fn query_top_genre(conn: &Connection, from: i64, to: i64) -> Result<Option<GenreStat>, String> {
    conn.query_row(
        "SELECT genre, COUNT(*) AS c
         FROM plays WHERE timestamp >= ?1 AND timestamp < ?2 AND genre IS NOT NULL AND genre != ''
         GROUP BY genre ORDER BY c DESC LIMIT 1",
        [from, to],
        |r| Ok(GenreStat { genre: r.get(0)?, count: r.get(1)? }),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

fn query_by_time_of_day(conn: &Connection, from: i64, to: i64) -> Result<TimeOfDay, String> {
    // Local-time hour buckets: morning 5-11, afternoon 12-16, evening 17-20, night 21-4.
    let mut stmt = conn
        .prepare(
            "SELECT CAST(strftime('%H', timestamp, 'unixepoch', 'localtime') AS INTEGER) AS h, COUNT(*)
             FROM plays WHERE timestamp >= ?1 AND timestamp < ?2 GROUP BY h",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([from, to], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut t = TimeOfDay::default();
    for row in rows {
        let (h, c) = row.map_err(|e| e.to_string())?;
        match h {
            5..=11 => t.morning += c,
            12..=16 => t.afternoon += c,
            17..=20 => t.evening += c,
            _ => t.night += c,
        }
    }
    Ok(t)
}

fn query_by_day_of_week(conn: &Connection, from: i64, to: i64) -> Result<[i64; 7], String> {
    // strftime '%w': 0 = Sunday .. 6 = Saturday.
    let mut stmt = conn
        .prepare(
            "SELECT CAST(strftime('%w', timestamp, 'unixepoch', 'localtime') AS INTEGER) AS d, COUNT(*)
             FROM plays WHERE timestamp >= ?1 AND timestamp < ?2 GROUP BY d",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([from, to], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut out = [0i64; 7];
    for row in rows {
        let (d, c) = row.map_err(|e| e.to_string())?;
        if (0..7).contains(&d) {
            out[d as usize] = c;
        }
    }
    Ok(out)
}

fn query_biggest_discovery(conn: &Connection, from: i64, to: i64) -> Result<Option<DiscoveryStat>, String> {
    // A track first heard within the window that racked up the most plays in it.
    conn.query_row(
        "SELECT track_id, track_title, artist_name, cover_art_id, COUNT(*) AS c, MIN(timestamp)
         FROM plays WHERE timestamp >= ?1 AND timestamp < ?2
         GROUP BY track_id HAVING c > 1 ORDER BY c DESC, MIN(timestamp) DESC LIMIT 1",
        [from, to],
        |r| {
            Ok(DiscoveryStat {
                track_id: r.get(0)?,
                title: r.get(1)?,
                artist: r.get(2)?,
                cover_art_id: r.get(3)?,
                count: r.get(4)?,
                first_heard: r.get(5)?,
            })
        },
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

fn query_streak(conn: &Connection, from: i64, to: i64) -> Result<Streak, String> {
    // Distinct local days with at least one play, ascending.
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT date(timestamp, 'unixepoch', 'localtime') AS d
             FROM plays WHERE timestamp >= ?1 AND timestamp < ?2 ORDER BY d ASC",
        )
        .map_err(|e| e.to_string())?;
    let days = stmt
        .query_map([from, to], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let days_active = days.len() as i64;
    let mut longest = 0i64;
    let mut current = 0i64;
    let mut prev: Option<i64> = None;
    for d in &days {
        let day_num = day_to_epoch_days(d);
        match prev {
            Some(p) if day_num == p + 1 => current += 1,
            _ => current = 1,
        }
        if current > longest {
            longest = current;
        }
        prev = Some(day_num);
    }
    Ok(Streak { days_active, longest_streak: longest })
}

/// Converts an `YYYY-MM-DD` string to a day count since the epoch for adjacency
/// comparison. Returns 0 on a malformed string (won't extend any streak).
fn day_to_epoch_days(d: &str) -> i64 {
    let parts: Vec<i64> = d.split('-').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 3 {
        return 0;
    }
    let (y, m, day) = (parts[0], parts[1], parts[2]);
    // Days from civil date (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
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
fn csv_field(v: Option<&str>) -> String {
    let s = v.unwrap_or("");
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ── Serializable result types (camelCase to match frontend) ──────────────────

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
pub struct RecapStats {
    pub from: i64,
    pub to: i64,
    pub total_plays: i64,
    pub total_seconds: i64,
    pub top_tracks: Vec<TrackStat>,
    pub top_artists: Vec<ArtistStat>,
    pub top_albums: Vec<AlbumStat>,
    pub top_genre: Option<GenreStat>,
    pub by_time_of_day: TimeOfDay,
    pub by_day_of_week: [i64; 7],
    pub biggest_discovery: Option<DiscoveryStat>,
    pub streak: Streak,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackStat {
    pub track_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub cover_art_id: Option<String>,
    pub count: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistStat {
    pub artist_id: Option<String>,
    pub name: String,
    pub count: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumStat {
    pub album_id: Option<String>,
    pub name: String,
    pub artist: Option<String>,
    pub cover_art_id: Option<String>,
    pub count: i64,
}

#[derive(serde::Serialize)]
pub struct GenreStat {
    pub genre: String,
    pub count: i64,
}

#[derive(serde::Serialize, Default)]
pub struct TimeOfDay {
    pub morning: i64,
    pub afternoon: i64,
    pub evening: i64,
    pub night: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryStat {
    pub track_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub cover_art_id: Option<String>,
    pub count: i64,
    pub first_heard: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Streak {
    pub days_active: i64,
    pub longest_streak: i64,
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
