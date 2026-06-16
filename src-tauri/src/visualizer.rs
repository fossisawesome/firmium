//! Real-time audio visualizer support.
//!
//! `process_chunk` is called inline from the decode-feeder for every decoded
//! chunk of interleaved samples. It downmixes to mono and pushes samples into
//! a shared ring buffer. A background task periodically reads that buffer,
//! runs an FFT, applies per-bar smoothing, and emits `firmium:audio-analysis`
//! events ({ bass, bars }) for the frontend to render via WebGL.
//!
//! The analysis task idles (no FFT work) whenever `enabled` is false, so
//! there's no overhead when the visualizer panel is closed.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use tauri::{AppHandle, Emitter};

const BUFFER_CAPACITY: usize = 8192;
const FFT_SIZE: usize = 2048;
const BAR_COUNT: usize = 32;
const ANALYSIS_INTERVAL: Duration = Duration::from_millis(16);

pub struct VisualizerState {
    enabled: AtomicBool,
    sample_rate: AtomicU32,
    buffer: Mutex<VecDeque<f32>>,
    smooth_bars: Mutex<Vec<f32>>,
    smooth_bass: Mutex<f32>,
}

impl VisualizerState {
    pub fn new() -> Self {
        VisualizerState {
            enabled: AtomicBool::new(false),
            sample_rate: AtomicU32::new(44100),
            buffer: Mutex::new(VecDeque::with_capacity(BUFFER_CAPACITY)),
            smooth_bars: Mutex::new(vec![0.0; BAR_COUNT]),
            smooth_bass: Mutex::new(0.0),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            self.buffer.lock().clear();
            *self.smooth_bars.lock() = vec![0.0; BAR_COUNT];
            *self.smooth_bass.lock() = 0.0;
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

            // Bass: max magnitude in 40–250 Hz, skip DC and subsonic
            let bass_lo = ((40.0 / bin_hz) as usize).max(1).min(magnitudes.len() - 1);
            let bass_hi = ((250.0 / bin_hz) as usize + 1).min(magnitudes.len());
            let raw_bass = magnitudes[bass_lo..bass_hi].iter().cloned().fold(0.0f32, f32::max);
            let raw_bass_norm = (raw_bass / (FFT_SIZE as f32 / 32.0)).min(1.0);

            let mut sb = state.smooth_bass.lock();
            let alpha = if raw_bass_norm > *sb { 0.6 } else { 0.15 };
            *sb += alpha * (raw_bass_norm - *sb);
            let bass_out = *sb;
            drop(sb);

            let raw_bars = compute_bars(&magnitudes, sample_rate);

            let mut smooth = state.smooth_bars.lock();
            for (s, &raw) in smooth.iter_mut().zip(raw_bars.iter()) {
                let a = if raw > *s { 0.6_f32 } else { 0.15_f32 };
                *s += a * (raw - *s);
            }
            let bars_out: Vec<f32> = smooth.clone();
            drop(smooth);

            let _ = app_handle.emit("firmium:audio-analysis", serde_json::json!({
                "bass": bass_out,
                "bars": bars_out,
            }));
        }
    });
}

/// Maps frequency magnitudes into BAR_COUNT log-spaced bars from 40 Hz to 18 kHz.
/// Uses the max bin within each band (not avg) for a more reactive display.
fn compute_bars(magnitudes: &[f32], sample_rate: f32) -> Vec<f32> {
    const F_MIN: f32 = 40.0;
    const F_MAX: f32 = 18000.0;
    let bin_hz = sample_rate / FFT_SIZE as f32;
    let log_min = F_MIN.ln();
    let log_max = F_MAX.ln();
    let n = magnitudes.len();

    (0..BAR_COUNT)
        .map(|i| {
            let lo_hz = (log_min + (i as f32 / BAR_COUNT as f32) * (log_max - log_min)).exp();
            let hi_hz = (log_min + ((i + 1) as f32 / BAR_COUNT as f32) * (log_max - log_min)).exp();
            let lo_bin = ((lo_hz / bin_hz) as usize).min(n.saturating_sub(1));
            let hi_bin = ((hi_hz / bin_hz) as usize + 1).clamp(lo_bin + 1, n);
            let peak = magnitudes[lo_bin..hi_bin].iter().cloned().fold(0.0f32, f32::max);
            (peak / (FFT_SIZE as f32 / 32.0)).min(1.0)
        })
        .collect()
}
