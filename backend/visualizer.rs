//! Real-time audio visualizer support.
//!
//! `process_chunk` is called inline from the decode-feeder for every decoded
//! chunk of interleaved samples. It downmixes to mono and pushes samples into
//! a shared ring buffer. A background task periodically reads that buffer,
//! runs an FFT, applies per-bar smoothing, and stores the latest bar/wave/band
//! results. The iced shader visualizer reads those in-process via the getters.
//!
//! The analysis task idles whenever `enabled` is false, so there's no overhead
//! when the visualizer panel is closed.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;

const BUFFER_CAPACITY: usize = 8192;
const FFT_SIZE: usize = 2048;
pub const BAR_COUNT: usize = 120;
// Scope waveform: same count as bars so scope.wgsl can use the bar_buffer.
pub const WAVE_POINTS: usize = 120;
const ANALYSIS_INTERVAL: Duration = Duration::from_millis(16);

pub struct VisualizerState {
    pub(crate) enabled: AtomicBool,
    sample_rate: AtomicU32,
    buffer: Mutex<VecDeque<f32>>,
    smooth_bars: Mutex<Vec<f32>>,
    smooth_bass: Mutex<f32>,
    smooth_mid: Mutex<f32>,
    smooth_treble: Mutex<f32>,
    last_bass: Mutex<f32>,
    last_bars: Mutex<Vec<f32>>,
    last_wave: Mutex<Vec<f32>>,
    last_mid: Mutex<f32>,
    last_treble: Mutex<f32>,
    /// Set true after each analysis frame; cleared by the renderer once it
    /// has consumed the data. Prevents redundant GPU uploads on idle frames.
    dirty: AtomicBool,
}

impl VisualizerState {
    pub fn new() -> Self {
        VisualizerState {
            enabled: AtomicBool::new(false),
            sample_rate: AtomicU32::new(44100),
            buffer: Mutex::new(VecDeque::with_capacity(BUFFER_CAPACITY)),
            smooth_bars: Mutex::new(vec![0.0; BAR_COUNT]),
            smooth_bass: Mutex::new(0.0),
            smooth_mid: Mutex::new(0.0),
            smooth_treble: Mutex::new(0.0),
            last_bass: Mutex::new(0.0),
            last_bars: Mutex::new(vec![0.0; BAR_COUNT]),
            last_wave: Mutex::new(vec![0.0; WAVE_POINTS]),
            last_mid: Mutex::new(0.0),
            last_treble: Mutex::new(0.0),
            dirty: AtomicBool::new(false),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            self.buffer.lock().clear();
            *self.smooth_bars.lock() = vec![0.0; BAR_COUNT];
            *self.smooth_bass.lock() = 0.0;
            *self.smooth_mid.lock() = 0.0;
            *self.smooth_treble.lock() = 0.0;
            *self.last_bass.lock() = 0.0;
            *self.last_bars.lock() = vec![0.0; BAR_COUNT];
            *self.last_wave.lock() = vec![0.0; WAVE_POINTS];
            *self.last_mid.lock() = 0.0;
            *self.last_treble.lock() = 0.0;
            self.dirty.store(false, Ordering::Relaxed);
        }
    }

    // --- Getters used by the shader widget ---

    pub fn bars(&self) -> Vec<f32> {
        self.last_bars.lock().clone()
    }

    pub fn wave(&self) -> Vec<f32> {
        self.last_wave.lock().clone()
    }

    pub fn bass(&self) -> f32 {
        *self.last_bass.lock()
    }

    pub fn mid(&self) -> f32 {
        *self.last_mid.lock()
    }

