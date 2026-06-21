// ============================================================================
// PLAY HISTORY STATS COMMANDS
// ============================================================================
// Read-only views over the local play-history DB (`db.rs`) for the Stats Export
// page and Firmium Recap. All aggregation happens in SQL; no server calls.

use tauri::State;

use crate::db::{PlayHistory, PlayHistorySummary, RecapStats};

#[tauri::command]
pub fn get_recap_stats(history: State<'_, PlayHistory>, from_ts: i64, to_ts: i64) -> Result<RecapStats, String> {
    history.recap(from_ts, to_ts)
}

#[tauri::command]
pub fn get_play_history_summary(history: State<'_, PlayHistory>) -> Result<PlayHistorySummary, String> {
    history.summary()
}

#[tauri::command]
pub fn export_play_history(history: State<'_, PlayHistory>, format: String) -> Result<String, String> {
    history.export(&format)
}

/// Writes UTF-8 text to a user-chosen path (from the dialog plugin's save picker).
/// Used for CSV/JSON export.
#[tauri::command]
pub fn save_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| e.to_string())
}

/// Writes raw bytes to a user-chosen path. Used for recap PNG export.
#[tauri::command]
pub fn save_binary_file(path: String, bytes: Vec<u8>) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}
