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

/// Below this, monstercat's exponential spread inverts (`strength * 1.5 < 1.0`
/// grows outward instead of decaying) — snap to disabled instead.
const MONSTERCAT_MIN_EFFECTIVE: f32 = 0.7;

pub struct VisualizerState {
    pub(crate) enabled: AtomicBool,
    /// Monstercat spread intensity; 0.0 = disabled. Mutually exclusive with
    /// `waves` (enabling one via `set_monstercat`/`set_waves` disables the
    /// other, enforced by the caller in `update/transport.rs`).
    monstercat: Mutex<f32>,
    waves: AtomicBool,
    waves_smoothing: AtomicU32,
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

impl Default for VisualizerState {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualizerState {
    pub fn new() -> Self {
        VisualizerState {
            enabled: AtomicBool::new(false),
            monstercat: Mutex::new(1.0),
            waves: AtomicBool::new(false),
            waves_smoothing: AtomicU32::new(5),
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

    /// Set Monstercat-style bar smoothing intensity (spatial spread across
    /// neighboring bars, cava-style). Lower intensity spreads energy wider
    /// (smoother, blobbier); higher intensity decays faster (sharper,
    /// narrower peaks). Values below [`MONSTERCAT_MIN_EFFECTIVE`] snap to 0.0
    /// (disabled) since the spread math inverts below that threshold.
    /// Mutually exclusive with `waves` — the caller is responsible for
    /// disabling the other mode.
    pub fn set_monstercat(&self, intensity: f32) {
        let intensity = if intensity < MONSTERCAT_MIN_EFFECTIVE { 0.0 } else { intensity };
        *self.monstercat.lock() = intensity;
    }

    /// Toggle Waves-style bar smoothing (Catmull-Rom spline across sparse
    /// control points spaced `smoothing` bars apart, clamped 2-16). Mutually
    /// exclusive with `monstercat` — the caller is responsible for disabling
    /// the other mode.
    pub fn set_waves(&self, enabled: bool, smoothing: u32) {
        self.waves.store(enabled, Ordering::Relaxed);
        self.waves_smoothing.store(smoothing.clamp(2, 16), Ordering::Relaxed);
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

            let mut raw_bars = compute_bars(&magnitudes, sample_rate);
            if state.waves.load(Ordering::Relaxed) {
                waves_smooth(&mut raw_bars, state.waves_smoothing.load(Ordering::Relaxed) as usize);
            } else {
                let intensity = *state.monstercat.lock();
                if intensity > 0.0 {
                    monstercat_filter(&mut raw_bars, intensity);
                }
            }

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

/// Monstercat-style bar smoothing: spreads each bar's energy into its
/// neighbors via exponential decay (`intensity` sets the decay base, so lower
/// = wider/smoother spread, higher = sharper/narrower peaks), then a light
/// Catmull-Rom pass smooths the kinks where overlapping decays meet. Caller
/// must ensure `intensity` is either 0.0 or >= [`MONSTERCAT_MIN_EFFECTIVE`]
/// (see `set_monstercat`).
#[allow(clippy::needless_range_loop)]
fn monstercat_filter(bars: &mut [f32], intensity: f32) {
    let n = bars.len();
    if n == 0 {
        return;
    }

    for z in 0..n {
        let bar_value = bars[z];

        for m_y in (0..z).rev() {
            let de = (z - m_y) as f32;
            let spread = bar_value / (intensity * 1.5).powf(de);
            if spread > bars[m_y] {
                bars[m_y] = spread;
            }
        }

        for m_y in (z + 1)..n {
            let de = (m_y - z) as f32;
            let spread = bar_value / (intensity * 1.5).powf(de);
            if spread > bars[m_y] {
                bars[m_y] = spread;
            }
        }
    }

    waves_smooth(bars, 2);
}

/// Catmull-Rom spline interpolation for a single dimension.
fn catmull_rom_1d(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Subsamples bars into sparse control points every `step` bars, then
/// interpolates a smooth C1-continuous curve back over the full bar count.
fn waves_smooth(bars: &mut [f32], step: usize) {
    let n = bars.len();
    if n < 4 {
        return;
    }
    let step = step.clamp(2, 16);

    let mut control_points: Vec<f32> = Vec::new();
    let mut i = 0;
    while i < n {
        control_points.push(bars[i]);
        i += step;
    }
    if !(n - 1).is_multiple_of(step) {
        control_points.push(bars[n - 1]);
    }

    let cp_count = control_points.len();
    if cp_count < 2 {
        return;
    }

    let last_cp = (cp_count - 1) as f32;
    for (i, bar) in bars.iter_mut().enumerate() {
        let pos = i as f32 / (n - 1).max(1) as f32 * last_cp;
        let segment = (pos.floor() as usize).min(cp_count - 2);
        let t = pos - segment as f32;

        let p0 = control_points[segment.saturating_sub(1)];
        let p1 = control_points[segment];
        let p2 = control_points[(segment + 1).min(cp_count - 1)];
        let p3 = control_points[(segment + 2).min(cp_count - 1)];

        *bar = catmull_rom_1d(p0, p1, p2, p3, t).clamp(0.0, 1.0);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monstercat_filter_spreads_energy_into_neighbors() {
        let mut bars = vec![0.0f32; 16];
        bars[8] = 1.0;
        monstercat_filter(&mut bars, 1.0);
        assert!(bars[7] > 0.0 && bars[7] < 1.0);
        assert!(bars[9] > 0.0 && bars[9] < 1.0);
        assert!(bars[0] < bars[7]);
    }

    #[test]
    fn monstercat_filter_stays_in_unit_range() {
        let mut bars = vec![0.0, 0.3, 1.0, 0.2, 0.0, 0.8, 0.1, 0.0];
        monstercat_filter(&mut bars, 1.0);
        for &b in &bars {
            assert!((0.0..=1.0).contains(&b));
        }
    }

    #[test]
    fn monstercat_filter_no_panic_on_short_input() {
        let mut bars = vec![0.5, 0.5];
        monstercat_filter(&mut bars, 1.0);
        let mut empty: Vec<f32> = vec![];
        monstercat_filter(&mut empty, 1.0);
    }

    #[test]
    fn monstercat_filter_lower_intensity_spreads_wider() {
        let mut wide = vec![0.0f32; 16];
        wide[8] = 1.0;
        monstercat_filter(&mut wide, 0.7);

        let mut narrow = vec![0.0f32; 16];
        narrow[8] = 1.0;
        monstercat_filter(&mut narrow, 5.0);

        assert!(wide[0] > narrow[0]);
    }

    #[test]
    fn waves_smooth_stays_in_unit_range() {
        let mut bars = vec![0.0, 0.3, 1.0, 0.2, 0.0, 0.8, 0.1, 0.0];
        waves_smooth(&mut bars, 5);
        for &b in &bars {
            assert!((0.0..=1.0).contains(&b));
        }
    }

    #[test]
    fn waves_smooth_no_panic_on_short_input() {
        let mut bars = vec![0.5, 0.5];
        waves_smooth(&mut bars, 5);
        let mut empty: Vec<f32> = vec![];
        waves_smooth(&mut empty, 5);
    }

    #[test]
    fn waves_smooth_clamps_step() {
        let mut bars = vec![0.0, 0.3, 1.0, 0.2, 0.0, 0.8, 0.1, 0.0];
        // step=0 clamps to 2 internally — must not panic or divide by zero.
        waves_smooth(&mut bars, 0);
        waves_smooth(&mut bars, 999);
    }
}
