use iced::Task;

use firmium_backend::events::BackendEvent;
use crate::PlaybackState;

use super::super::message::Message;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn update_transport(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePlay => Task::perform(
                firmium_backend::commands::queue::toggle_play(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                ),
                Message::PlaybackDone,
            ),
            Message::Next => Task::perform(
                firmium_backend::commands::queue::queue_next(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                ),
                Message::PlaybackDone,
            ),
            Message::Prev => Task::perform(
                firmium_backend::commands::queue::queue_prev(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                ),
                Message::PlaybackDone,
            ),
            Message::ToggleShuffle => {
                firmium_backend::commands::queue::toggle_shuffle(&self.backend.bus, &self.backend.queue_state);
                Task::none()
            }
            Message::CycleRepeat => {
                let (one, all) = if !self.repeat_one && !self.repeat_all {
                    (false, true)
                } else if self.repeat_all {
                    (true, false)
                } else {
                    (false, false)
                };
                firmium_backend::commands::queue::set_repeat_mode(&self.backend.bus, &self.backend.queue_state, one, all);
                Task::none()
            }
            Message::SetVolume(v) => {
                self.volume = v;
                firmium_backend::commands::queue::set_queue_volume(
                    &self.backend.bus,
                    &self.backend.queue_state,
                    &self.backend.audio_player,
                    v,
                );
                Task::none()
            }
            Message::SeekTo(secs) => {
                self.position = secs as f64;
                let _ = firmium_backend::commands::queue::seek_queue(&self.backend.queue_state, &self.backend.audio_player, secs as f64);
                Task::none()
            }
            Message::TogglePanel(p) => {
                self.right_panel = if self.right_panel == Some(p) { None } else { Some(p) };
                self.backend
                    .audio_player
                    .set_visualizer_enabled(self.right_panel == Some(Panel::Visualizer));
                if self.right_panel == Some(Panel::Equalizer) {
                    self.eq_state = Some(firmium_backend::commands::equalizer::get_eq_state());
                }
                Task::batch([self.maybe_fetch_lyrics(), self.maybe_fetch_similar(), self.maybe_fetch_viz_colors()])
            }
            Message::SetVizMode(m) => {
                self.visualizer_mode = m;
                Task::none()
            }
            Message::SetVizCoverColors(on) => {
                self.viz_cover_colors = on;
                self.save_config();
                self.maybe_fetch_viz_colors()
            }
            Message::VizColorsLoaded(track_id, res) => {
                if self.viz_palette_track.as_deref() == Some(track_id.as_str()) {
                    if let Ok(colors) = res {
                        self.viz_palette = Some(colors.orb);
                    }
                }
                Task::none()
            }
            Message::LyricsLoaded(track_id, res) => {
                if self.lyrics_track_id.as_deref() == Some(track_id.as_str()) {
                    self.lyrics = res.ok().flatten();
                }
                Task::none()
            }
            Message::SimilarLoaded(track_id, res) => {
                if self.similar_track_id.as_deref() == Some(track_id.as_str()) {
                    self.similar_results = res.unwrap_or_default();
                    let ids: Vec<String> = self
                        .similar_results
                        .iter()
                        .filter_map(|m| m.song.cover_art_id.clone())
                        .collect();
                    return self.load_cover_ids(ids);
                }
                Task::none()
            }
            Message::PlayQueueIndex(idx) => Task::perform(
                firmium_backend::commands::queue::play_queue_index(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                    idx,
                ),
                Message::PlaybackDone,
            ),
            Message::PlaybackDone(Err(e)) => {
                eprintln!("transport command failed: {e}");
                Task::none()
            }
            Message::PlaybackDone(Ok(())) => Task::none(),

            // ── Resume-queue prompt ─────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}

impl App {
    pub(crate) fn maybe_fetch_lyrics(&mut self) -> Task<Message> {
        if self.right_panel != Some(Panel::Lyrics) {
            return Task::none();
        }
        let song = if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).cloned()
        } else {
            None
        };
        let Some(song) = song else {
            self.lyrics = None;
            self.lyrics_track_id = None;
            return Task::none();
        };
        if self.lyrics_track_id.as_deref() == Some(song.id.as_str()) {
            return Task::none();
        }
        self.lyrics = None;
        self.lyrics_track_id = Some(song.id.clone());
        let id = song.id.clone();
        Task::perform(
            firmium_backend::commands::subsonic::get_song_lyrics(
                self.backend.app_state.clone(),
                song.id,
                song.artist,
                song.title,
                song.duration,
                self.lrclib_enabled,
            ),
            move |res| Message::LyricsLoaded(id.clone(), res),
        )
    }

    pub(crate) fn maybe_fetch_similar(&mut self) -> Task<Message> {
        if self.right_panel != Some(Panel::Similar) {
            return Task::none();
        }
        let song = if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).cloned()
        } else {
            None
        };
        let Some(song) = song else {
            self.similar_results.clear();
            self.similar_track_id = None;
            return Task::none();
        };
        if self.similar_track_id.as_deref() == Some(song.id.as_str()) {
            return Task::none();
        }
        self.similar_results.clear();
        self.similar_track_id = Some(song.id.clone());
        let genre = song
            .genres
            .as_ref()
            .and_then(|g| g.as_array())
            .and_then(|a| a.first())
            .and_then(|g| g.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let id = song.id.clone();
        Task::perform(
            firmium_backend::commands::subsonic::get_similar_tracks_fallback(
                self.backend.app_state.clone(),
                song.id,
                song.artist_id,
                genre,
                Some(20),
            ),
            move |res| Message::SimilarLoaded(id.clone(), res),
        )
    }

    pub(crate) fn handle_backend(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::PlaybackStateChanged { player_id, state, .. } => {
                if matches!(state, PlaybackState::Loading | PlaybackState::Playing) {
                    self.current_player_id = Some(player_id.clone());
                }
                if self.current_player_id.as_deref() == Some(player_id.as_str()) {
                    self.playback_state = state;
                }
            }
            BackendEvent::PlaybackPosition { player_id, position, duration } => {
                if self.current_player_id.as_deref() == Some(player_id.as_str()) {
                    self.position = position;
                    self.duration = duration;
                    if let Some(episode) = &self.current_podcast_episode {
                        if let Some(store) = &self.backend.podcasts {
                            let position_ms = (position * 1000.0) as i64;
                            if let Err(e) = store.update_position(&episode.id, position_ms) {
                                eprintln!("Failed to save podcast position: {e}");
                            }
                        }
                    }
                }
            }
            BackendEvent::PlaybackFinished { .. } => {
                self.position = 0.0;
            }
            BackendEvent::QueueStateChanged(snapshot) => {
                self.queue = snapshot.queue;
                self.queue_idx = snapshot.queue_idx;
                self.repeat_one = snapshot.repeat_one;
                self.repeat_all = snapshot.repeat_all;
                self.shuffle = snapshot.shuffle_enabled;
                self.volume = snapshot.volume;
                self.crossfade_enabled = snapshot.crossfade_enabled;
                self.crossfade_duration = snapshot.crossfade_duration;
                self.gapless_enabled = snapshot.gapless_enabled;
                self.replay_gain_enabled = snapshot.replay_gain_enabled;
                self.current_player_id = snapshot.player_id;
                self.current_podcast_episode = None;
            }
            BackendEvent::QueueExhausted(_song) => {}
            BackendEvent::SessionExpired => {
                self.authed = false;
                self.show_account_switcher = true;
            }
        }
    }
}
