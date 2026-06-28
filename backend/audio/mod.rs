//! Desktop-only audio playback engine: `symphonia` for decoding, `cpal` for
//! device output. Each playback session decodes into a ring buffer on a
//! blocking task; a single shared cpal output stream mixes all active
//! sessions' ring buffers in realtime.
//!
//! Output is opened at the primary session's native sample rate whenever
//! possible (no resampling — "bit-perfect"). When that's not possible
//! (multiple sessions during a crossfade, or a device that can't match the
//! native rate), `output::mix_into`'s linear-interpolation resampler degrades
//! gracefully instead of failing playback.

mod decoder;
pub mod eq;
mod output;
mod session;
mod streaming_reader;

use std::collections::HashMap;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};
use uuid::Uuid;

use crate::events::{AudioInfo, BackendEvent, EventBus};
use crate::visualizer::VisualizerState;
use crate::{AudioDevice, PlaybackState};

use eq::{EqBand, EqShared};
use decoder::DecoderHandle;
use output::OutputStream;
use session::{PlayerId, ResampleState, SeekReply, SeekRequest, Session, SessionMap};
use streaming_reader::{FileSource, StreamingReader, VecSource};

const PLAYER_NOT_FOUND: &str = "Player not found";

/// Decoder handle, optional duration, consumed-bytes buffer (for seek-rebuild
/// fallback), native sample rate, and channel count.
type OpenedStream = (DecoderHandle, Option<f64>, Arc<Mutex<Vec<u8>>>, u32, u16);

struct OutputHandle {
    device: cpal::Device,
    stream: OutputStream,
}

/// Main audio player manager (Send + Sync safe).
pub struct AudioPlayer {
    sessions: SessionMap,
    output: RwLock<OutputHandle>,
    /// Reused HTTP client — avoids rebuilding a connection pool and TLS context on every track.
    http_client: reqwest::blocking::Client,
    /// Event bus for broadcasting playback state/position changes to the UI.
    bus: EventBus,
    /// True while a crossfade's volume ramp is in flight. Reopening the
    /// stream mid-crossfade would silence whichever session is on the old
    /// stream, so reopens are deferred until the fade completes.
    crossfade_in_progress: AtomicBool,
    /// Shared state for the audio visualizer (sample ring buffer + analysis toggle).
    pub(crate) visualizer: Arc<VisualizerState>,
    /// "off" | "relaxed" | "strict" — controls whether the output stream is reopened
    /// to match each track's native sample rate. "off" skips reopening entirely.
    bit_perfect_mode: parking_lot::Mutex<String>,
    /// Live-updatable equalizer band config shared with every decode feeder.
    eq: Arc<EqShared>,
}

impl AudioPlayer {
    /// Initialize the audio player with the default audio device.
    pub fn new(bus: EventBus) -> Result<Self, String> {
        let sessions: SessionMap = Arc::new(RwLock::new(HashMap::new()));
        let (device, stream) = output::open_default(Arc::clone(&sessions))?;

        let http_client = reqwest::blocking::Client::builder()
            .user_agent(concat!("firmium-desktop/", env!("CARGO_PKG_VERSION")))
            // Pin TLS verification explicitly (default is on) so it can't be weakened by accident.
            .danger_accept_invalid_certs(false)
            .danger_accept_invalid_hostnames(false)
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let visualizer = Arc::new(VisualizerState::new());
        crate::visualizer::spawn_analysis_task(Arc::clone(&visualizer));

        let eq = Arc::new(EqShared::new(crate::commands::equalizer::resolve_runtime()));

        Ok(AudioPlayer {
            sessions,
            output: RwLock::new(OutputHandle { device, stream }),
            http_client,
            bus,
            crossfade_in_progress: AtomicBool::new(false),
            visualizer,
            bit_perfect_mode: parking_lot::Mutex::new("relaxed".to_string()),
            eq,
        })
    }

    /// Replace the active EQ bands and enable flag (live, affects running tracks).
    pub fn set_eq_runtime(&self, enabled: bool, bands: Vec<EqBand>) {
        self.eq.set(enabled, bands);
    }

    fn bit_perfect_is_strict(&self) -> bool {
        *self.bit_perfect_mode.lock() == "strict"
    }

    /// Enable or disable the audio visualizer analysis task.
    pub fn set_visualizer_enabled(&self, enabled: bool) {
        self.visualizer.set_enabled(enabled);
    }

    /// Shared visualizer analysis state — the iced canvas reads `snapshot()` from it.
    pub fn visualizer(&self) -> Arc<VisualizerState> {
        Arc::clone(&self.visualizer)
    }

