//! Playback session state shared between the Tauri command layer, the
//! decode-feeder task, and the realtime cpal mixing callback.

use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use super::decoder::DecoderHandle;
use super::eq::{self, EqChain, EqShared};
use crate::visualizer::{self, VisualizerState};

/// Unique identifier for each playback session.
pub type PlayerId = String;

/// Shared session map.
pub type SessionMap = Arc<RwLock<HashMap<PlayerId, Arc<Session>>>>;

/// Number of interleaved f32 samples to keep buffered ahead of the output
/// callback (roughly half a second at 48kHz stereo).
pub const RING_HIGH_WATER: usize = 48_000;

/// Linear-interpolation resample cursor used when a session's native sample
/// rate doesn't match the currently-open output stream's rate. When the
/// rates match, `step` is 1.0 and this degenerates to an exact passthrough
/// (no resampling) — see `output::mix_into`.
#[derive(Default)]
pub struct ResampleState {
    pub pos: f64,
    pub current: Vec<f32>,
    pub next: Vec<f32>,
    pub initialized: bool,
}

/// Sent to a running decode-feeder task to request a native seek.
pub struct SeekRequest {
    pub position_secs: f64,
    pub reply: SyncSender<SeekReply>,
}

pub enum SeekReply {
    Ok,
    Failed,
}

pub struct Session {
    /// Decoded interleaved f32 samples awaiting playback.
    pub ring: Mutex<VecDeque<f32>>,
    pub volume: Mutex<f32>,
    /// Live-updateable replay gain multiplier (stored as f32 bits in an AtomicU32).
    pub replay_gain_factor: AtomicU32,
    pub playing: AtomicBool,
    /// Set once the decode-feeder hits end-of-stream.
    pub finished_decoding: AtomicBool,
    /// True while the initial network fetch + decoder setup is in progress.
    pub loading: AtomicBool,
    pub has_watcher: AtomicBool,
    pub native_sample_rate: AtomicU32,
    pub native_channels: AtomicU32,
    pub track_id: String,
    pub duration: Mutex<Option<f64>>,
    /// Raw bytes consumed so far — used to rebuild the decoder for seeks
    /// that the native `DecoderHandle::seek` can't satisfy.
    pub buffered_bytes: Mutex<Arc<Mutex<Vec<u8>>>>,
    pub playback_start_time: Mutex<Option<Instant>>,
    pub accumulated_time: Mutex<f64>,
    pub resample: Mutex<ResampleState>,
    /// Channel to the currently-running decode-feeder for native seeks.
    pub seek_tx: Mutex<Option<SyncSender<SeekRequest>>>,
    /// Cancels the currently-running decode-feeder (set when replaced by a
    /// seek-rebuild feeder).
    pub cancel: Mutex<Arc<AtomicBool>>,
}

