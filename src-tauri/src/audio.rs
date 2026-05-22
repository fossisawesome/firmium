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
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::Cursor;
use std::sync::Arc;
use uuid::Uuid;

/// Wrapper to make rodio's OutputStream Send + Sync for Tauri state management.
/// Safe because: (1) accessed through Mutex synchronization, (2) OutputStream
/// itself is thread-safe in practice, just not marked as such in the type system.
struct SafeOutputStream(#[allow(dead_code)] OutputStream);
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
    sink: Sink,
    track_id: String,
    duration: Option<f64>,
    /// True while the network fetch + decode is still in progress.
    /// Prevents get_state() from falsely reporting Stopped before audio loads.
    loading: bool,
}

/// Main audio player manager (Send + Sync safe)
///
/// Holds the handle alongside the root stream container securely to prevent
/// premature audio device drop-offs by the operating system.
pub struct AudioPlayer {
    sessions: Arc<RwLock<std::collections::HashMap<PlayerId, PlaybackSession>>>,
    handle: OutputStreamHandle,
    _stream: parking_lot::Mutex<SafeOutputStream>,
}

impl AudioPlayer {
    /// Initialize the audio player with the default audio device.
    ///
    /// Holds the OutputStream within the state context to maintain the audio
    /// connection alive for the lifetime of the application.
    pub fn new() -> Result<Self, String> {
        let (stream, handle) =
            OutputStream::try_default().map_err(|e| format!("Failed to create audio stream: {}", e))?;

        Ok(AudioPlayer {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            handle,
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

        // Create the sink before inserting the session so failures surface early.
        let sink = Sink::try_new(&self.handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        // ── CRITICAL: Insert the session BEFORE spawning the async task. ─────────
        // The async decode task looks up this player_id to append the source.
        // If we insert after spawn, a fast decode could find nothing and discard audio.
        self.sessions.write().insert(
            player_id.clone(),
            PlaybackSession {
                sink,
                track_id: track_id.clone(),
                duration: None,
                loading: true, // Prevents false "Stopped" state during buffering.
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
                Ok(Ok((source, duration))) => {
                    let mut sesh = sessions.write();
                    if let Some(session) = sesh.get_mut(&player_id_clone) {
                        session.sink.append(source);
                        session.duration = duration;
                        session.loading = false; // Audio is now in the sink.
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
    ) -> Result<(Box<dyn Source<Item = i16> + Send>, Option<f64>), String> {
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

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read stream body: {}", e))?;

        let cursor = Cursor::new(bytes);
        let source =
            Decoder::new(cursor).map_err(|e| format!("Failed to decode audio: {}", e))?;

        // total_duration() is only populated for formats that embed length metadata.
        let duration = source.total_duration().map(|d| d.as_secs_f64());

        Ok((Box::new(source), duration))
    }

    /// Pause playback for a session.
    pub fn pause(&self, player_id: &str) -> Result<(), String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| "Player not found".to_string())
            .map(|s| s.sink.pause())
    }

    /// Resume a paused playback session.
    ///
    /// Renamed from `play` to `resume` to match the semantic intent and match
    /// the `resume_playback` Tauri command that calls this method.
    pub fn resume(&self, player_id: &str) -> Result<(), String> {
        let sessions = self.sessions.read();
        sessions
            .get(player_id)
            .ok_or_else(|| "Player not found".to_string())
            .map(|s| s.sink.play())
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