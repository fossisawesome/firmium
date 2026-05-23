/// Audio playback module using rodio for native OS audio engine integration.
/// Provides low-latency, high-quality streaming with minimal CPU usage.
///
/// Features:
/// - Streaming playback (no full file buffering)
/// - Volume control with memory persistence
/// - Playback state management (Playing, Paused, Stopped, Loading)
/// - Multiple device support
/// - Background playback via dedicated thread
///
/// Design: Uses OutputStreamHandle (Send + Sync) instead of OutputStream to maintain
/// thread-safety for Tauri managed state. The OutputStream itself is held securely
/// within a Mutex container to guarantee its lifecycle matches the application state.
///
/// Session lifecycle:
///   play_stream() → session inserted immediately with loading=true
///   async decode → source appended, loading set to false, sink.play() called
///   get_state()  → returns Loading while buffering, Playing/Paused/Stopped after
///   is_finished()→ returns Ok(false) while loading; Ok(true) only after audio plays out

use parking_lot::{Mutex, RwLock};
use rodio::mixer::Mixer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// A Read+Seek wrapper over a streaming HTTP response body.
///
/// Bytes are read from the response on demand and buffered locally.
/// This keeps the HTTP connection open for as long as audio is playing,
/// which allows Navidrome (and other Subsonic servers) to maintain the
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
        let mut buf = self.buffer.lock();
        if target <= buf.len() {
            return Ok(());
        }
        let needed = target - buf.len();
        let prev = buf.len();
        buf.resize(prev + needed, 0);
        let mut filled = 0;
        while filled < needed {
            let n = self.response.read(&mut buf[prev + filled..])?;
            if n == 0 { break; }
            filled += n;
        }
        buf.truncate(prev + filled);
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

/// Represents the current playback state.
///
/// `Loading` is the initial state while the audio is being fetched and decoded.
/// The frontend should not treat a Loading session as finished.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

/// Audio device information
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub default: bool,
}

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
}

impl AudioPlayer {
    /// Initialize the audio player with the default audio device.
    ///
    /// Holds the OutputStream within the state context to maintain the audio
    /// connection alive for the lifetime of the application.
    pub fn new() -> Result<Self, String> {
        let stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("Failed to create audio stream: {}", e))?;
        let mixer = stream.mixer().clone();

        Ok(AudioPlayer {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            mixer,
            _stream: parking_lot::Mutex::new(SafeOutputStream(stream)),
        })
    }

    /// Get available audio output devices.
    /// On most systems rodio abstracts device selection; returns the default device.
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
    /// * `stream_url` - HTTP/HTTPS URL to the audio stream
    /// * `track_id`   - Application track identifier
    ///
    /// # Returns
    /// A unique player ID for this playback session.
    pub fn play_stream(&self, stream_url: &str, track_id: String) -> Result<PlayerId, String> {
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
                playback_start_time: None,
                accumulated_time: 0.0,
            },
        );

        // Clone everything the async task needs — Arc clone is cheap.
        let stream_url = stream_url.to_string();
        let player_id_clone = player_id.clone();
        let sessions = Arc::clone(&self.sessions);

        tauri::async_runtime::spawn(async move {
            // Use spawn_blocking because reqwest blocking I/O and rodio Decoder
            // must run on a thread that is allowed to block.
            let decode_result = tauri::async_runtime::spawn_blocking(move || {
                Self::fetch_and_decode_blocking(&stream_url)
            })
            .await;

            match decode_result {
                Ok(Ok((source, duration, shared_buffer))) => {
                    let mut sesh = sessions.write();
                    if let Some(session) = sesh.get_mut(&player_id_clone) {
                        session.sink.append(source);
                        session.duration = duration;
                        session.buffered_bytes = shared_buffer;
                        session.loading = false; // Audio is now in the sink.
                        session.playback_start_time = Some(std::time::Instant::now());
                        session.accumulated_time = 0.0;
                        session.sink.play();
                    }
                    // If session was removed (e.g. user skipped) before decode
                    // finished, silently discard — the sink was already stopped.
                }
                Ok(Err(e)) => {
                    eprintln!("Audio decode failed for {}: {}", player_id_clone, e);
                    sessions.write().remove(&player_id_clone);
                }
                Err(join_err) => {
                    eprintln!("Blocking task panicked for {}: {}", player_id_clone, join_err);
                    sessions.write().remove(&player_id_clone);
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
    /// If you use a self-signed certificate on your Subsonic server, add the
    /// certificate to your OS trust store instead of bypassing validation globally.
    fn fetch_and_decode_blocking(
        url: &str,
    ) -> Result<(Box<dyn Source<Item = f32> + Send>, Option<f64>, Arc<Mutex<Vec<u8>>>), String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("firmium-desktop/1.0")
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .get(url)
            .send()
            .map_err(|e| format!("Failed to fetch stream: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let (streaming_reader, shared_buffer) = StreamingReader::new(response);
        let reader = BufReader::new(streaming_reader);
        let source =
            Decoder::try_from(reader).map_err(|e| format!("Failed to decode audio: {}", e))?;

        let duration = source.total_duration().map(|d| d.as_secs_f64());

        Ok((Box::new(source), duration, shared_buffer))
    }

    /// Pause playback for a session.
    pub fn pause(&self, player_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write();
        sessions
            .get_mut(player_id)
            .ok_or_else(|| "Player not found".to_string())
            .map(|s| {
                // Save accumulated time when pausing
                if let Some(start) = s.playback_start_time {
                    s.accumulated_time += start.elapsed().as_secs_f64();
                    s.playback_start_time = None;
                }
                s.sink.pause()
            })
    }

    /// Resume a paused playback session.
    ///
    /// Renamed from `play` to `resume` to match the semantic intent and match
    /// the `resume_playback` Tauri command that calls this method.
    pub fn resume(&self, player_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write();
        sessions
            .get_mut(player_id)
            .ok_or_else(|| "Player not found".to_string())
            .map(|s| {
                // Reset start time when resuming
                s.playback_start_time = Some(std::time::Instant::now());
                s.sink.play()
            })
    }

    /// Stop playback and remove the session entirely.
    pub fn stop(&self, player_id: &str) -> Result<(), String> {
        self.sessions
            .write()
            .remove(player_id)
            .ok_or_else(|| "Player not found".to_string())
            .map(|s| s.sink.stop())
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

    /// Get current playback state.
    ///
    /// Returns `Loading` while the initial network fetch is in progress so the
    /// frontend does not mistake the pre-audio empty sink for a finished track.
    pub fn get_state(&self, player_id: &str) -> Result<PlaybackState, String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| "Player not found".to_string())
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
            .ok_or_else(|| "Player not found".to_string())
            .map(|s| s.sink.set_volume(volume))
    }

    /// Get current volume (0.0 to 1.0).
    pub fn get_volume(&self, player_id: &str) -> Result<f32, String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| "Player not found".to_string())
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
            .ok_or_else(|| "Player not found".to_string())
            .map(|s| s.duration)
    }

    /// Get current playback position in seconds.
    ///
    /// Calculates position based on accumulated time and elapsed time since playback started.
    pub fn get_current_position(&self, player_id: &str) -> Result<f64, String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| "Player not found".to_string())
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
            .ok_or_else(|| "Player not found".to_string())?;

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
        Self::new().expect("Failed to initialize audio player")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_player_creation() {
        let player = AudioPlayer::new();
        assert!(player.is_ok());
    }

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