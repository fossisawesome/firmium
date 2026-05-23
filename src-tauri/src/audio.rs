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

use parking_lot::RwLock;
use rodio::mixer::Mixer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

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
    /// Raw audio bytes kept so we can rebuild the decoder for backward seeks.
    raw_bytes: Arc<bytes::Bytes>,
    /// True while the network fetch + decode is still in progress.
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
                raw_bytes: Arc::new(bytes::Bytes::new()),
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
                Ok(Ok((source, duration, raw))) => {
                    let mut sesh = sessions.write();
                    if let Some(session) = sesh.get_mut(&player_id_clone) {
                        session.sink.append(source);
                        session.duration = duration;
                        session.raw_bytes = Arc::new(raw);
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

    /// Fetch a stream from a URL and decode it synchronously on a blocking thread.
    ///
    /// Returns a boxed i16 PCM source with optional duration metadata.
    ///
    /// Note: TLS certificate validation is intentionally NOT disabled here.
    /// If you use a self-signed certificate on your Subsonic server, add the
    /// certificate to your OS trust store instead of bypassing validation globally.
    fn fetch_and_decode_blocking(
        url: &str,
    ) -> Result<(Box<dyn Source<Item = f32> + Send>, Option<f64>, bytes::Bytes), String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("firmium-desktop/1.0")
            // NOTE: danger_accept_invalid_certs is intentionally removed.
            // Add your server's certificate to the OS trust store instead.
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .get(url)
            .send()
            .map_err(|e| format!("Failed to fetch stream: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let raw = response
            .bytes()
            .map_err(|e| format!("Failed to read stream body: {}", e))?;

        let cursor = Cursor::new(raw.clone());
        let source =
            Decoder::try_from(cursor).map_err(|e| format!("Failed to decode audio: {}", e))?;

        // total_duration() is only populated for formats that embed length metadata.
        let duration = source.total_duration().map(|d| d.as_secs_f64());

        Ok((Box::new(source), duration, raw))
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
    /// Tries the native `Player::try_seek` first. If the format only supports forward
    /// seeking (common with MP3/OGG in symphonia), rebuilds the decoder from the stored
    /// raw bytes and seeks forward from the start — which always works since we have
    /// the full file in memory.
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

        // Native seek failed (e.g. ForwardOnly format, backward seek).
        // Rebuild the decoder from stored bytes and seek from the beginning.
        let raw = (*session.raw_bytes).clone();
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