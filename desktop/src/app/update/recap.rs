use iced::Task;


use super::super::export::save_export;
use super::super::message::Message;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn update_recap(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetRecapRange(r) => {
                self.recap_range = r;
                self.compute_recap()
            }
            Message::RecapNext => {
                self.recap_card = (self.recap_card + 1).min(RECAP_CARDS - 1);
                Task::none()
            }
            Message::RecapPrev => {
                self.recap_card = self.recap_card.saturating_sub(1);
                Task::none()
            }

            // ── Listening stats ─────────────────────────────────────────────────
            Message::ExportStats(format) => {
                let Some(history) = &self.backend.history else { return Task::none() };
                // Serialize synchronously (DB handle isn't Send); the async task
                // only does the file dialog + write on the owned string.
                let contents = match firmium_backend::commands::stats::export_play_history(history, format.clone()) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("export_play_history failed: {e}");
                        return Task::none();
                    }
                };
                let ext = if format == "json" { "json" } else { "csv" };
                Task::perform(
                    save_export(format!("firmium-history.{ext}"), ext.to_string(), contents),
                    Message::ExportDone,
                )
            }
            Message::ExportDone(Ok(_)) => Task::none(),
            Message::ExportDone(Err(e)) => {
                eprintln!("export save failed: {e}");
                Task::none()
            }

            // ── Genre browsing ──────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}

impl App {
    pub(crate) fn load_history_summary(&mut self) {
        if let Some(history) = &self.backend.history {
            self.history_summary = firmium_backend::commands::stats::get_play_history_summary(history).ok();
        }
    }

    pub(crate) fn compute_recap(&mut self) -> Task<Message> {
        self.recap_card = 0;
        let Some(history) = &self.backend.history else {
            self.recap = None;
            return Task::none();
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let from = self.recap_range.from_ts(now);
        match firmium_backend::commands::stats::get_recap_stats(history, from, now) {
            Ok(stats) => {
                let mut ids: Vec<String> = Vec::new();
                ids.extend(stats.top_tracks.iter().filter_map(|s| s.cover_art_id.clone()));
                ids.extend(stats.top_albums.iter().filter_map(|s| s.cover_art_id.clone()));
                if let Some(d) = &stats.biggest_discovery {
                    if let Some(c) = &d.cover_art_id {
                        ids.push(c.clone());
                    }
                }
                self.recap = Some(stats);
                self.load_cover_ids(ids)
            }
            Err(e) => {
                eprintln!("recap failed: {e}");
                self.recap = None;
                Task::none()
            }
        }
    }
}
