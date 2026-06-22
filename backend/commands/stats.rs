// ============================================================================
// PLAY HISTORY STATS
// ============================================================================
// Read-only views over the local play-history DB (`db.rs`) for the Stats Export
// page and Firmium Recap. All aggregation happens in SQL; no server calls.
// The save_* helpers take a path already chosen via `rfd` in the UI layer.

use crate::db::{PlayHistory, PlayHistorySummary, RecapStats};

pub fn get_recap_stats(history: &PlayHistory, from_ts: i64, to_ts: i64) -> Result<RecapStats, String> {
    history.recap(from_ts, to_ts)
}

pub fn get_play_history_summary(history: &PlayHistory) -> Result<PlayHistorySummary, String> {
    history.summary()
}

pub fn export_play_history(history: &PlayHistory, format: String) -> Result<String, String> {
    history.export(&format)
}

/// Writes UTF-8 text to a user-chosen path. Used for CSV/JSON export.
pub fn save_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| e.to_string())
}

/// Writes raw bytes to a user-chosen path. Used for recap PNG export.
pub fn save_binary_file(path: String, bytes: Vec<u8>) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}
