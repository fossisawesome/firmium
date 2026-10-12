//! User preferences. `Settings` is the full snapshot sent to the UI; each
//! `set` applies side effects (audio engine, queue state, keyring) and persists.

use serde::Serialize;
use serde_json::Value;

use super::Core;
use crate::commands::{credentials, queue};

const SERVICE: &str = "firmium-desktop";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme_id: String,
    pub ui_theme_id: String,
    pub font_family: String,
    pub scrollbar_width: u32,
    pub download_format: String,
    pub lrclib_enabled: bool,
    pub lyrics_word_fill: bool,
    pub window_decorations: bool,
    pub volume: f32,
    pub crossfade_enabled: bool,
    pub crossfade_duration: f32,
    pub crossfade_curve: String,
    pub gapless_enabled: bool,
    pub replay_gain_enabled: bool,
    pub auto_continue: bool,
    pub bit_perfect_mode: String,
    pub lastfm_key: String,
    pub lastfm_secret: String,
    pub listenbrainz_token: String,
}

fn secret(key: &str) -> String {
    credentials::get_password(Some(SERVICE), key).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn store_secret(key: &str, value: &str) {
    if value.is_empty() {
        let _ = credentials::delete_password(Some(SERVICE), key);
    } else {
        let _ = credentials::save_password(Some(SERVICE), key, value);
    }
}

impl Core {
    /// Pushes persisted playback settings into the queue state / audio engine.
    pub(crate) fn apply_persisted_playback(&self) {
        let s = self.settings();
        queue::init_playback_settings(
            &self.backend.queue_state,
            s.volume,
            s.crossfade_enabled,
            s.crossfade_duration,
            s.crossfade_curve,
            s.gapless_enabled,
            s.replay_gain_enabled,
            s.auto_continue,
        );
        self.backend.audio_player.set_bit_perfect_mode(s.bit_perfect_mode);
    }

    pub fn settings(&self) -> Settings {
        let c = self.config.lock();
        Settings {
            theme_id: c.theme_id.clone().unwrap_or_else(|| "firmium".into()),
            ui_theme_id: c.ui_theme_id.clone().unwrap_or_else(|| "default".into()),
            font_family: c.font_family.clone().unwrap_or_else(|| "Liberation Mono".into()),
            scrollbar_width: c.scrollbar_width.unwrap_or(10).clamp(6, 20),
            download_format: c.download_format.clone().unwrap_or_else(|| "raw".into()),
            lrclib_enabled: c.lrclib_enabled.unwrap_or(true),
            lyrics_word_fill: c.lyrics_word_fill.unwrap_or(false),
            window_decorations: c.window_decorations.unwrap_or(true),
            volume: c.volume.unwrap_or(0.8),
            crossfade_enabled: c.crossfade_enabled.unwrap_or(false),
            crossfade_duration: c.crossfade_duration.unwrap_or(5.0),
            crossfade_curve: c.crossfade_curve.clone().unwrap_or_else(|| "linear".into()),
            gapless_enabled: c.gapless_enabled.unwrap_or(true),
            replay_gain_enabled: c.replay_gain_enabled.unwrap_or(true),
            auto_continue: c.auto_continue.unwrap_or(false),
            bit_perfect_mode: c.bit_perfect_mode.clone().unwrap_or_else(|| "relaxed".into()),
            lastfm_key: secret("lastfm_key"),
            lastfm_secret: secret("lastfm_secret"),
            listenbrainz_token: secret("listenbrainz_token"),
        }
    }

    /// Applies one setting by its camelCase key. Crossfade and gapless are
    /// mutually exclusive, and strict bit-perfect cannot coexist with crossfade.
    pub fn set_setting(&self, key: &str, value: Value) -> Result<Settings, String> {
        let b = &self.backend;
        let as_bool = || value.as_bool().ok_or_else(|| format!("{key}: expected bool"));
        let as_f32 = || value.as_f64().map(|v| v as f32).ok_or_else(|| format!("{key}: expected number"));
        let as_str = || value.as_str().map(String::from).ok_or_else(|| format!("{key}: expected string"));
        match key {
            "themeId" => self.config.lock().theme_id = Some(as_str()?),
            "uiThemeId" => self.config.lock().ui_theme_id = Some(as_str()?),
            "fontFamily" => self.config.lock().font_family = Some(as_str()?),
            "scrollbarWidth" => self.config.lock().scrollbar_width = Some((as_f32()? as u32).clamp(6, 20)),
            "downloadFormat" => self.config.lock().download_format = Some(as_str()?),
            "lrclibEnabled" => self.config.lock().lrclib_enabled = Some(as_bool()?),
            "lyricsWordFill" => self.config.lock().lyrics_word_fill = Some(as_bool()?),
            "windowDecorations" => self.config.lock().window_decorations = Some(as_bool()?),
            "volume" => {
                let v = as_f32()?.clamp(0.0, 1.0);
                self.config.lock().volume = Some(v);
                queue::set_queue_volume(&b.bus, &b.queue_state, &b.audio_player, v);
            }
            "crossfadeEnabled" => {
                let on = as_bool()?;
                let dur = {
                    let mut c = self.config.lock();
                    c.crossfade_enabled = Some(on);
                    if on {
                        c.gapless_enabled = Some(false);
                        if c.bit_perfect_mode.as_deref() == Some("strict") {
                            c.bit_perfect_mode = Some("relaxed".into());
                        }
                    }
                    c.crossfade_duration.unwrap_or(5.0)
                };
                if on {
                    queue::set_gapless_enabled(&b.bus, &b.queue_state, false);
                    b.audio_player.set_bit_perfect_mode(self.settings().bit_perfect_mode);
                }
                queue::set_crossfade_settings(&b.bus, &b.queue_state, on, dur);
            }
            "crossfadeDuration" => {
                let secs = as_f32()?.clamp(1.0, 12.0);
                let on = {
                    let mut c = self.config.lock();
                    c.crossfade_duration = Some(secs);
                    c.crossfade_enabled.unwrap_or(false)
                };
                queue::set_crossfade_settings(&b.bus, &b.queue_state, on, secs);
            }
            "crossfadeCurve" => {
                let curve = as_str()?;
                self.config.lock().crossfade_curve = Some(curve.clone());
                queue::set_crossfade_curve(&b.bus, &b.queue_state, curve);
            }
            "gaplessEnabled" => {
                let on = as_bool()?;
                let dur = {
                    let mut c = self.config.lock();
                    c.gapless_enabled = Some(on);
                    if on {
                        c.crossfade_enabled = Some(false);
                    }
                    c.crossfade_duration.unwrap_or(5.0)
                };
                if on {
                    queue::set_crossfade_settings(&b.bus, &b.queue_state, false, dur);
                }
                queue::set_gapless_enabled(&b.bus, &b.queue_state, on);
            }
            "replayGainEnabled" => {
                let on = as_bool()?;
                self.config.lock().replay_gain_enabled = Some(on);
                queue::set_replay_gain_enabled(&b.queue_state, &b.audio_player, on);
            }
            "autoContinue" => {
                let on = as_bool()?;
                self.config.lock().auto_continue = Some(on);
                queue::set_auto_continue(&b.queue_state, on);
            }
            "bitPerfectMode" => {
                let mode = as_str()?;
                {
                    let mut c = self.config.lock();
                    c.bit_perfect_mode = Some(mode.clone());
                    // Strict mode reopens the output per track, so gapless can't hold.
                    if mode == "strict" {
                        c.gapless_enabled = Some(false);
                    }
                }
                if mode == "strict" {
                    queue::set_gapless_enabled(&b.bus, &b.queue_state, false);
                }
                b.audio_player.set_bit_perfect_mode(mode);
            }
            "lastfmKey" => store_secret("lastfm_key", as_str()?.trim()),
            "lastfmSecret" => store_secret("lastfm_secret", as_str()?.trim()),
            "listenbrainzToken" => store_secret("listenbrainz_token", as_str()?.trim()),
            other => return Err(format!("unknown setting: {other}")),
        }
        self.save_config();
        Ok(self.settings())
    }

    /// Resets preferences to defaults. Connection and accounts are untouched.
    pub fn reset_settings(&self) -> Settings {
        {
            let mut c = self.config.lock();
            c.download_format = None;
            c.lrclib_enabled = None;
            c.lyrics_word_fill = None;
            c.window_decorations = None;
            c.scrollbar_width = None;
            c.bit_perfect_mode = None;
            c.crossfade_enabled = None;
            c.crossfade_duration = None;
            c.crossfade_curve = None;
            c.gapless_enabled = None;
            c.replay_gain_enabled = None;
            c.auto_continue = None;
        }
        self.save_config();
        self.apply_persisted_playback();
        self.settings()
    }
}