    pub fn set_bit_perfect_mode(&self, mode: String) {
        *self.bit_perfect_mode.lock() = mode;
    }

    /// Reopen the output device to match `target_rate`/`target_channels` if they
    /// differ from the currently open stream's config.
    ///
    /// No-ops (returns `Ok(false)`) if: rates already match, a crossfade is in
    /// flight, or the device has no compatible config for this rate (the
    /// session still plays via `output::mix_into`'s resampler in that case).
    fn reopen_stream_if_needed(&self, target_rate: u32, target_channels: u16) -> Result<bool, String> {
        {
            let out = self.output.read();
            if out.stream.sample_rate == target_rate && out.stream.channels == target_channels {
                return Ok(false);
            }
        }
        if self.crossfade_in_progress.load(Ordering::Relaxed) {
            return Ok(false);
        }

        let mut out = self.output.write();
        let Some(config) = output::find_compatible_config(&out.device, target_rate, target_channels) else {
            return Ok(false);
        };

        let new_stream = output::open_with_config(&out.device, config, Arc::clone(&self.sessions))?;
        out.stream = new_stream;
        Ok(true)
    }

    /// Get available audio output devices (real cpal enumeration). Output
    /// routing still uses the system default; the list lets the EQ UI assign a
    /// profile per physical device.
    #[allow(deprecated, dead_code)] // cpal 0.17 deprecates name() in favor of description()/id(); name() is sufficient here
    pub fn list_devices() -> Vec<AudioDevice> {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let default_name = host
            .default_output_device()
            .and_then(|d| d.name().ok());

        let mut devices: Vec<AudioDevice> = host
            .output_devices()
            .map(|iter| {
                iter.filter_map(|d| d.name().ok())
                    .map(|name| {
                        let default = Some(&name) == default_name.as_ref();
                        AudioDevice { name, default }
                    })
                    .collect()
            })
            .unwrap_or_default();

        if devices.is_empty() {
            devices.push(AudioDevice { name: "Default Output".to_string(), default: true });
        }
        devices
    }

    /// Name of the current default output device (used to resolve which device's
    /// EQ profile is audibly active).
    #[allow(deprecated)]
    pub fn default_output_name() -> Option<String> {
        use cpal::traits::{DeviceTrait, HostTrait};
        cpal::default_host().default_output_device().and_then(|d| d.name().ok())
    }

    /// Start streaming and playing a track from a URL.
    pub fn play_stream(self_arc: &Arc<Self>, stream_url: &str, track_id: String, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        Self::start_session(self_arc, stream_url, track_id, false, replay_gain_db)
    }

    /// Pre-fetch and decode a track without starting audio output.
    pub fn preload_stream(self_arc: &Arc<Self>, stream_url: &str, track_id: String, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        Self::start_session(self_arc, stream_url, track_id, true, replay_gain_db)
    }

