use std::collections::HashSet;

use crate::commands::mappers::Song;
use crate::events::{BackendEvent, EventBus};
use parking_lot::Mutex;

pub struct QueueState {
    pub inner: Mutex<QueueStateInner>,
}

pub struct QueueStateInner {
    pub queue: Vec<Song>,
    pub queue_idx: i32,
    pub repeat_one: bool,
    pub repeat_all: bool,
    pub shuffle_enabled: bool,
    // Indices played in the current shuffle pass — so each track plays once
    // before any repeats. Cleared whenever the queue identity or shuffle state
    // changes. Not part of the serialized snapshot.
    pub shuffle_played: HashSet<usize>,
    pub crossfade_enabled: bool,
    pub crossfade_duration: f32,
    // "linear" or "logarithmic" — shape of the crossfade volume ramp.
    pub crossfade_curve: String,
    pub gapless_enabled: bool,
    pub replay_gain_enabled: bool,
    // Smart Radio: when the queue is exhausted, ask the frontend to seed and
    // append more tracks instead of stopping.
    pub auto_continue: bool,
    pub volume: f32,
    // Per-track progress flags — reset on each new track
    pub crossfade_started: bool,
    pub preload_started: bool,
    pub cached_duration: Option<f64>,
    pub last_queue_save_position: f64,
    // Active audio session IDs
    pub current_player_id: Option<String>,
    pub preloaded_player_id: Option<String>,
    pub preloaded_track_id: Option<String>,
    // Debounced play-queue save timer
    pub save_timer: Option<tokio::task::JoinHandle<()>>,
}

impl Default for QueueState {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueState {
    pub fn new() -> Self {
        QueueState {
            inner: Mutex::new(QueueStateInner {
                queue: Vec::new(),
                queue_idx: -1,
                repeat_one: false,
                repeat_all: false,
                shuffle_enabled: false,
                shuffle_played: HashSet::new(),
                crossfade_enabled: false,
                crossfade_duration: 5.0,
                crossfade_curve: "linear".to_string(),
                gapless_enabled: true,
                replay_gain_enabled: true,
                auto_continue: false,
                volume: 0.8,
                crossfade_started: false,
                preload_started: false,
                cached_duration: None,
                last_queue_save_position: 0.0,
                current_player_id: None,
                preloaded_player_id: None,
                preloaded_track_id: None,
                save_timer: None,
            }),
        }
    }
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueueStateSnapshot {
    pub queue: Vec<Song>,
    pub queue_idx: i32,
    pub repeat_one: bool,
    pub repeat_all: bool,
    pub shuffle_enabled: bool,
    pub crossfade_enabled: bool,
    pub crossfade_duration: f32,
    pub crossfade_curve: String,
    pub gapless_enabled: bool,
    pub replay_gain_enabled: bool,
    pub volume: f32,
    /// Active audio session ID — forwarded to TS so AudioBridge can filter events correctly.
    pub player_id: Option<String>,
}

impl QueueStateInner {
    pub fn snapshot(&self) -> QueueStateSnapshot {
        QueueStateSnapshot {
            queue: self.queue.clone(),
            queue_idx: self.queue_idx,
            repeat_one: self.repeat_one,
            repeat_all: self.repeat_all,
            shuffle_enabled: self.shuffle_enabled,
            crossfade_enabled: self.crossfade_enabled,
            crossfade_duration: self.crossfade_duration,
            crossfade_curve: self.crossfade_curve.clone(),
            gapless_enabled: self.gapless_enabled,
            replay_gain_enabled: self.replay_gain_enabled,
            volume: self.volume,
            player_id: self.current_player_id.clone(),
        }
    }

    pub fn reset_track_progress(&mut self) {
        self.crossfade_started = false;
        self.preload_started = false;
        self.cached_duration = None;
        self.last_queue_save_position = 0.0;
    }
}

pub fn emit_queue_state(bus: &EventBus, qs: &QueueState) {
    let snapshot = qs.inner.lock().snapshot();
    bus.emit(BackendEvent::QueueStateChanged(snapshot));
}
