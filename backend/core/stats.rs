//! Listening stats: history summary, recent plays, export.

use super::Core;
use crate::db::{PlayHistorySummary, RecentPlay};

impl Core {
    fn history(&self) -> Result<&crate::db::PlayHistory, String> {
        self.backend.history.as_deref().ok_or_else(|| "Play history unavailable".to_string())
    }

    pub fn history_summary(&self) -> Result<PlayHistorySummary, String> {
        self.history()?.summary()
    }

    pub fn recent_plays(&self, limit: usize) -> Result<Vec<RecentPlay>, String> {
        self.history()?.recent_plays(limit)
    }

    /// Serialized play history ("csv" or "json").
    pub fn export_history(&self, format: &str) -> Result<String, String> {
        self.history()?.export(format)
    }
}
