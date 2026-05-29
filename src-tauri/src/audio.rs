/// Audio playback module using rodio for native OS audio engine integration.
/// Provides low-latency, high-quality streaming with minimal CPU usage.
///
/// Features:
/// - Streaming playback (no full file buffering)
/// - Volume control with memory persistence
/// - Playback state management (Playing, Paused, Stopped, Loading)
/// - Multiple device support
/// - Background playback via dedicated thread
/// - ReplayGain normalization applied per-source via rodio's amplify chain
/// - Native crossfade between sessions with sub-50ms volume steps
/// - Tauri event emission for state changes (eliminates JS polling)
///
/// Design: Uses OutputStreamHandle (Send + Sync) instead of OutputStream to maintain
/// thread-safety for Tauri managed state. The OutputStream itself is held securely
/// within a Mutex container to guarantee its lifecycle matches the application state.
///
/// Session lifecycle:
///   play_stream() → session inserted immediately with loading=true → emits "playback-state-changed" (loading)
///   async decode → source appended, loading set to false, sink.play() called → emits "playback-state-changed" (playing)
///   finish watcher detects empty sink → emits "playback-finished" → session removed
///   get_state()  → returns Loading while buffering, Playing/Paused/Stopped after
///   is_finished()→ returns Ok(false) while loading; Ok(true) only after audio plays out

use parking_lot::{Mutex, RwLock};
use rodio::mixer::Mixer;
#[cfg(not(target_os = "android"))]
use rodio::DeviceSinkBuilder;
use rodio::{Decoder, MixerDeviceSink, Player, Source};
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

/// A Read+Seek wrapper over a streaming HTTP response body.
///
/// Bytes are read from the response on demand and buffered locally.
/// This keeps the HTTP connection open for as long as audio is playing,
/// which allows Navidrome (and other OpenSubsonic servers) to maintain the
/// "Now Playing" status for the full track duration rather than just the
/// brief moment the file is being downloaded.
///
/// Backward seeks are supported via the in-memory buffer. Forward seeks
/// past the buffered position drain bytes from the live HTTP connection.
struct StreamingReader {
    response: reqwest::blocking::Response,
    /// Shared buffer so the seek fallback can rebuild the decoder from buffered bytes.
    buffer: Arc<Mutex<Vec<u8>>>,
    pos: usize,
}

impl StreamingReader {
    fn new(response: reqwest::blocking::Response) -> (Self, Arc<Mutex<Vec<u8>>>) {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let reader = Self { response, buffer: Arc::clone(&buffer), pos: 0 };
        (reader, buffer)
    }

    fn fill_to(&mut self, target: usize) -> io::Result<()> {
        {
            let buf = self.buffer.lock();
            if target <= buf.len() {
                return Ok(());
            }
        }
        // Read from the network without holding the lock to avoid blocking other readers.
        let needed = target - self.buffer.lock().len();
        let mut tmp = vec![0u8; needed];
        let mut filled = 0;
        while filled < needed {
            let n = self.response.read(&mut tmp[filled..])?;
            if n == 0 { break; }
            filled += n;
        }
        self.buffer.lock().extend_from_slice(&tmp[..filled]);
        Ok(())
    }
}

impl Read for StreamingReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let buffered = self.buffer.lock();
        if self.pos < buffered.len() {
            let n = buf.len().min(buffered.len() - self.pos);
            buf[..n].copy_from_slice(&buffered[self.pos..self.pos + n]);
            drop(buffered);
            self.pos += n;
            return Ok(n);
        }
        drop(buffered);
        // Read new bytes from the HTTP connection and buffer them.
        let n = self.response.read(buf)?;
        self.buffer.lock().extend_from_slice(&buf[..n]);
        self.pos += n;
        Ok(n)
    }
}

impl Seek for StreamingReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let new_pos = match from {
            SeekFrom::Start(n) => n as usize,
            SeekFrom::Current(n) => (self.pos as i64 + n).max(0) as usize,
            SeekFrom::End(n) => {
                let mut rest = Vec::new();
                self.response.read_to_end(&mut rest)?;
                let mut buf = self.buffer.lock();
                buf.extend_from_slice(&rest);
                (buf.len() as i64 + n).max(0) as usize
            }
        };

        {
            let buf = self.buffer.lock();
            if new_pos > buf.len() {
                drop(buf);
                self.fill_to(new_pos)?;
            }
        }

        let buf = self.buffer.lock();
        self.pos = new_pos.min(buf.len());
        Ok(self.pos as u64)
    }
}

