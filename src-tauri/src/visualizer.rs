//! Real-time audio visualizer support.
//!
//! `process_chunk` is called inline from the decode-feeder (see
//! `audio::session::spawn_decode_feeder`) for every decoded chunk of
//! interleaved samples. It downmixes whatever is playing to mono and pushes
//! samples into a shared ring buffer. A background task periodically reads
//! that buffer, runs an FFT, and emits `firmium:audio-analysis` events
//! ({ bass, bars }) for the frontend to render.
//!
//! `process_chunk` is always called from the decode chain but only writes
//! samples (and the analysis task only runs its FFT) while `enabled` is
//! true, so there's no overhead when the visualizer panel is closed.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use tauri::{AppHandle, Emitter};

/// Number of mono samples kept for analysis. Must be >= FFT_SIZE.
const BUFFER_CAPACITY: usize = 4096;
/// FFT window size (power of two).
const FFT_SIZE: usize = 1024;
/// Number of frequency bars emitted to the frontend.
const BAR_COUNT: usize = 24;
/// Analysis cadence.
const ANALYSIS_INTERVAL: Duration = Duration::from_millis(50);

/// Shared state between the playback decode chain (writer) and the
/// analysis task (reader).
pub struct VisualizerState {
    enabled: AtomicBool,
    sample_rate: AtomicU32,
    buffer: Mutex<VecDeque<f32>>,
}

impl VisualizerState {
    pub fn new() -> Self {
        VisualizerState {
            enabled: AtomicBool::new(false),
            sample_rate: AtomicU32::new(44100),
            buffer: Mutex::new(VecDeque::with_capacity(BUFFER_CAPACITY)),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            self.buffer.lock().clear();
        }
    }

    fn push_sample(&self, sample: f32, sample_rate: u32) {
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
        let mut buf = self.buffer.lock();
        if buf.len() >= BUFFER_CAPACITY {
            buf.pop_front();
        }
        buf.push_back(sample);
    }
}

/// Downmix an interleaved chunk of samples to mono and feed `VisualizerState`.
///
/// Called inline from the decode-feeder for every decoded chunk. No-op
/// (besides the atomic load) while the visualizer is disabled.
pub fn process_chunk(samples: &[f32], channels: u16, sample_rate: u32, state: &VisualizerState) {
    if !state.enabled.load(Ordering::Relaxed) {
        return;
    }
    let channels = channels.max(1) as usize;
    for frame in samples.chunks(channels) {
        let sum: f32 = frame.iter().sum();
        state.push_sample(sum / channels as f32, sample_rate);
    }
}

/// Spawn the background analysis task. Runs for the lifetime of the app;
/// idles (no FFT work) whenever `state.enabled` is false.
pub fn spawn_analysis_task(app_handle: AppHandle, state: Arc<VisualizerState>) {
    tauri::async_runtime::spawn(async move {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        loop {
            tokio::time::sleep(ANALYSIS_INTERVAL).await;
            if !state.enabled.load(Ordering::Relaxed) {
                continue;
            }

            let samples: Vec<f32> = {
                let buf = state.buffer.lock();
                if buf.len() < FFT_SIZE {
                    continue;
                }
                buf.iter().rev().take(FFT_SIZE).copied().collect()
            };

            // Hann window to reduce spectral leakage.
            let mut spectrum: Vec<Complex32> = samples
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0)).cos();
                    Complex32::new(s * w, 0.0)
                })
                .collect();

            fft.process(&mut spectrum);

            let magnitudes: Vec<f32> = spectrum[..FFT_SIZE / 2].iter().map(|c| c.norm()).collect();

            let sample_rate = state.sample_rate.load(Ordering::Relaxed).max(1) as f32;
            let bin_hz = sample_rate / FFT_SIZE as f32;
            let bass_bins = ((250.0 / bin_hz) as usize).clamp(1, magnitudes.len());
            let bass: f32 = magnitudes[..bass_bins].iter().sum::<f32>() / bass_bins as f32;
            let bass_norm = (bass / (FFT_SIZE as f32 / 8.0)).min(1.0);

            let bars = compute_bars(&magnitudes);

            let _ = app_handle.emit("firmium:audio-analysis", serde_json::json!({
                "bass": bass_norm,
                "bars": bars,
            }));
        }
    });
}

/// Groups the magnitude spectrum into `BAR_COUNT` log-spaced bars, normalized to 0..1.
fn compute_bars(magnitudes: &[f32]) -> Vec<f32> {
    let n = magnitudes.len();
    let log_n = (n as f32).ln();
    let mut bars = Vec::with_capacity(BAR_COUNT);

    for i in 0..BAR_COUNT {
        let lo = ((i as f32 / BAR_COUNT as f32) * log_n).exp() as usize;
        let hi = (((i + 1) as f32 / BAR_COUNT as f32) * log_n).exp().ceil() as usize;
        let lo = lo.min(n.saturating_sub(1));
        let hi = hi.clamp(lo + 1, n);

        let slice = &magnitudes[lo..hi];
        let avg = slice.iter().sum::<f32>() / slice.len() as f32;
        bars.push((avg / (FFT_SIZE as f32 / 16.0)).min(1.0));
    }

    bars
}
