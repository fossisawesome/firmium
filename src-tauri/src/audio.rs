//! Audio playback module using rodio for native OS audio engine integration.
//! Provides low-latency, high-quality streaming with minimal CPU usage.
//!
//! Features:
//! - Streaming playback (no full file buffering)
//! - Volume control with memory persistence
//! - Playback state management (Playing, Paused, Stopped, Loading)
//! - Multiple device support
//! - Background playback via dedicated thread
//! - ReplayGain normalization applied per-source via rodio's amplify chain
//! - Native crossfade between sessions with sub-50ms volume steps
//! - Tauri event emission for state changes (eliminates JS polling)
//!
//! Design: Uses OutputStreamHandle (Send + Sync) instead of OutputStream to maintain
//! thread-safety for Tauri managed state. The OutputStream itself is held securely
//! within a Mutex container to guarantee its lifecycle matches the application state.
//!
//! Session lifecycle:
//!   play_stream() → session inserted immediately with loading=true → emits "playback-state-changed" (loading)
//!   async decode → source appended, loading set to false, sink.play() called → emits "playback-state-changed" (playing)
//!   finish watcher detects empty sink → emits "playback-finished" → session removed
//!   get_state()  → returns Loading while buffering, Playing/Paused/Stopped after
//!   is_finished()→ returns Ok(false) while loading; Ok(true) only after audio plays out

use parking_lot::{Mutex, RwLock};
use rodio::mixer::Mixer;
#[cfg(not(target_os = "android"))]
use rodio::cpal;
#[cfg(not(target_os = "android"))]
use rodio::cpal::traits::{DeviceTrait, HostTrait};
#[cfg(not(target_os = "android"))]
use rodio::DeviceSinkBuilder;
use rodio::{Decoder, MixerDeviceSink, Player, Source};
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

