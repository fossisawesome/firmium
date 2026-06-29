use iced::Task;


use super::super::message::Message;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn update_podcasts(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PodcastChannelsLoaded(Ok(channels)) => {
                self.podcast_channels = channels;
                Task::none()
            }
            Message::PodcastChannelsLoaded(Err(e)) => {
                eprintln!("Failed to load podcast channels: {e}");
                Task::none()
            }
            Message::OpenAddPodcastModal => {
                self.podcast_add_modal_open = true;
                self.podcast_add_error = None;
                Task::none()
            }
            Message::CloseAddPodcastModal => {
                self.podcast_add_modal_open = false;
                self.podcast_add_url_input.clear();
                Task::none()
            }
            Message::PodcastAddUrlChanged(url) => {
                self.podcast_add_url_input = url;
                Task::none()
            }
            Message::SubmitAddPodcastChannel => {
                let url = self.podcast_add_url_input.clone();
                if url.trim().is_empty() {
                    return Task::none();
                }
                if let Some(store) = self.backend.podcasts.clone() {
                    let state = self.backend.app_state.clone();
                    Task::perform(crate::podcasts::add_channel(state, store, url), Message::PodcastChannelAdded)
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelAdded(Ok(channel)) => {
                self.podcast_channels.push(channel);
                self.podcast_add_modal_open = false;
                self.podcast_add_url_input.clear();
                self.podcast_add_error = None;
                Task::none()
            }
            Message::PodcastChannelAdded(Err(e)) => {
                self.podcast_add_error = Some(e);
                Task::none()
            }
            Message::PodcastEpisodesLoaded(Ok(episodes)) => {
                self.podcast_episodes = episodes;
                Task::none()
            }
            Message::PodcastEpisodesLoaded(Err(e)) => {
                eprintln!("Failed to load podcast episodes: {e}");
                Task::none()
            }
            Message::RefreshPodcastChannel(channel_id, feed_url) => {
                if let Some(store) = self.backend.podcasts.clone() {
                    let state = self.backend.app_state.clone();
                    Task::perform(
                        crate::podcasts::refresh_channel(state, store, channel_id, feed_url),
                        Message::PodcastChannelRefreshed,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelRefreshed(Ok(_new_count)) => {
                if let View::PodcastDetail(channel_id) = self.view.clone() {
                    if let Some(store) = self.backend.podcasts.clone() {
                        return Task::perform(
                            async move { crate::podcasts::list_episodes(store, channel_id) },
                            Message::PodcastEpisodesLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::PodcastChannelRefreshed(Err(e)) => {
                eprintln!("Failed to refresh podcast channel: {e}");
                Task::none()
            }
            Message::UnsubscribePodcastChannel(channel_id) => {
                if let Some(store) = self.backend.podcasts.clone() {
                    Task::perform(
                        async move { crate::podcasts::unsubscribe(store, channel_id) },
                        Message::PodcastChannelUnsubscribed,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelUnsubscribed(Ok(())) => {
                if let Some(store) = self.backend.podcasts.clone() {
                    Task::perform(
                        async move { crate::podcasts::list_channels(store) },
                        Message::PodcastChannelsLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelUnsubscribed(Err(e)) => {
                eprintln!("Failed to unsubscribe podcast channel: {e}");
                Task::none()
            }
            Message::PlayPodcastEpisode(episode) => {
                let resume_secs = episode.position_ms as f64 / 1000.0;
                match crate::audio::AudioPlayer::play_stream(
                    &self.backend.audio_player,
                    &episode.audio_url,
                    episode.id.clone(),
                    None,
                ) {
                    Ok(player_id) => {
                        self.current_player_id = Some(player_id.clone());
                        self.current_podcast_episode = Some(episode);
                        if resume_secs > 0.0 {
                            let _ = self.backend.audio_player.seek(&player_id, resume_secs);
                        }
                    }
                    Err(e) => eprintln!("Failed to play podcast episode: {e}"),
                }
                Task::none()
            }
            _ => unreachable!(),
        }
    }
}