    pub fn treble(&self) -> f32 {
        *self.last_treble.lock()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn clear_dirty(&self) {
        self.dirty.store(false, Ordering::Relaxed);
    }

    /// Legacy snapshot for any remaining canvas-based callers.
    #[allow(dead_code)]
    pub fn snapshot(&self) -> (f32, Vec<f32>, Vec<f32>) {
        (self.bass(), self.bars(), self.wave())
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

pub fn spawn_analysis_task(state: Arc<VisualizerState>) {
    tokio::spawn(async move {
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
                    let w = 0.5
                        - 0.5
                            * (2.0 * std::f32::consts::PI * i as f32
                                / (FFT_SIZE as f32 - 1.0))
                                .cos();
                    Complex32::new(s * w, 0.0)
                })
                .collect();

            fft.process(&mut spectrum);

            let magnitudes: Vec<f32> =
                spectrum[..FFT_SIZE / 2].iter().map(|c| c.norm()).collect();
            let sample_rate = state.sample_rate.load(Ordering::Relaxed).max(1) as f32;
            let bin_hz = sample_rate / FFT_SIZE as f32;

            let norm = FFT_SIZE as f32 / 32.0;

            let band_energy = |lo_hz: f32, hi_hz: f32| -> f32 {
                let lo = ((lo_hz / bin_hz) as usize).max(1).min(magnitudes.len() - 1);
                let hi = ((hi_hz / bin_hz) as usize + 1).min(magnitudes.len());
                magnitudes[lo..hi].iter().cloned().fold(0.0f32, f32::max) / norm
            };

            let raw_bass = band_energy(40.0, 250.0).min(1.0);
            let raw_mid = band_energy(250.0, 4000.0).min(1.0);
            let raw_treble = band_energy(4000.0, 18000.0).min(1.0);

            let smooth = |current: &mut f32, raw: f32| {
                let alpha = if raw > *current { 0.6 } else { 0.15 };
                *current += alpha * (raw - *current);
            };

            let bass_out = {
                let mut sb = state.smooth_bass.lock();
                smooth(&mut sb, raw_bass);
                *sb
            };
            let mid_out = {
                let mut sm = state.smooth_mid.lock();
                smooth(&mut sm, raw_mid);
                *sm
            };
            let treble_out = {
                let mut st = state.smooth_treble.lock();
                smooth(&mut st, raw_treble);
                *st
            };

            let raw_bars = compute_bars(&magnitudes, sample_rate);

            let bars_out: Vec<f32> = {
                let mut smooth_b = state.smooth_bars.lock();
                for (s, &raw) in smooth_b.iter_mut().zip(raw_bars.iter()) {
                    let a = if raw > *s { 0.6_f32 } else { 0.15_f32 };
                    *s += a * (raw - *s);
                }
                smooth_b.clone()
            };

            // Scope waveform: downsample the latest samples (chronological order)
            // to WAVE_POINTS. samples[0] is oldest (FFT_SIZE-1 ago), samples is
            // newest-first (rev().take()), so reverse for chronological.
            let step = (FFT_SIZE / WAVE_POINTS).max(1);
            let wave_out: Vec<f32> = (0..WAVE_POINTS)
                .map(|i| {
                    // samples[0] = newest, samples[FFT_SIZE-1] = oldest
                    // we want chronological: oldest first
                    let newest_first_idx = FFT_SIZE - 1 - i * step;
                    samples[newest_first_idx.min(FFT_SIZE - 1)].clamp(-1.0, 1.0)
                })
                .collect();

            *state.last_bass.lock() = bass_out;
            *state.last_mid.lock() = mid_out;
            *state.last_treble.lock() = treble_out;
            *state.last_bars.lock() = bars_out;
            *state.last_wave.lock() = wave_out;
            state.dirty.store(true, Ordering::Relaxed);
        }
    });
}

/// Maps frequency magnitudes into BAR_COUNT log-spaced bars from 40 Hz to 18 kHz.
fn compute_bars(magnitudes: &[f32], sample_rate: f32) -> Vec<f32> {
    const F_MIN: f32 = 40.0;
    const F_MAX: f32 = 18000.0;
    let bin_hz = sample_rate / FFT_SIZE as f32;
    let log_min = F_MIN.ln();
    let log_max = F_MAX.ln();
    let n = magnitudes.len();

    (0..BAR_COUNT)
        .map(|i| {
            let lo_hz =
                (log_min + (i as f32 / BAR_COUNT as f32) * (log_max - log_min)).exp();
            let hi_hz =
                (log_min + ((i + 1) as f32 / BAR_COUNT as f32) * (log_max - log_min))
                    .exp();
            let lo_bin = ((lo_hz / bin_hz) as usize).min(n.saturating_sub(1));
            let hi_bin = ((hi_hz / bin_hz) as usize + 1).clamp(lo_bin + 1, n);
            let peak = magnitudes[lo_bin..hi_bin]
                .iter()
                .cloned()
                .fold(0.0f32, f32::max);
            (peak / (FFT_SIZE as f32 / 32.0)).min(1.0)
        })
        .collect()
}
