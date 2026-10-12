//! Bridges the backend `EventBus` to Tauri events the Svelte UI listens for.

use firmium_backend::events::{BackendEvent, EventBus};
use serde_json::json;
use tauri::{AppHandle, Emitter};

pub fn start(app: AppHandle, bus: &EventBus) {
    let mut rx = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            let ev = match rx.recv().await {
                Ok(ev) => ev,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            };
            let _ = match ev {
                BackendEvent::PlaybackStateChanged { player_id, state, .. } => {
                    app.emit("playback-state-changed", json!({ "playerId": player_id, "state": state }))
                }
                BackendEvent::PlaybackPosition { player_id, position, duration } => {
                    app.emit("playback-position", json!({ "playerId": player_id, "position": position, "duration": duration }))
                }
                BackendEvent::PlaybackFinished { player_id } => app.emit("playback-finished", json!({ "playerId": player_id })),
                BackendEvent::QueueStateChanged(s) => app.emit("queue-state-changed", s),
                BackendEvent::SessionExpired => app.emit("session-expired", ()),
                BackendEvent::QueueExhausted(_) => Ok(()),
            };
        }
    });
}
