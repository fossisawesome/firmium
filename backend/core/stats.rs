//! Listening stats: Recap windows, history summary, recent plays, export.

use super::Core;
use crate::db::{PlayHistorySummary, RecapStats, RecentPlay};

/// Inclusive lower bound (unix seconds) for a named Recap window.
fn range_from(range: &str, now: i64) -> Option<i64> {
    let day = 86_400;
    Some(match range {
        "week" => now - 7 * day,
        "month" => now - 30 * day,
        "threeMonths" => now - 90 * day,
        "year" => now - 365 * day,
        "all" => 0,
        _ => return None,
    })
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Core {
    fn history(&self) -> Result<&crate::db::PlayHistory, String> {
        self.backend.history.as_deref().ok_or_else(|| "Play history unavailable".to_string())
    }

    /// `range`: "week" | "month" | "threeMonths" | "year" | "all" | "custom" (uses `custom`).
    pub fn recap(&self, range: &str, custom: Option<(i64, i64)>) -> Result<RecapStats, String> {
        let now = now_secs();
        let (from, to) = match (range, custom) {
            ("custom", Some((f, t))) => (f, t),
            _ => (range_from(range, now).ok_or_else(|| format!("unknown recap range: {range}"))?, now),
        };
        self.history()?.recap(from, to)
    }

    /// True once per week when there is listening history to recap; records the time shown.
    pub fn weekly_recap_due(&self) -> bool {
        let now = now_secs();
        let due = self.config.lock().last_weekly_recap.is_none_or(|t| now - t >= 7 * 86_400);
        if !due || self.history_summary().map_or(true, |s| s.total_plays == 0) {
            return false;
        }
        self.config.lock().last_weekly_recap = Some(now);
        self.save_config();
        true
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

#[cfg(test)]
mod tests {
    use super::range_from;

    #[test]
    fn named_ranges() {
        assert_eq!(range_from("week", 1_000_000), Some(1_000_000 - 7 * 86_400));
        assert_eq!(range_from("all", 5), Some(0));
        assert_eq!(range_from("bogus", 5), None);
    }
}
