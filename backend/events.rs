//! Backend → UI event bus. Replaces Tauri's `app.emit`/`app.listen`.
//!
//! Backend tasks (audio sessions, the queue manager, the OpenSubsonic client)
//! broadcast `BackendEvent`s onto a `tokio::sync::broadcast` channel. Both the
//! queue manager task and the iced subscription hold independent receivers, so
//! the same event fans out to every consumer exactly like the old Tauri
//! emit/listen pair did.

use crate::commands::mappers::Song;
use crate::queue_state::QueueStateSnapshot;
use crate::types::PlaybackState;

/// Native sample rate / channel count reported alongside a "playing" state change.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u16,
}

/// An event emitted by the backend for the UI (and the queue manager) to react to.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BackendEvent {
    PlaybackStateChanged {
        player_id: String,
        state: PlaybackState,
        #[allow(dead_code)]
        audio_info: Option<AudioInfo>,
    },
    PlaybackPosition {
        player_id: String,
        position: f64,
        duration: Option<f64>,
    },
    PlaybackFinished {
        player_id: String,
    },
    QueueStateChanged(QueueStateSnapshot),
    /// Auto-continue (Smart Radio): queue drained — seed more tracks from this one.
    QueueExhausted(Song),
    /// HTTP 401 or OpenSubsonic error 40/41 — credentials no longer valid.
    SessionExpired,
}

/// Cloneable broadcast handle. Cheap to clone; each `subscribe()` returns an
/// independent receiver.
#[derive(Clone)]
pub struct EventBus {
    tx: tokio::sync::broadcast::Sender<BackendEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        // 1024 buffered events. Position ticks (~3/s per track) and state
        // changes are tiny; if a consumer lags past the buffer it only drops
        // intermediate position updates, which self-correct on the next tick.
        let (tx, _rx) = tokio::sync::broadcast::channel(1024);
        Self { tx }
    }

    /// Broadcast an event. `send` errs only when there are zero live receivers
    /// (startup/shutdown) — harmless, so the result is dropped.
    pub fn emit(&self, event: BackendEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<BackendEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
