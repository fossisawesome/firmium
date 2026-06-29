use iced::Task;


use super::super::message::Message;
use super::super::App;

impl App {
    pub(crate) fn update_queue_resume(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PlayQueueFetched(Ok(Some(q))) => {
                // Only prompt for a queue that isn't already playing locally.
                if !q.entries.is_empty() && self.queue.is_empty() {
                    self.resume_queue = Some(q);
                }
                Task::none()
            }
            Message::PlayQueueFetched(Ok(None)) => Task::none(),
            Message::PlayQueueFetched(Err(e)) => {
                eprintln!("get_play_queue failed: {e:?}");
                Task::none()
            }
            Message::ResumeQueue => {
                let Some(q) = self.resume_queue.take() else { return Task::none() };
                let start_idx = q
                    .current
                    .as_deref()
                    .and_then(|cur| q.entries.iter().position(|s| s.id == cur))
                    .unwrap_or(0);
                let pos = q.position_ms.unwrap_or(0).max(0) as f64 / 1000.0;
                Task::perform(
                    crate::commands::queue::set_queue(
                        self.backend.queue_state.clone(),
                        self.backend.app_state.clone(),
                        self.backend.audio_player.clone(),
                        q.entries,
                        start_idx,
                    ),
                    move |res| match res {
                        // Seek to the saved offset once the track is loaded.
                        Ok(()) => Message::SeekTo(pos as f32),
                        Err(e) => Message::PlaybackDone(Err(e)),
                    },
                )
            }
            Message::DismissResume => {
                self.resume_queue = None;
                Task::none()
            }

            // ── Account switcher ────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}
