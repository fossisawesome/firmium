use iced::Task;


use super::super::format::filter_energy;
use super::super::message::Message;
use super::super::App;

impl App {
    pub(crate) fn update_mix(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GenerateMix(energy) => Task::perform(
                firmium_backend::commands::subsonic::get_random_songs(self.backend.app_state.clone(), Some(200), None),
                move |res| Message::MixFetched(energy, res),
            ),
            Message::MixFetched(energy, Ok(songs)) => {
                let mix = filter_energy(songs, energy);
                if mix.is_empty() {
                    return Task::none();
                }
                Task::perform(
                    firmium_backend::commands::queue::set_queue(
                        self.backend.queue_state.clone(),
                        self.backend.app_state.clone(),
                        self.backend.audio_player.clone(),
                        mix,
                        0,
                    ),
                    Message::PlaybackDone,
                )
            }
            Message::MixFetched(_, Err(e)) => {
                eprintln!("mix fetch failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }

            // ── Transport ─────────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}