// Safety: reqwest's blocking Response is Send (it wraps a sync I/O handle).
unsafe impl Send for StreamingReader {}
unsafe impl Sync for StreamingReader {}

/// Wrapper to make rodio's MixerDeviceSink Send + Sync for Tauri state management.
/// Safe because: (1) accessed through Mutex synchronization, (2) MixerDeviceSink
/// itself is thread-safe in practice, just not marked as such in the type system.
struct SafeOutputStream(#[allow(dead_code)] MixerDeviceSink);
unsafe impl Send for SafeOutputStream {}
unsafe impl Sync for SafeOutputStream {}

/// Unique identifier for each playback session
pub type PlayerId = String;

const PLAYER_NOT_FOUND: &str = "Player not found";

/// Represents the current playback state.
/// Re-export so callers can use audio::PlaybackState without touching lib.rs directly.
pub use crate::PlaybackState;

pub use crate::AudioDevice;

/// Playback session data
struct PlaybackSession {
    sink: Player,
    track_id: String,
    duration: Option<f64>,
    /// Shared buffer from StreamingReader — used to rebuild the decoder for backward seeks.
    buffered_bytes: Arc<Mutex<Vec<u8>>>,
    /// True while the network fetch + initial decode is still in progress.
    /// Prevents get_state() from falsely reporting Stopped before audio loads.
    loading: bool,
    /// True once a finish-watcher task is running for this session.
    /// Guards against spawning a second watcher when resume() is called on a
    /// session that already got one from start_session().
    has_watcher: bool,
    /// Time when playback started or resumed.
    playback_start_time: Option<std::time::Instant>,
    /// Accumulated playback time (seconds).
    accumulated_time: f64,
}

/// Main audio player manager (Send + Sync safe)
///
/// Holds the handle alongside the root stream container securely to prevent
/// premature audio device drop-offs by the operating system.
pub struct AudioPlayer {
    sessions: Arc<RwLock<std::collections::HashMap<PlayerId, PlaybackSession>>>,
    mixer: Mixer,
    _stream: parking_lot::Mutex<SafeOutputStream>,
    /// Reused HTTP client — avoids rebuilding a connection pool and TLS context on every track.
    http_client: reqwest::blocking::Client,
    /// Tauri app handle for emitting state-change events to the frontend.
    app_handle: AppHandle,
}

impl AudioPlayer {
    /// Initialize the audio player with the default audio device.
    ///
    /// Holds the OutputStream within the state context to maintain the audio
    /// connection alive for the lifetime of the application.
    #[cfg(not(target_os = "android"))]
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("Failed to create audio stream: {}", e))?;
        let mixer = stream.mixer().clone();
        let http_client = reqwest::blocking::Client::builder()
            .user_agent(concat!("firmium-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(AudioPlayer {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            mixer,
            _stream: parking_lot::Mutex::new(SafeOutputStream(stream)),
            http_client,
            app_handle,
        })
    }

    /// Get available audio output devices.
    pub fn list_devices() -> Vec<AudioDevice> {
        vec![AudioDevice {
            name: "Default Output".to_string(),
            default: true,
        }]
    }

    /// Start streaming and playing a track from a URL.
    ///
    /// The session is registered immediately (with `loading = true`) before the
    /// async fetch begins, eliminating the race condition where the decode task
    /// could complete before the session was inserted.
    ///
    /// # Arguments
    /// * `stream_url`     - HTTP/HTTPS URL to the audio stream
    /// * `track_id`       - Application track identifier
    /// * `replay_gain_db` - Optional ReplayGain track gain in dB (applied via Source::amplify)
    ///
    /// # Returns
    /// A unique player ID for this playback session.
    pub fn play_stream(&self, stream_url: &str, track_id: String, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        self.start_session(stream_url, track_id, false, replay_gain_db)
    }

    /// Pre-fetch and decode a track without starting audio output.
    ///
    /// The session is created in a paused state so it occupies no audible output.
    /// Call `resume()` on the returned player ID to begin playback instantly,
    /// with no HTTP fetch or decode delay — enabling gapless track transitions.
    pub fn preload_stream(&self, stream_url: &str, track_id: String, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        self.start_session(stream_url, track_id, true, replay_gain_db)
    }

    /// Shared logic for play_stream and preload_stream.
    /// `start_paused = true` leaves the sink paused after decode completes (gapless preload).
    fn start_session(&self, stream_url: &str, track_id: String, start_paused: bool, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        let player_id = Uuid::new_v4().to_string();

        // Stop any existing session for this track before starting a new one.
        self.stop_track(&track_id);

        // Create the player before inserting the session so failures surface early.
        let sink = Player::connect_new(&self.mixer);

        // ── CRITICAL: Insert the session BEFORE spawning the async task. ─────────
        // The async decode task looks up this player_id to append the source.
        // If we insert after spawn, a fast decode could find nothing and discard audio.
        self.sessions.write().insert(
            player_id.clone(),
            PlaybackSession {
                sink,
                track_id: track_id.clone(),
                duration: None,
                buffered_bytes: Arc::new(Mutex::new(Vec::new())),
                loading: true, // Prevents false "Stopped" state during buffering.
                has_watcher: false,
                playback_start_time: None,
                accumulated_time: 0.0,
            },
        );

        // Only emit events for actively playing sessions, not silent preloads.
        if !start_paused {
            let _ = self.app_handle.emit("playback-state-changed", serde_json::json!({
                "playerId": player_id,
                "state": "loading"
            }));
        }

        // Clone everything the async task needs — Arc clone is cheap.
        let stream_url = stream_url.to_string();
        let player_id_clone = player_id.clone();
        let sessions = Arc::clone(&self.sessions);
        let http_client = self.http_client.clone();
        let app_handle = self.app_handle.clone();

        tauri::async_runtime::spawn(async move {
            // Use spawn_blocking because reqwest blocking I/O and rodio Decoder
            // must run on a thread that is allowed to block.
            let decode_result = tauri::async_runtime::spawn_blocking(move || {
                Self::fetch_and_decode_blocking(http_client, &stream_url)
            })
            .await;

            match decode_result {
                Ok(Ok((source, duration, shared_buffer))) => {
                    // Apply ReplayGain by wrapping the source in an amplify chain.
                    // This scales sample amplitudes so the sink's volume knob remains
                    // a pure master-volume control unaffected by per-track gain.
                    let amplified: Box<dyn Source<Item = f32> + Send> = if let Some(gain_db) = replay_gain_db {
                        let factor = 10_f32.powf(gain_db / 20.0).clamp(0.01, 4.0);
                        Box::new(source.amplify(factor))
                    } else {
                        source
                    };

                    // Fade-in over 25ms to eliminate the start-of-playback pop.
                    let amplified = amplified.fade_in(Duration::from_millis(25));

                    let mut sesh = sessions.write();
                    if let Some(session) = sesh.get_mut(&player_id_clone) {
                        session.sink.append(amplified);
                        session.duration = duration;
                        session.buffered_bytes = shared_buffer;
                        session.loading = false; // Audio is now in the sink.
                        if start_paused {
                            // Preloaded session — stay paused until promoted by the frontend.
                            session.sink.pause();
                        } else {
                            session.playback_start_time = Some(std::time::Instant::now());
                            session.accumulated_time = 0.0;
                            session.sink.play();
                            let _ = app_handle.emit("playback-state-changed", serde_json::json!({
                                "playerId": player_id_clone,
                                "state": "playing"
                            }));
                        }
                    }
                    // If session was removed (e.g. user skipped) before decode
                    // finished, silently discard — the sink was already stopped.
                }
                Ok(Err(e)) => {
                    eprintln!("Audio decode failed for {}: {}", player_id_clone, e);
                    sessions.write().remove(&player_id_clone);
                    if !start_paused {
                        let _ = app_handle.emit("playback-state-changed", serde_json::json!({
                            "playerId": player_id_clone,
                            "state": "stopped"
                        }));
                    }
                }
                Err(join_err) => {
                    eprintln!("Blocking task panicked for {}: {}", player_id_clone, join_err);
                    sessions.write().remove(&player_id_clone);
                }
            }

            // Finish watcher — polls at 100ms, emits "playback-position" every ~300ms and
            // "playback-finished" when the sink drains. Only runs for non-preloaded sessions.
            if !start_paused {
                let sessions_watch = Arc::clone(&sessions);
                let app_handle_watch = app_handle.clone();
                let pid = player_id_clone.clone();
                tauri::async_runtime::spawn(async move {
                    let mut tick: u8 = 0;
                    loop {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        tick = tick.wrapping_add(1);

                        let (finished, pos_payload) = {
                            let s = sessions_watch.read();
                            match s.get(&pid) {
                                None => break, // Session removed externally (stop() called).
                                Some(session) => {
                                    let finished = !session.loading && session.sink.empty();
                                    // Emit position every ~300ms when actively playing.
                                    let pos_payload = if tick % 3 == 0 && !session.loading && session.playback_start_time.is_some() {
                                        let pos = session.accumulated_time
                                            + session.playback_start_time
                                                .map(|t| t.elapsed().as_secs_f64())
                                                .unwrap_or(0.0);
                                        Some(serde_json::json!({
                                            "playerId": pid,
                                            "position": pos,
                                            "duration": session.duration
                                        }))
                                    } else {
                                        None
                                    };
                                    (finished, pos_payload)
                                }
                            }
                        };

                        if let Some(payload) = pos_payload {
                            let _ = app_handle_watch.emit("playback-position", payload);
                        }

                        if finished {
                            sessions_watch.write().remove(&pid);
                            let _ = app_handle_watch.emit("playback-finished", serde_json::json!({
                                "playerId": pid
                            }));
                            break;
                        }
                    }
                });
                // Record that this session has a running watcher so resume() can
                // skip spawning a duplicate.
                if let Some(s) = sessions.write().get_mut(&player_id_clone) {
                    s.has_watcher = true;
                }
            }
        });

        Ok(player_id)
    }

    /// Fetch a stream from a URL and open a streaming decoder.
    ///
    /// Unlike a full-download approach, this keeps the HTTP connection open and
    /// reads audio bytes on demand as rodio consumes them. Because Navidrome
    /// tracks "Now Playing" based on active stream connections, keeping this
    /// connection alive for the song duration ensures the admin panel reflects
    /// the correct playback state.
    ///
    /// Note: TLS certificate validation is intentionally NOT disabled here.
    /// If you use a self-signed certificate on your OpenSubsonic server, add the
    /// certificate to your OS trust store instead of bypassing validation globally.
    fn fetch_and_decode_blocking(
        client: reqwest::blocking::Client,
        url: &str,
    ) -> Result<(Box<dyn Source<Item = f32> + Send>, Option<f64>, Arc<Mutex<Vec<u8>>>), String> {
        let response = client
            .get(url)
            .send()
            .map_err(|e| format!("Failed to fetch stream: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let (mut streaming_reader, shared_buffer) = StreamingReader::new(response);
        // Pre-buffer 512 KB before decoding so the rodio output thread has data
        // from the first callback and isn't immediately starved by network jitter.
        let _ = streaming_reader.fill_to(512 * 1024);
        let reader = BufReader::with_capacity(256 * 1024, streaming_reader);
        let source =
            Decoder::try_from(reader).map_err(|e| format!("Failed to decode audio: {}", e))?;

        let duration = source.total_duration().map(|d| d.as_secs_f64());

        Ok((Box::new(source), duration, shared_buffer))
    }

    /// Pause playback for a session.
    pub fn pause(&self, player_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write();
        let result = sessions
            .get_mut(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| {
                // Mute instantly to eliminate the pause pop without blocking the
                // tokio executor with thread::sleep under a write lock.
                let vol = s.sink.volume();
                s.sink.set_volume(0.0);
                // Save accumulated time when pausing
                if let Some(start) = s.playback_start_time {
                    s.accumulated_time += start.elapsed().as_secs_f64();
                    s.playback_start_time = None;
                }
                s.sink.pause();
                s.sink.set_volume(vol); // Restore so resume plays at the correct level.
            });
        if result.is_ok() {
            let _ = self.app_handle.emit("playback-state-changed", serde_json::json!({
                "playerId": player_id,
                "state": "paused"
            }));
        }
        result
    }

    /// Resume a paused playback session.
    ///
    /// Renamed from `play` to `resume` to match the semantic intent and match
    /// the `resume_playback` Tauri command that calls this method.
    pub fn resume(&self, player_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write();
        let result = sessions
            .get_mut(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| {
                // Reset start time when resuming
                s.playback_start_time = Some(std::time::Instant::now());
                s.sink.play()
            });
        if result.is_ok() {
            let _ = self.app_handle.emit("playback-state-changed", serde_json::json!({
                "playerId": player_id,
                "state": "playing"
            }));

            // Promoted preloads need their own finish watcher + position emitter.
            // Non-preloaded sessions already have one from start_session(), so skip them
            // to avoid duplicate position/finished events after pause-resume cycles.
            let needs_watcher = sessions.get(player_id).map_or(false, |s| !s.has_watcher);
            if needs_watcher {
                if let Some(s) = sessions.get_mut(player_id) {
                    s.has_watcher = true;
                }
                let sessions_watch = Arc::clone(&self.sessions);
                let app_handle_watch = self.app_handle.clone();
                let pid = player_id.to_string();
                tauri::async_runtime::spawn(async move {
                    let mut tick: u8 = 0;
                    loop {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        tick = tick.wrapping_add(1);

                        let (finished, pos_payload) = {
                            let s = sessions_watch.read();
                            match s.get(&pid) {
                                None => break,
                                Some(session) => {
                                    let finished = !session.loading && session.sink.empty();
                                    let pos_payload = if tick % 3 == 0 && !session.loading && session.playback_start_time.is_some() {
                                        let pos = session.accumulated_time
                                            + session.playback_start_time
                                                .map(|t| t.elapsed().as_secs_f64())
                                                .unwrap_or(0.0);
                                        Some(serde_json::json!({
                                            "playerId": pid,
                                            "position": pos,
                                            "duration": session.duration
                                        }))
                                    } else {
                                        None
                                    };
                                    (finished, pos_payload)
                                }
                            }
                        };

                        if let Some(payload) = pos_payload {
                            let _ = app_handle_watch.emit("playback-position", payload);
                        }

                        if finished {
                            sessions_watch.write().remove(&pid);
                            let _ = app_handle_watch.emit("playback-finished", serde_json::json!({
                                "playerId": pid
                            }));
                            break;
                        }
                    }
                });
            }
        }
        result
    }

    /// Stop playback and remove the session entirely.
    pub fn stop(&self, player_id: &str) -> Result<(), String> {
        // Mute and stop under a single write lock; avoids blocking the tokio
        // executor with thread::sleep under a lock.
        self.sessions
            .write()
            .remove(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| {
                s.sink.set_volume(0.0);
                s.sink.stop()
            })
    }

    /// Stop all active sessions for a given track ID (used before starting a new session).
    fn stop_track(&self, track_id: &str) {
        let mut sessions = self.sessions.write();
        sessions.retain(|_, session| {
            if session.track_id == track_id {
                session.sink.stop();
                false
            } else {
                true
            }
        });
    }

    /// Cross-fade from `old_player_id` into a new stream over `fade_duration_ms` milliseconds.
    ///
    /// Starts the new stream, sets its initial volume to 0, then ramps old→0 and new→target
    /// in a background Tokio task using 25 evenly-spaced steps. The old session is stopped
    /// and removed after the fade completes.
    ///
    /// Returns the new player ID so the caller can track the incoming session.
    pub fn crossfade_to(
        &self,
        old_player_id: &str,
        stream_url: &str,
        track_id: String,
        fade_duration_ms: u64,
        target_volume: f32,
        replay_gain_db: Option<f32>,
    ) -> Result<PlayerId, String> {
        let new_player_id = self.start_session(stream_url, track_id, false, replay_gain_db)?;

        // Mute the new player immediately — the fade task will bring it up.
        {
            let sessions = self.sessions.read();
            if let Some(s) = sessions.get(&new_player_id) {
                s.sink.set_volume(0.0);
            }
        }

        let old_id = old_player_id.to_string();
        let new_id = new_player_id.clone();
        let sessions = Arc::clone(&self.sessions);
        let old_exists = !old_player_id.is_empty() && self.sessions.read().contains_key(old_player_id);

        tauri::async_runtime::spawn(async move {
            const STEPS: u32 = 25;
            let step_ms = (fade_duration_ms / STEPS as u64).max(50);

            for step in 1..=STEPS {
                tokio::time::sleep(Duration::from_millis(step_ms)).await;
                let progress = step as f32 / STEPS as f32;

                // Acquire and release the read lock each step so other operations aren't blocked.
                let s = sessions.read();
                if old_exists {
                    if let Some(sess) = s.get(&old_id) {
                        sess.sink.set_volume((target_volume * (1.0 - progress)).max(0.0));
                    }
                }
                if let Some(sess) = s.get(&new_id) {
                    sess.sink.set_volume(target_volume * progress);
                }
            }

            // Stop old session after fade.
            if old_exists {
                if let Some(s) = sessions.write().remove(&old_id) {
                    s.sink.stop();
                }
            }
        });

        Ok(new_player_id)
    }

    /// Get current playback state.
    ///
    /// Returns `Loading` while the initial network fetch is in progress so the
    /// frontend does not mistake the pre-audio empty sink for a finished track.
    pub fn get_state(&self, player_id: &str) -> Result<PlaybackState, String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| {
                if s.loading {
                    PlaybackState::Loading
                } else if s.sink.is_paused() {
                    PlaybackState::Paused
                } else if s.sink.empty() {
                    PlaybackState::Stopped
                } else {
                    PlaybackState::Playing
                }
            })
    }

    /// Set volume (0.0 to 1.0). Value is clamped to the valid range.
    pub fn set_volume(&self, player_id: &str, volume: f32) -> Result<(), String> {
        let volume = volume.clamp(0.0, 1.0);
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| s.sink.set_volume(volume))
    }

    /// Get current volume (0.0 to 1.0).
    pub fn get_volume(&self, player_id: &str) -> Result<f32, String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| s.sink.volume())
    }

    /// Check if playback has finished.
    ///
    /// Returns `Ok(false)` while audio is still loading (sink is empty but not done).
    /// Returns `Ok(true)` only after audio has been appended and the sink drains.
    /// Returns `Ok(true)` if the session no longer exists (was stopped/cleaned up)
    /// so the monitoring loop in the frontend does not get stuck.
    pub fn is_finished(&self, player_id: &str) -> Result<bool, String> {
        let sessions = self.sessions.read();
        match sessions.get(player_id) {
            None => Ok(true), // Session removed = effectively finished.
            Some(s) => {
                if s.loading {
                    Ok(false) // Still buffering — not finished yet.
                } else {
                    Ok(s.sink.empty())
                }
            }
        }
    }

    /// Get track duration in seconds if available (only populated after decode).
    pub fn get_duration(&self, player_id: &str) -> Result<Option<f64>, String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| s.duration)
    }

    /// Get current playback position in seconds.
    ///
    /// Calculates position based on accumulated time and elapsed time since playback started.
    pub fn get_current_position(&self, player_id: &str) -> Result<f64, String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
            .map(|s| {
                let mut position = s.accumulated_time;
                if let Some(start) = s.playback_start_time {
                    position += start.elapsed().as_secs_f64();
                }
                position
            })
    }

    /// Seek to a position in seconds.
    ///
    /// Uses the native Player::try_seek. Because the underlying StreamingReader
    /// buffers all bytes already consumed, symphonia can seek backward through
    /// the buffer without re-fetching from the network.
    pub fn seek(&self, player_id: &str, position: f64) -> Result<(), String> {
        let pos = Duration::from_secs_f64(position.max(0.0));
        let mut sessions = self.sessions.write();
        let session = sessions
            .get_mut(player_id)
            .ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;

        if session.sink.try_seek(pos).is_ok() {
            session.accumulated_time = position.max(0.0);
            session.playback_start_time = if session.sink.is_paused() {
                None
            } else {
                Some(std::time::Instant::now())
            };
            return Ok(());
        }

        // Native seek failed (e.g. ForwardOnly format / backward seek in MP3/OGG).
        // Rebuild the decoder from the in-memory buffer and seek from the start.
        let raw = session.buffered_bytes.lock().clone();
        if raw.is_empty() {
            return Err("Seek failed: no buffered data available yet".to_string());
        }
        let cursor = Cursor::new(raw);
        let mut source =
            Decoder::try_from(cursor).map_err(|e| format!("Failed to re-decode for seek: {}", e))?;

        source
            .try_seek(pos)
            .map_err(|e| format!("Seek failed: {}", e))?;

        let was_paused = session.sink.is_paused();
        // Pause first to allow audio thread to stabilize before switching sinks.
        // This prevents buffer underrun/overrun during rapid seeking.
        if !was_paused {
            session.sink.pause();
        }
        std::thread::sleep(Duration::from_millis(10));
        session.sink.stop();

        let new_sink = Player::connect_new(&self.mixer);
        new_sink.append(source);
        if was_paused {
            new_sink.pause();
        } else {
            new_sink.play();
        }

        session.sink = new_sink;
        session.accumulated_time = position.max(0.0);
        session.playback_start_time = if was_paused {
            None
        } else {
            Some(std::time::Instant::now())
        };

        Ok(())
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        panic!("AudioPlayer::default() is not supported; use AudioPlayer::new(app_handle)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_state_serialization() {
        let state = PlaybackState::Playing;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"playing\"");
    }

    #[test]
    fn test_loading_state_serialization() {
        let state = PlaybackState::Loading;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"loading\"");
    }
}
