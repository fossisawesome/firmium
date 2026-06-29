use iced::Task;


use super::super::message::Message;
use super::super::App;

impl App {
    pub(crate) fn update_search(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchInput(q) => {
                self.search_query = q;
                Task::none()
            }
            Message::SubmitSearch => {
                let q = self.search_query.trim().to_string();
                if q.is_empty() {
                    return Task::none();
                }
                Task::perform(
                    crate::commands::subsonic::search(self.backend.app_state.clone(), q),
                    Message::SearchLoaded,
                )
            }
            Message::SearchLoaded(Ok(res)) => {
                let ids: Vec<String> = res
                    .albums
                    .iter()
                    .filter_map(|a| a.cover_art_id.clone())
                    .chain(res.songs.iter().filter_map(|s| s.cover_art_id.clone()))
                    .collect();
                self.search_results = Some(res);
                self.load_cover_ids(ids)
            }
            Message::SearchLoaded(Err(e)) => {
                eprintln!("search failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::SetSearchRatingFilter(n) => {
                self.search_rating_filter = if self.search_rating_filter == n { 0 } else { n };
                Task::none()
            }

            // ── Settings ────────────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}
