// ============================================================================
// LOCAL PODCAST SUBSCRIPTIONS (SQLite)
// ============================================================================
// Client-side podcast storage — Navidrome implements no server-side podcast
// endpoints (https://github.com/navidrome/navidrome/issues/793), so Firmium
// fetches/parses RSS feeds itself and stores subscriptions+episodes here.
// Local only, no cross-device sync. Mirrors `db.rs`'s `PlayHistory` pattern.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodcastChannel {
    pub id: String,
    pub feed_url: String,
    pub title: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodcastEpisode {
    pub id: String,
    pub channel_id: String,
    pub guid: String,
    pub title: String,
    pub description: Option<String>,
    pub audio_url: String,
    pub duration_seconds: Option<i64>,
    pub published_at: Option<i64>,
    pub position_ms: i64,
}

/// A freshly-parsed episode, not yet assigned a row id.
#[derive(Debug, Clone)]
pub struct NewEpisode {
    pub guid: String,
    pub title: String,
    pub description: Option<String>,
    pub audio_url: String,
    pub duration_seconds: Option<i64>,
    pub published_at: Option<i64>,
}

pub struct PodcastStore {
    conn: Mutex<Connection>,
}

fn db_path() -> PathBuf {
    crate::paths::data_dir().join("podcasts.db")
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS podcast_channels (
    id TEXT PRIMARY KEY,
    feed_url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT,
    image_url TEXT,
    added_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS podcast_episodes (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL REFERENCES podcast_channels(id),
    guid TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    audio_url TEXT NOT NULL,
    duration_seconds INTEGER,
    published_at INTEGER,
    position_ms INTEGER NOT NULL DEFAULT 0,
    UNIQUE(channel_id, guid)
);
CREATE INDEX IF NOT EXISTS idx_episodes_channel ON podcast_episodes(channel_id);
";

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl PodcastStore {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn add_channel(
        &self,
        feed_url: &str,
        title: &str,
        description: Option<&str>,
        image_url: Option<&str>,
    ) -> Result<PodcastChannel, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let added_at = now();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO podcast_channels (id, feed_url, title, description, image_url, added_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, feed_url, title, description, image_url, added_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(PodcastChannel {
            id,
            feed_url: feed_url.to_string(),
            title: title.to_string(),
            description: description.map(str::to_string),
            image_url: image_url.map(str::to_string),
            added_at,
        })
    }

    /// Inserts episodes not already present (deduped by `(channel_id, guid)`).
    /// Returns how many were actually new.
    pub fn insert_episodes(&self, channel_id: &str, episodes: &[NewEpisode]) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut inserted = 0;
        for ep in episodes {
            let id = uuid::Uuid::new_v4().to_string();
            let result = conn.execute(
                "INSERT OR IGNORE INTO podcast_episodes (id, channel_id, guid, title, description, audio_url, duration_seconds, published_at, position_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
                params![id, channel_id, ep.guid, ep.title, ep.description, ep.audio_url, ep.duration_seconds, ep.published_at],
            )
            .map_err(|e| e.to_string())?;
            inserted += result;
        }
        Ok(inserted)
    }

    pub fn list_channels(&self) -> Result<Vec<PodcastChannel>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, feed_url, title, description, image_url, added_at FROM podcast_channels ORDER BY added_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(PodcastChannel {
                    id: r.get(0)?,
                    feed_url: r.get(1)?,
                    title: r.get(2)?,
                    description: r.get(3)?,
                    image_url: r.get(4)?,
                    added_at: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn list_episodes(&self, channel_id: &str) -> Result<Vec<PodcastEpisode>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, channel_id, guid, title, description, audio_url, duration_seconds, published_at, position_ms FROM podcast_episodes WHERE channel_id = ?1 ORDER BY published_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![channel_id], |r| {
                Ok(PodcastEpisode {
                    id: r.get(0)?,
                    channel_id: r.get(1)?,
                    guid: r.get(2)?,
                    title: r.get(3)?,
                    description: r.get(4)?,
                    audio_url: r.get(5)?,
                    duration_seconds: r.get(6)?,
                    published_at: r.get(7)?,
                    position_ms: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn unsubscribe(&self, channel_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM podcast_episodes WHERE channel_id = ?1", params![channel_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM podcast_channels WHERE id = ?1", params![channel_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_position(&self, episode_id: &str, position_ms: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE podcast_episodes SET position_ms = ?1 WHERE id = ?2",
            params![position_ms, episode_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}