/// Decoded audio source plus its duration (if known), the raw byte buffer
/// backing the streaming reader (kept alive for the lifetime of playback),
/// and the source's native sample rate/channel count (used for bit-perfect
/// output device reopening).
type DecodedSource = (Box<dyn Source<Item = f32> + Send>, Option<f64>, Arc<Mutex<Vec<u8>>>, u32, u16);

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
/// Safe because: (1) accessed through RwLock synchronization, (2) MixerDeviceSink
/// itself is thread-safe in practice, just not marked as such in the type system.
struct SafeOutputStream(#[allow(dead_code)] MixerDeviceSink);
unsafe impl Send for SafeOutputStream {}
unsafe impl Sync for SafeOutputStream {}

/// The currently open OS audio output stream plus the mixer feeding it.
///
/// Bundled together so a "bit-perfect" reopen (matching the device's sample
/// rate to the playing track's native rate) swaps both atomically under a
/// single write lock — sessions connect new `Player`s via `mixer.clone()`.
struct OutputDevice {
    mixer: Mixer,
    stream: SafeOutputStream,
    sample_rate: u32,
    channel_count: u16,
}

/// Unique identifier for each playback session
pub type PlayerId = String;

/// Shared session map type, used by the finish-watcher helper which is spawned
/// from contexts that don't have access to `&self`.
type SessionMap = Arc<RwLock<std::collections::HashMap<PlayerId, PlaybackSession>>>;

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
    sessions: SessionMap,
    /// The OS output stream + mixer. Wrapped in RwLock so a track with a
    /// different native sample rate can trigger a "bit-perfect" reopen.
    output: RwLock<OutputDevice>,
    /// Reused HTTP client — avoids rebuilding a connection pool and TLS context on every track.
    http_client: reqwest::blocking::Client,
    /// Tauri app handle for emitting state-change events to the frontend.
    app_handle: AppHandle,
    /// User setting: when true, reopen the output stream to match each
    /// track's native sample rate (avoiding rodio's forced resampling).
    bit_perfect_enabled: AtomicBool,
    /// True while a crossfade's volume ramp is in flight. Reopening the
    /// stream mid-crossfade would silence whichever session is on the old
    /// mixer, so reopens are deferred until the fade completes.
    crossfade_in_progress: AtomicBool,
    /// Shared state for the audio visualizer (sample ring buffer + analysis toggle).
    pub(crate) visualizer: Arc<crate::visualizer::VisualizerState>,
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
        let config = stream.config();
        let sample_rate = config.sample_rate().get();
        let channel_count = config.channel_count().get();
        let http_client = reqwest::blocking::Client::builder()
            .user_agent(concat!("firmium-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let visualizer = Arc::new(crate::visualizer::VisualizerState::new());
        crate::visualizer::spawn_analysis_task(app_handle.clone(), Arc::clone(&visualizer));

        Ok(AudioPlayer {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            output: RwLock::new(OutputDevice {
                mixer,
                stream: SafeOutputStream(stream),
                sample_rate,
                channel_count,
            }),
            http_client,
            app_handle,
            bit_perfect_enabled: AtomicBool::new(true),
            crossfade_in_progress: AtomicBool::new(false),
            visualizer,
        })
    }

    /// Enable or disable bit-perfect output stream reopening.
    pub fn set_bit_perfect_enabled(&self, enabled: bool) {
        self.bit_perfect_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Enable or disable the audio visualizer analysis task.
    pub fn set_visualizer_enabled(&self, enabled: bool) {
        self.visualizer.set_enabled(enabled);
    }

    /// Reopen the output device to match `target_rate`/`target_channels` if they
    /// differ from the currently open stream. Returns `Ok(true)` if a reopen
    /// happened (callers must reconnect their `Player` to the new mixer).
    ///
    /// No-ops (returns `Ok(false)`) if: rates already match, bit-perfect mode is
    /// disabled, or a crossfade is in flight (reopening would silence whichever
    /// session is still attached to the old mixer).
    #[cfg(not(target_os = "android"))]
    fn reopen_stream_if_needed(&self, target_rate: u32, target_channels: u16) -> Result<bool, String> {
        {
            let out = self.output.read();
            if out.sample_rate == target_rate && out.channel_count == target_channels {
                return Ok(false);
            }
        }
        if !self.bit_perfect_enabled.load(Ordering::Relaxed) {
            return Ok(false);
        }
        if self.crossfade_in_progress.load(Ordering::Relaxed) {
            return Ok(false);
        }

        let device = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "No output device".to_string())?;

        let supported = device.supported_output_configs().map_err(|e| e.to_string())?;
        let exact = supported
            .into_iter()
            .find(|c| {
                target_rate >= c.min_sample_rate() && target_rate <= c.max_sample_rate() && c.channels() == target_channels
            })
            .map(|c| c.with_sample_rate(target_rate));

        let builder = DeviceSinkBuilder::from_device(device).map_err(|e| e.to_string())?;
        let new_stream = if let Some(cfg) = exact {
            builder
                .with_supported_config(&cfg)
                .open_sink_or_fallback()
                .map_err(|e| format!("Failed to reopen stream: {e}"))?
        } else {
            // Device doesn't support the track's exact rate — fall back to the
            // nearest supported config (rodio resamples, same as before).
            builder
                .open_sink_or_fallback()
                .map_err(|e| format!("Failed to reopen stream: {e}"))?
        };

        let new_mixer = new_stream.mixer().clone();
        let new_config = new_stream.config();
        let new_sample_rate = new_config.sample_rate().get();
        let new_channel_count = new_config.channel_count().get();

        let mut out = self.output.write();
        out.mixer = new_mixer;
        out.stream = SafeOutputStream(new_stream);
        out.sample_rate = new_sample_rate;
        out.channel_count = new_channel_count;
        Ok(true)
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
    pub fn play_stream(self_arc: &Arc<Self>, stream_url: &str, track_id: String, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        Self::start_session(self_arc, stream_url, track_id, false, replay_gain_db)
    }

    /// Pre-fetch and decode a track without starting audio output.
    ///
    /// The session is created in a paused state so it occupies no audible output.
    /// Call `resume()` on the returned player ID to begin playback instantly,
    /// with no HTTP fetch or decode delay — enabling gapless track transitions.
    pub fn preload_stream(self_arc: &Arc<Self>, stream_url: &str, track_id: String, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        Self::start_session(self_arc, stream_url, track_id, true, replay_gain_db)
    }

    /// Shared logic for play_stream and preload_stream.
    /// `start_paused = true` leaves the sink paused after decode completes (gapless preload).
    fn start_session(self_arc: &Arc<Self>, stream_url: &str, track_id: String, start_paused: bool, replay_gain_db: Option<f32>) -> Result<PlayerId, String> {
        let player_id = Uuid::new_v4().to_string();

        // Stop any existing session for this track before starting a new one.
        self_arc.stop_track(&track_id);

        // Create the player before inserting the session so failures surface early.
        let sink = Player::connect_new(&self_arc.output.read().mixer);

        // ── CRITICAL: Insert the session BEFORE spawning the async task. ─────────
        // The async decode task looks up this player_id to append the source.
        // If we insert after spawn, a fast decode could find nothing and discard audio.
        self_arc.sessions.write().insert(
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
            let _ = self_arc.app_handle.emit("playback-state-changed", serde_json::json!({
                "playerId": player_id,
                "state": "loading"
            }));
        }

        // Clone everything the async task needs — Arc clone is cheap.
        let stream_url = stream_url.to_string();
        let player_id_clone = player_id.clone();
        let player = Arc::clone(self_arc);
        let sessions = Arc::clone(&self_arc.sessions);
        let http_client = self_arc.http_client.clone();
        let app_handle = self_arc.app_handle.clone();

        tauri::async_runtime::spawn(async move {
            // Use spawn_blocking because reqwest blocking I/O and rodio Decoder
            // must run on a thread that is allowed to block.
            let decode_result = tauri::async_runtime::spawn_blocking(move || {
                Self::fetch_and_decode_blocking(http_client, &stream_url)
            })
            .await;

            match decode_result {
                Ok(Ok((source, duration, shared_buffer, native_rate, native_channels))) => {
                    // Try to match the output device to this track's native rate before
                    // appending — avoids rodio resampling for bit-perfect playback.
                    // Only safe when this is the sole active session (no crossfade/preload
                    // overlap on the old mixer); reopen_stream_if_needed() enforces that.
                    #[cfg(not(target_os = "android"))]
                    let reopened = if sessions.read().len() <= 1 {
                        player.reopen_stream_if_needed(native_rate, native_channels).unwrap_or_else(|e| {
                            eprintln!("Bit-perfect stream reopen failed: {e}");
                            false
                        })
                    } else {
                        false
                    };
                    #[cfg(target_os = "android")]
                    let reopened = false;

                    // Apply ReplayGain by wrapping the source in an amplify chain.
                    // This scales sample amplitudes so the sink's volume knob remains
                    // a pure master-volume control unaffected by per-track gain.
                    let amplified: Box<dyn Source<Item = f32> + Send> = if let Some(gain_db) = replay_gain_db {
                        let factor = 10_f32.powf(gain_db / 20.0).clamp(0.01, 4.0);
                        Box::new(source.amplify(factor))
                    } else {
                        source
                    };

                    // Tap samples for the audio visualizer (no-op when disabled).
                    let amplified: Box<dyn Source<Item = f32> + Send> =
                        Box::new(crate::visualizer::tap(amplified, Arc::clone(&player.visualizer)));

                    // Fade-in over 25ms to eliminate the start-of-playback pop.
                    let amplified = amplified.fade_in(Duration::from_millis(25));

                    let mut sesh = sessions.write();
                    if let Some(session) = sesh.get_mut(&player_id_clone) {
                        if reopened {
                            // The old mixer/stream was just dropped — the session's
                            // sink was connected to it and is now silent. Reconnect
                            // to the freshly opened mixer before appending.
                            session.sink = Player::connect_new(&player.output.read().mixer);
                        }
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
                            let output_rate = player.output.read().sample_rate;
                            let _ = app_handle.emit("playback-state-changed", serde_json::json!({
                                "playerId": player_id_clone,
                                "state": "playing",
                                "audioInfo": {
                                    "sampleRate": native_rate,
                                    "channels": native_channels,
                                    "bitPerfect": native_rate == output_rate,
                                }
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
                Self::spawn_finish_watcher(Arc::clone(&sessions), app_handle.clone(), player_id_clone.clone());
                // Record that this session has a running watcher so resume() can
                // skip spawning a duplicate.
                if let Some(s) = sessions.write().get_mut(&player_id_clone) {
                    s.has_watcher = true;
                }
            }
        });

        Ok(player_id)
    }

    /// Spawn a finish-watcher task for `player_id`.
    ///
    /// Polls every 100ms; emits "playback-position" every ~300ms while playing,
    /// and "playback-finished" (removing the session) once the sink drains or
    /// the session is removed externally (e.g. stop()).
    ///
    /// Takes the session map and app handle directly (rather than `&self`) so it
    /// can be called from the 'static async task spawned by `start_session`.
    fn spawn_finish_watcher(sessions: SessionMap, app_handle: AppHandle, player_id: PlayerId) {
        tauri::async_runtime::spawn(async move {
            let mut tick: u8 = 0;
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                tick = tick.wrapping_add(1);

                let (finished, pos_payload) = {
                    let s = sessions.read();
                    match s.get(&player_id) {
                        None => break, // Session removed externally (stop() called).
                        Some(session) => {
                            let finished = !session.loading && session.sink.empty();
                            // Emit position every ~300ms when actively playing.
                            let pos_payload = if tick.is_multiple_of(3) && !session.loading && session.playback_start_time.is_some() {
                                let pos = session.accumulated_time
                                    + session.playback_start_time
                                        .map(|t| t.elapsed().as_secs_f64())
                                        .unwrap_or(0.0);
                                Some(serde_json::json!({
                                    "playerId": player_id,
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
                    let _ = app_handle.emit("playback-position", payload);
                }

                if finished {
                    sessions.write().remove(&player_id);
                    let _ = app_handle.emit("playback-finished", serde_json::json!({
                        "playerId": player_id
                    }));
                    break;
                }
            }
        });
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
    ) -> Result<DecodedSource, String> {
        // Local library tracks are passed as `file://<absolute path>` instead of an
        // HTTP URL — decode directly from disk via a seekable BufReader<File>.
        if let Some(path) = url.strip_prefix("file://") {
            return Self::decode_local_file(path);
        }

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
        let sample_rate = source.sample_rate().get();
        let channel_count = source.channels().get();

        Ok((Box::new(source), duration, shared_buffer, sample_rate, channel_count))
    }

    /// Open and decode a local audio file from disk (local library / downloaded tracks).
    ///
    /// `BufReader<File>` is natively `Read + Seek`, so backward seeks work directly
    /// through the decoder without the in-memory buffer rebuild used for HTTP streams.
    fn decode_local_file(path: &str) -> Result<DecodedSource, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::with_capacity(256 * 1024, file);
        let source =
            Decoder::try_from(reader).map_err(|e| format!("Failed to decode audio: {}", e))?;

        let duration = source.total_duration().map(|d| d.as_secs_f64());
        let sample_rate = source.sample_rate().get();
        let channel_count = source.channels().get();

        Ok((Box::new(source), duration, Arc::new(Mutex::new(Vec::new())), sample_rate, channel_count))
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
            let needs_watcher = sessions.get(player_id).is_some_and(|s| !s.has_watcher);
            if needs_watcher {
                if let Some(s) = sessions.get_mut(player_id) {
                    s.has_watcher = true;
                }
                Self::spawn_finish_watcher(Arc::clone(&self.sessions), self.app_handle.clone(), player_id.to_string());
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
        self_arc: &Arc<Self>,
        old_player_id: &str,
        stream_url: &str,
        track_id: String,
        fade_duration_ms: u64,
        target_volume: f32,
        replay_gain_db: Option<f32>,
    ) -> Result<PlayerId, String> {
        // Both old and new sessions must stay on the same mixer for the
        // duration of the fade — defer any bit-perfect reopen until it ends.
        self_arc.crossfade_in_progress.store(true, Ordering::Relaxed);

        let new_player_id = Self::start_session(self_arc, stream_url, track_id, false, replay_gain_db)?;

        // Mute the new player immediately — the fade task will bring it up.
        {
            let sessions = self_arc.sessions.read();
            if let Some(s) = sessions.get(&new_player_id) {
                s.sink.set_volume(0.0);
            }
        }

        let old_id = old_player_id.to_string();
        let new_id = new_player_id.clone();
        let player = Arc::clone(self_arc);
        let old_exists = !old_player_id.is_empty() && self_arc.sessions.read().contains_key(old_player_id);

        tauri::async_runtime::spawn(async move {
            const STEPS: u32 = 25;
            let step_ms = (fade_duration_ms / STEPS as u64).max(50);

            for step in 1..=STEPS {
                tokio::time::sleep(Duration::from_millis(step_ms)).await;
                let progress = step as f32 / STEPS as f32;

                // Acquire and release the read lock each step so other operations aren't blocked.
                let s = player.sessions.read();
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
                if let Some(s) = player.sessions.write().remove(&old_id) {
                    s.sink.stop();
                }
            }

            player.crossfade_in_progress.store(false, Ordering::Relaxed);
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

        // Fast path: native seek under a short-lived write lock.
        {
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
        }

        // Native seek failed (e.g. ForwardOnly format / backward seek in MP3/OGG).
        // Rebuild the decoder from the in-memory buffer and seek from the start.
        // The buffer clone, decode, and stabilization sleep below all happen
        // WITHOUT holding the sessions lock, so other sessions' watchers, volume
        // changes, and state queries aren't stalled for the duration.
        let (raw, was_paused) = {
            let sessions = self.sessions.read();
            let session = sessions
                .get(player_id)
                .ok_or_else(|| PLAYER_NOT_FOUND.to_string())?;

            let was_paused = session.sink.is_paused();
            // Pause first to allow audio thread to stabilize before switching sinks.
            // This prevents buffer underrun/overrun during rapid seeking.
            if !was_paused {
                session.sink.pause();
            }
            let raw = session.buffered_bytes.lock().clone();
            (raw, was_paused)
        };
        if raw.is_empty() {
            return Err("Seek failed: no buffered data available yet".to_string());
        }

        std::thread::sleep(Duration::from_millis(10));

        let cursor = Cursor::new(raw);
        let mut source =
            Decoder::try_from(cursor).map_err(|e| format!("Failed to re-decode for seek: {}", e))?;

        source
            .try_seek(pos)
            .map_err(|e| format!("Seek failed: {}", e))?;

        let new_sink = Player::connect_new(&self.output.read().mixer);
        new_sink.append(source);
        if was_paused {
            new_sink.pause();
        } else {
            new_sink.play();
        }

        // Re-acquire the lock only to swap in the new sink. If the session was
        // removed (e.g. a concurrent stop()) while we were decoding, stop the
        // new sink before dropping it so its audio thread doesn't keep playing.
        let mut sessions = self.sessions.write();
        let Some(session) = sessions.get_mut(player_id) else {
            new_sink.stop();
            return Err(PLAYER_NOT_FOUND.to_string());
        };
        session.sink.set_volume(0.0);
        session.sink.stop();
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