impl Session {
    pub fn new(track_id: String) -> Self {
        Self {
            ring: Mutex::new(VecDeque::with_capacity(RING_HIGH_WATER * 2)),
            volume: Mutex::new(1.0),
            replay_gain_factor: AtomicU32::new(1.0f32.to_bits()),
            playing: AtomicBool::new(false),
            finished_decoding: AtomicBool::new(false),
            loading: AtomicBool::new(true),
            has_watcher: AtomicBool::new(false),
            native_sample_rate: AtomicU32::new(44_100),
            native_channels: AtomicU32::new(2),
            track_id,
            duration: Mutex::new(None),
            buffered_bytes: Mutex::new(Arc::new(Mutex::new(Vec::new()))),
            playback_start_time: Mutex::new(None),
            accumulated_time: Mutex::new(0.0),
            resample: Mutex::new(ResampleState::default()),
            seek_tx: Mutex::new(None),
            cancel: Mutex::new(Arc::new(AtomicBool::new(false))),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.finished_decoding.load(Ordering::Relaxed) && self.ring.lock().is_empty()
    }

    pub fn channels(&self) -> u16 {
        self.native_channels.load(Ordering::Relaxed) as u16
    }

    pub fn sample_rate(&self) -> u32 {
        self.native_sample_rate.load(Ordering::Relaxed)
    }
}

/// Spawn a blocking task that continuously decodes `decoder` into `session.ring`.
///
/// `apply_fade_in` ramps the first 25ms of audio linearly from silence (used
/// only for the initial decode-feeder of a track, not seek-rebuilds).
/// `replay_gain_factor` is written to `session.replay_gain_factor` so it can be
/// updated live (e.g. when the user toggles ReplayGain off). `visualizer`
/// receives a copy of each decoded chunk for analysis.
#[allow(clippy::too_many_arguments)]
pub fn spawn_decode_feeder(
    session: Arc<Session>,
    mut decoder: DecoderHandle,
    apply_fade_in: bool,
    replay_gain_factor: f32,
    visualizer: Arc<VisualizerState>,
    eq: Arc<EqShared>,
    bit_perfect_strict: bool,
    cancel: Arc<AtomicBool>,
    seek_rx: Receiver<SeekRequest>,
) {
    session.replay_gain_factor.store(replay_gain_factor.to_bits(), Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || {
        let channels = decoder.channels.max(1) as usize;
        let sample_rate = decoder.sample_rate;

        // EQ chain is rebuilt lazily whenever the shared config generation changes.
        // Strict bit-perfect mode bypasses EQ entirely to keep the signal untouched.
        let mut eq_chain: Option<EqChain> = None;
        let mut eq_gen: u64 = 0;
        let fade_total = if apply_fade_in {
            (sample_rate as usize * channels * 25 / 1000).max(channels)
        } else {
            0
        };
        let mut fade_done = 0usize;

        loop {
            if cancel.load(Ordering::Relaxed) {
                return;
            }

            // Handle a pending seek request.
            if let Ok(req) = seek_rx.try_recv() {
                match decoder.seek(Duration::from_secs_f64(req.position_secs.max(0.0))) {
                    Ok(()) => {
                        session.ring.lock().clear();
                        *session.resample.lock() = ResampleState::default();
                        let _ = req.reply.send(SeekReply::Ok);
                    }
                    Err(_) => {
                        let _ = req.reply.send(SeekReply::Failed);
                    }
                }
            }

            // Backpressure: don't decode further than the ring can hold.
            while session.ring.lock().len() > RING_HIGH_WATER {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
                // Keep servicing seek requests while backpressured.
                if let Ok(req) = seek_rx.try_recv() {
                    match decoder.seek(Duration::from_secs_f64(req.position_secs.max(0.0))) {
                        Ok(()) => {
                            session.ring.lock().clear();
                            *session.resample.lock() = ResampleState::default();
                            let _ = req.reply.send(SeekReply::Ok);
                        }
                        Err(_) => {
                            let _ = req.reply.send(SeekReply::Failed);
                        }
                    }
                }
            }

            match decoder.next_samples() {
                Ok(Some(mut samples)) => {
                    let rg = f32::from_bits(session.replay_gain_factor.load(Ordering::Relaxed));
                    if rg != 1.0 {
                        for s in &mut samples {
                            *s *= rg;
                        }
                    }

                    if !bit_perfect_strict {
                        let gen = eq.generation();
                        if gen != eq_gen {
                            eq_gen = gen;
                            let cfg = eq.snapshot();
                            eq_chain = if cfg.enabled && !eq::bands_are_flat(&cfg.bands) {
                                EqChain::new(&cfg.bands, sample_rate, channels)
                            } else {
                                None
                            };
                        }
                        if let Some(chain) = eq_chain.as_mut() {
                            chain.process_interleaved(&mut samples);
                        }
                    }

                    visualizer::process_chunk(&samples, channels as u16, sample_rate, &visualizer);

                    if fade_done < fade_total {
                        for s in &mut samples {
                            if fade_done >= fade_total {
                                break;
                            }
                            *s *= fade_done as f32 / fade_total as f32;
                            fade_done += 1;
                        }
                    }

                    session.ring.lock().extend(samples);
                }
                Ok(None) => {
                    session.finished_decoding.store(true, Ordering::Relaxed);
                    return;
                }
                Err(e) => {
                    eprintln!("Decode error for track {}: {e}", session.track_id);
                    session.finished_decoding.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }
    });
}

/// Spawn a finish-watcher task for `player_id`.
///
/// Polls every 100ms; emits "playback-position" every ~300ms while playing,
/// and "playback-finished" (removing the session) once the session drains or
/// is removed externally (e.g. stop()).
pub fn spawn_finish_watcher(sessions: SessionMap, app_handle: AppHandle, player_id: PlayerId) {
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
                        let loading = session.loading.load(Ordering::Relaxed);
                        let finished = !loading && session.is_empty();
                        let pos_payload = if tick.is_multiple_of(3) && !loading && session.playback_start_time.lock().is_some() {
                            let pos = *session.accumulated_time.lock()
                                + session.playback_start_time.lock()
                                    .map(|t| t.elapsed().as_secs_f64())
                                    .unwrap_or(0.0);
                            Some(serde_json::json!({
                                "playerId": player_id,
                                "position": pos,
                                "duration": *session.duration.lock()
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
