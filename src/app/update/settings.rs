use iced::Task;

use crate::theme::Tokens;

use super::super::message::Message;
use super::super::App;

impl App {
    pub(crate) fn update_settings(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectTheme(id) => {
                if let Some(entry) = self.themes.iter().find(|t| t.id == id) {
                    self.tokens = Tokens::from_entry(entry);
                    self.theme_id = id;
                    self.save_config();
                }
                Task::none()
            }
            Message::SelectFont(name) => {
                self.font_family = name;
                self.save_config();
                Task::none()
            }
            Message::SetCrossfadeEnabled(on) => {
                self.crossfade_enabled = on;
                if on {
                    if self.gapless_enabled {
                        self.gapless_enabled = false;
                        crate::commands::queue::set_gapless_enabled(&self.backend.bus, &self.backend.queue_state, false);
                    }
                    if self.bit_perfect_mode == "strict" {
                        self.bit_perfect_mode = "relaxed".to_string();
                        self.backend.audio_player.set_bit_perfect_mode("relaxed".to_string());
                    }
                }
                crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, on, self.crossfade_duration);
                Task::none()
            }
            Message::SetCrossfadeDuration(secs) => {
                self.crossfade_duration = secs;
                crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, self.crossfade_enabled, secs);
                Task::none()
            }
            Message::SetGapless(on) => {
                self.gapless_enabled = on;
                if on && self.crossfade_enabled {
                    self.crossfade_enabled = false;
                    crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, false, self.crossfade_duration);
                }
                crate::commands::queue::set_gapless_enabled(&self.backend.bus, &self.backend.queue_state, on);
                Task::none()
            }
            Message::SetReplayGain(on) => {
                self.replay_gain_enabled = on;
                crate::commands::queue::set_replay_gain_enabled(&self.backend.queue_state, &self.backend.audio_player, on);
                Task::none()
            }
            Message::SetAutoContinue(on) => {
                self.auto_continue = on;
                crate::commands::queue::set_auto_continue(&self.backend.queue_state, on);
                Task::none()
            }
            Message::SetBitPerfect(mode) => {
                self.bit_perfect_mode = mode.clone();
                self.backend.audio_player.set_bit_perfect_mode(mode);
                Task::none()
            }

            // ── Settings UI ─────────────────────────────────────────────────────
            Message::SetSettingsCategory(cat) => {
                self.settings_category = cat;
                Task::none()
            }
            Message::SetDownloadFormat(fmt) => {
                self.download_format = fmt;
                self.save_config();
                Task::none()
            }
            Message::SetLastfmEnabled(on) => {
                self.lastfm_enabled = on;
                if !on {
                    let _ = crate::commands::credentials::delete_password(Some("firmium-desktop"), "lastfm_key");
                    let _ = crate::commands::credentials::delete_password(Some("firmium-desktop"), "lastfm_secret");
                    self.lastfm_key.clear();
                    self.lastfm_secret.clear();
                }
                Task::none()
            }
            Message::SetLastfmKey(key) => {
                self.lastfm_key = key.clone();
                let _ = crate::commands::credentials::save_password(Some("firmium-desktop"), "lastfm_key", &key);
                Task::none()
            }
            Message::SetLastfmSecret(secret) => {
                self.lastfm_secret = secret.clone();
                let _ = crate::commands::credentials::save_password(Some("firmium-desktop"), "lastfm_secret", &secret);
                Task::none()
            }
            Message::SetListenbrainzEnabled(on) => {
                self.listenbrainz_enabled = on;
                if !on {
                    let _ = crate::commands::credentials::delete_password(Some("firmium-desktop"), "listenbrainz_token");
                    self.listenbrainz_token.clear();
                }
                Task::none()
            }
            Message::SetListenbrainzToken(token) => {
                self.listenbrainz_token = token.clone();
                let _ = crate::commands::credentials::save_password(Some("firmium-desktop"), "listenbrainz_token", &token);
                Task::none()
            }
            Message::SetLrclibEnabled(on) => {
                self.lrclib_enabled = on;
                self.save_config();
                Task::none()
            }
            Message::SetLyricsWordFill(on) => {
                self.lyrics_word_fill = on;
                self.save_config();
                Task::none()
            }
            Message::SetDecorations(on) => {
                self.window_decorations = on;
                self.save_config();
                // winit only offers a toggle (no absolute set); boot applies the
                // persisted value, so a single toggle here keeps UI and window in sync.
                iced::window::latest().then(|maybe_id| match maybe_id {
                    Some(id) => iced::window::toggle_decorations(id),
                    None => Task::none(),
                })
            }
            Message::SetScrollbarWidth(width) => {
                let clamped = width.max(6).min(20);
                self.scrollbar_width = clamped;
                self.save_config();
                Task::none()
            }
            Message::WipeCoverCache => {
                let _ = crate::commands::cover_cache::clear_cover_cache();
                self.cover_cache.clear();
                self.cover_cache_order.clear();
                Task::none()
            }
            Message::DeleteSettings => {
                // Reset preference fields to defaults (connection/account untouched).
                self.download_format = "raw".to_string();
                self.lrclib_enabled = true;
                self.lyrics_word_fill = false;
                self.window_decorations = true;
                self.viz_cover_colors = true;
                self.scrollbar_width = 10;
                self.bit_perfect_mode = "relaxed".to_string();
                self.crossfade_enabled = false;
                self.crossfade_duration = 5.0;
                self.gapless_enabled = true;
                self.replay_gain_enabled = true;
                self.auto_continue = false;
                self.backend.audio_player.set_bit_perfect_mode(self.bit_perfect_mode.clone());
                crate::commands::queue::set_gapless_enabled(&self.backend.bus, &self.backend.queue_state, self.gapless_enabled);
                crate::commands::queue::set_replay_gain_enabled(&self.backend.queue_state, &self.backend.audio_player, self.replay_gain_enabled);
                crate::commands::queue::set_auto_continue(&self.backend.queue_state, self.auto_continue);
                crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, self.crossfade_enabled, self.crossfade_duration);
                self.save_config();
                Task::none()
            }
            _ => unreachable!(),
        }
    }
}