    /// Shared logic for play_stream and preload_stream.
    /// `start_paused = true` leaves the session paused after decode completes (gapless preload).
    fn start_session(self_arc: &Arc<Self>, stream_url: &str, track_id: String, start_paused: bool, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        let player_id = Uuid::new_v4().to_string();

        self_arc.stop_track(&track_id);

        let session = Arc::new(Session::new(track_id.clone()));
        self_arc.sessions.write().insert(player_id.clone(), Arc::clone(&session));

        if !start_paused {
            self_arc.bus.emit(BackendEvent::PlaybackStateChanged {
                player_id: player_id.clone(),
                state: PlaybackState::Loading,
                audio_info: None,
            });
        }

        let stream_url = stream_url.to_string();
        let player_id_clone = player_id.clone();
        let player = Arc::clone(self_arc);
        let sessions = Arc::clone(&self_arc.sessions);
        let http_client = self_arc.http_client.clone();
        let bus = self_arc.bus.clone();

        tokio::spawn(async move {
            let decode_result = tokio::task::spawn_blocking(move || Self::fetch_and_open(http_client, &stream_url)).await;

            match decode_result {
                Ok(Ok((decoder, duration, shared_buffer, native_rate, native_channels))) => {
                    session.native_sample_rate.store(native_rate, Ordering::Relaxed);
                    session.native_channels.store(native_channels as u32, Ordering::Relaxed);
                    *session.duration.lock() = duration;
                    *session.buffered_bytes.lock() = shared_buffer;

                    // Reopen the output stream to this track's native rate when it's the
                    // sole active session, avoiding resampling for the common case.
                    if sessions.read().len() <= 1 && *player.bit_perfect_mode.lock() != "off" {
                        if let Err(e) = player.reopen_stream_if_needed(native_rate, native_channels) {
                            eprintln!("Output stream reopen failed: {e}");
                        }
                    }

                    let gain_factor = replay_gain_db
                        .filter(|db| db.is_finite())
                        .map(|db| (10f32).powf(db / 20.0).clamp(0.01, 4.0))
                        .unwrap_or(1.0);
                    let cancel = Arc::new(AtomicBool::new(false));
                    let (seek_tx, seek_rx) = std::sync::mpsc::sync_channel(1);
                    *session.cancel.lock() = Arc::clone(&cancel);
                    *session.seek_tx.lock() = Some(seek_tx);

                    session::spawn_decode_feeder(Arc::clone(&session), decoder, true, gain_factor, Arc::clone(&player.visualizer), Arc::clone(&player.eq), player.bit_perfect_is_strict(), cancel, seek_rx);

                    session.loading.store(false, Ordering::Relaxed);

                    if start_paused {
                        session.playing.store(false, Ordering::Relaxed);
                    } else {
                        *session.playback_start_time.lock() = Some(Instant::now());
                        *session.accumulated_time.lock() = 0.0;
                        session.playing.store(true, Ordering::Relaxed);
                        session.has_watcher.store(true, Ordering::Relaxed);

                        bus.emit(BackendEvent::PlaybackStateChanged {
                            player_id: player_id_clone.clone(),
                            state: PlaybackState::Playing,
                            audio_info: Some(AudioInfo { sample_rate: native_rate, channels: native_channels }),
                        });

                        session::spawn_finish_watcher(Arc::clone(&sessions), bus.clone(), player_id_clone.clone());
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Audio decode failed for {}: {}", player_id_clone, e);
                    sessions.write().remove(&player_id_clone);
                    if !start_paused {
                        bus.emit(BackendEvent::PlaybackStateChanged {
                            player_id: player_id_clone.clone(),
                            state: PlaybackState::Stopped,
                            audio_info: None,
                        });
                    }
                }
                Err(join_err) => {
                    eprintln!("Decode task panicked for {}: {}", player_id_clone, join_err);
                    sessions.write().remove(&player_id_clone);
                }
            }
        });

        Ok(player_id)
    }

    /// Fetch (or open a local file) and probe/open a decoder for `url`.
    ///
    /// Returns the decoder, the track's duration (if known), a shared buffer
    /// of consumed bytes (for seek-rebuild fallback — empty for local files,
    /// which can be seeked directly), and the native sample rate/channels.
    fn fetch_and_open(client: reqwest::blocking::Client, url: &str) -> Result<OpenedStream, String> {
        if let Some(path) = url.strip_prefix("file://") {
            return Self::open_local_file(path);
        }

        let response = client.get(url).send().map_err(|e| format!("Failed to fetch stream: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let (mut reader, shared_buffer) = StreamingReader::new(response);
        // Prime the buffer so the format probe has enough bytes to identify the container.
        let _ = reader.fill_to(512 * 1024);

        let (decoder, duration) = DecoderHandle::open(Box::new(reader))?;
        let rate = decoder.sample_rate;
        let channels = decoder.channels;
        Ok((decoder, duration, shared_buffer, rate, channels))
    }

    fn open_local_file(path: &str) -> Result<OpenedStream, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let len = file.metadata().ok().map(|m| m.len());
        let source = FileSource::new(BufReader::with_capacity(256 * 1024, file), len);

        let (decoder, duration) = DecoderHandle::open(Box::new(source))?;
        let rate = decoder.sample_rate;
        let channels = decoder.channels;
        Ok((decoder, duration, Arc::new(Mutex::new(Vec::new())), rate, channels))
    }

    /// Stop and remove the session for `player_id`.
    pub fn stop(&self, player_id: &str) -> Result<(), String> {
        let session = self.sessions.write().remove(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;
        session.playing.store(false, Ordering::Relaxed);
        session.cancel.lock().store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Stop and remove all sessions for `track_id` (called before starting a new one).
    fn stop_track(&self, track_id: &str) {
        let mut sessions = self.sessions.write();
        let to_remove: Vec<PlayerId> = sessions
            .iter()
            .filter(|(_, s)| s.track_id == track_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_remove {
            if let Some(session) = sessions.remove(&id) {
                session.playing.store(false, Ordering::Relaxed);
                session.cancel.lock().store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn pause(&self, player_id: &str) -> Result<(), String> {
        {
            let sessions = self.sessions.read();
            let session = sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;
            if let Some(start) = session.playback_start_time.lock().take() {
                *session.accumulated_time.lock() += start.elapsed().as_secs_f64();
            }
            session.playing.store(false, Ordering::Relaxed);
        }
        self.bus.emit(BackendEvent::PlaybackStateChanged {
            player_id: player_id.to_string(),
            state: PlaybackState::Paused,
            audio_info: None,
        });
        Ok(())
    }

    pub fn resume(&self, player_id: &str) -> Result<(), String> {
        let needs_watcher = {
            let sessions = self.sessions.read();
            let session = sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;
            *session.playback_start_time.lock() = Some(Instant::now());
            session.playing.store(true, Ordering::Relaxed);
            let needs_watcher = !session.has_watcher.load(Ordering::Relaxed);
            if needs_watcher {
                session.has_watcher.store(true, Ordering::Relaxed);
            }
            needs_watcher
        };

        self.bus.emit(BackendEvent::PlaybackStateChanged {
            player_id: player_id.to_string(),
            state: PlaybackState::Playing,
            audio_info: None,
        });

        if needs_watcher {
            session::spawn_finish_watcher(Arc::clone(&self.sessions), self.bus.clone(), player_id.to_string());
        }
        Ok(())
    }

    pub fn set_volume(&self, player_id: &str, volume: f32) -> Result<(), String> {
        let sessions = self.sessions.read();
        let session = sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;
        *session.volume.lock() = volume.clamp(0.0, 1.0);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_volume(&self, player_id: &str) -> Result<f32, String> {
        let sessions = self.sessions.read();
        let session = sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;
        let volume = *session.volume.lock();
        Ok(volume)
    }

    pub fn get_state(&self, player_id: &str) -> Result<PlaybackState, String> {
        let sessions = self.sessions.read();
        let session = sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;

        Ok(if session.loading.load(Ordering::Relaxed) {
            PlaybackState::Loading
        } else if !session.playing.load(Ordering::Relaxed) {
            PlaybackState::Paused
        } else if session.is_empty() {
            PlaybackState::Stopped
        } else {
            PlaybackState::Playing
        })
    }

    #[allow(dead_code)]
    pub fn is_finished(&self, player_id: &str) -> Result<bool, String> {
        let sessions = self.sessions.read();
        match sessions.get(player_id) {
            None => Ok(true),
            Some(session) => {
                if session.loading.load(Ordering::Relaxed) {
                    Ok(false)
                } else {
                    Ok(session.is_empty())
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_duration(&self, player_id: &str) -> Result<Option<f64>, String> {
        let sessions = self.sessions.read();
        let session = sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;
        let duration = *session.duration.lock();
        Ok(duration)
    }

    pub fn get_current_position(&self, player_id: &str) -> Result<f64, String> {
        let sessions = self.sessions.read();
        let session = sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;

        let mut pos = *session.accumulated_time.lock();
        if let Some(start) = *session.playback_start_time.lock() {
            pos += start.elapsed().as_secs_f64();
        }
        Ok(pos)
    }

    /// Seek to `position` seconds. Tries the running decode-feeder's native
    /// symphonia seek first; on failure (or timeout), rebuilds a decoder from
    /// the bytes buffered so far and seeks from there.
    pub fn seek(&self, player_id: &str, position: f64) -> Result<(), String> {
        let position = position.max(0.0);
        let session = {
            let sessions = self.sessions.read();
            Arc::clone(sessions.get(player_id).ok_or_else(|| PLAYER_NOT_FOUND.to_string())?)
        };

        let tx = session.seek_tx.lock().clone();
        if let Some(tx) = tx {
            let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
            if tx.send(SeekRequest { position_secs: position, reply: reply_tx }).is_ok() {
                if let Ok(SeekReply::Ok) = reply_rx.recv_timeout(Duration::from_millis(500)) {
                    Self::apply_seek_position(&session, position);
                    return Ok(());
                }
            }
        }

        // Native seek failed or timed out — rebuild a decoder from the bytes
        // buffered so far and seek from there.
        let raw = session.buffered_bytes.lock().lock().clone();
        if raw.is_empty() {
            return Err("Seek failed: no buffered data available yet".to_string());
        }

        let (mut decoder, _duration) = DecoderHandle::open(Box::new(VecSource::new(raw))).map_err(|e| format!("Failed to re-decode for seek: {}", e))?;
        decoder.seek(Duration::from_secs_f64(position)).map_err(|e| format!("Seek failed: {}", e))?;

        session.cancel.lock().store(true, Ordering::Relaxed);
        session.ring.lock().clear();
        *session.resample.lock() = ResampleState::default();

        let cancel = Arc::new(AtomicBool::new(false));
        let (seek_tx, seek_rx) = std::sync::mpsc::sync_channel(1);
        *session.cancel.lock() = Arc::clone(&cancel);
        *session.seek_tx.lock() = Some(seek_tx);

        // Seek-rebuild feeders don't reapply fade-in or ReplayGain (matches prior behavior).
        session::spawn_decode_feeder(Arc::clone(&session), decoder, false, 1.0, Arc::clone(&self.visualizer), Arc::clone(&self.eq), self.bit_perfect_is_strict(), cancel, seek_rx);

        Self::apply_seek_position(&session, position);
        Ok(())
    }

    fn apply_seek_position(session: &Session, position: f64) {
        *session.accumulated_time.lock() = position;
        *session.playback_start_time.lock() = if session.playing.load(Ordering::Relaxed) { Some(Instant::now()) } else { None };
    }

    /// Crossfade from `old_player_id` to a newly-started session for `stream_url`.
    ///
    /// Ramps the old session's volume down and the new session's volume up
    /// over `STEPS` steps spanning `fade_duration_ms`, then removes the old
    /// session and reopens the output stream to the new track's native rate.
    #[allow(clippy::too_many_arguments)]
    pub fn crossfade_to(
        self_arc: &Arc<Self>,
        old_player_id: &str,
        stream_url: &str,
        track_id: String,
        fade_duration_ms: u64,
        target_volume: f32,
        replay_gain_db: Option<f32>,
        curve: &str,
    ) -> Result<PlayerId, String> {
        self_arc.crossfade_in_progress.store(true, Ordering::Relaxed);

        // Map a 0.0–1.0 ramp position to a volume factor. Logarithmic approximates
        // equal-power (perceptual) fades; linear keeps the raw position.
        fn curve_gain(t: f32, logarithmic: bool) -> f32 {
            if logarithmic { 10f32.powf((t - 1.0) * 2.0) } else { t }
        }
        let logarithmic = curve == "logarithmic";

        let new_player_id = Self::start_session(self_arc, stream_url, track_id, false, replay_gain_db)?;

        if let Some(s) = self_arc.sessions.read().get(&new_player_id) {
            *s.volume.lock() = 0.0;
        }

        let old_id = old_player_id.to_string();
        let new_id = new_player_id.clone();
        let player = Arc::clone(self_arc);
        let old_exists = !old_player_id.is_empty() && self_arc.sessions.read().contains_key(old_player_id);

        tokio::spawn(async move {
            const STEPS: u32 = 25;
            let step_ms = (fade_duration_ms / STEPS as u64).max(50);

            for step in 1..=STEPS {
                tokio::time::sleep(Duration::from_millis(step_ms)).await;
                let progress = step as f32 / STEPS as f32;

                let sessions = player.sessions.read();
                if old_exists {
                    if let Some(session) = sessions.get(&old_id) {
                        *session.volume.lock() = (target_volume * curve_gain(1.0 - progress, logarithmic)).max(0.0);
                    }
                }
                if let Some(session) = sessions.get(&new_id) {
                    *session.volume.lock() = target_volume * curve_gain(progress, logarithmic);
                }
            }

            if old_exists {
                if let Some(session) = player.sessions.write().remove(&old_id) {
                    session.playing.store(false, Ordering::Relaxed);
                    session.cancel.lock().store(true, Ordering::Relaxed);
                }
            }

            player.crossfade_in_progress.store(false, Ordering::Relaxed);

            // Now that the fade is done and only the new session remains, reopen
            // the output stream to its native rate for bit-perfect playback.
            let target = player.sessions.read().get(&new_id).map(|s| (s.sample_rate(), s.channels()));
            if player.sessions.read().len() <= 1 && *player.bit_perfect_mode.lock() != "off" {
                if let Some((rate, channels)) = target {
                    if let Err(e) = player.reopen_stream_if_needed(rate, channels) {
                        eprintln!("Output stream reopen failed: {e}");
                    }
                }
            }
        });

        Ok(new_player_id)
    }

    /// Update the live replay-gain multiplier on every active session.
    /// Pass `1.0` to disable gain (ReplayGain toggled off).
    pub fn set_all_replay_gain_factors(&self, factor: f32) {
        let bits = factor.to_bits();
        for session in self.sessions.read().values() {
            session.replay_gain_factor.store(bits, Ordering::Relaxed);
        }
    }
}
