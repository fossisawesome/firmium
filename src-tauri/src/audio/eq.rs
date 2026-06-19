//! Hand-rolled biquad IIR equalizer applied in the decode loop after ReplayGain.
//!
//! Coefficients use the RBJ "Audio EQ Cookbook" formulas. Low/high ends are
//! shelving filters; middle bands are peaking filters. Each band runs an
//! independent biquad per channel (filter state must not be shared across
//! interleaved channels), so an `EqChain` holds `bands × channels` biquads.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

/// Filter shape for a band.
#[derive(Clone, Copy, PartialEq)]
pub enum BandKind {
    LowShelf,
    Peaking,
    HighShelf,
}

/// A single EQ band: center/corner frequency, gain in dB, and Q.
#[derive(Clone, Copy)]
pub struct EqBand {
    pub kind: BandKind,
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
}

/// One biquad section (transposed direct form II).
#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// Identity (passthrough) section.
    fn identity() -> Self {
        Biquad { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 }
    }

    /// Build normalized coefficients from raw (unnormalized) cookbook values.
    fn from_raw(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Biquad {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn from_band(band: &EqBand, sample_rate: f32) -> Self {
        // Degenerate frequency or flat gain → passthrough.
        if band.freq <= 0.0 || band.freq >= sample_rate * 0.5 {
            return Biquad::identity();
        }
        let a = 10f32.powf(band.gain_db / 40.0); // sqrt of linear gain
        let w0 = 2.0 * PI * band.freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let q = band.q.max(0.05);
        let alpha = sin_w0 / (2.0 * q);

        match band.kind {
            BandKind::Peaking => Biquad::from_raw(
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            BandKind::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                Biquad::from_raw(
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            BandKind::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                Biquad::from_raw(
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// A chain of biquad bands, with independent filter state per channel.
pub struct EqChain {
    /// `sections[channel][band]`
    sections: Vec<Vec<Biquad>>,
    channels: usize,
}

impl EqChain {
    /// Build a chain for `bands` at `sample_rate`, with `channels` independent
    /// state copies. Returns `None` if there's nothing to do (no bands).
    pub fn new(bands: &[EqBand], sample_rate: u32, channels: usize) -> Option<EqChain> {
        if bands.is_empty() || channels == 0 {
            return None;
        }
        let template: Vec<Biquad> = bands
            .iter()
            .map(|b| Biquad::from_band(b, sample_rate as f32))
            .collect();
        let sections = (0..channels).map(|_| template.clone()).collect();
        Some(EqChain { sections, channels })
    }

    /// Apply the chain in place over interleaved f32 samples.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for (i, sample) in samples.iter_mut().enumerate() {
            let ch = i % self.channels;
            let mut v = *sample;
            for biquad in &mut self.sections[ch] {
                v = biquad.process(v);
            }
            *sample = v;
        }
    }
}

/// True if every band is flat (no audible effect), letting callers skip the chain.
pub fn bands_are_flat(bands: &[EqBand]) -> bool {
    bands.iter().all(|b| b.gain_db.abs() < 0.01)
}

/// The currently-active EQ bands plus the master enable flag.
#[derive(Clone, Default)]
pub struct EqRuntimeConfig {
    pub enabled: bool,
    pub bands: Vec<EqBand>,
}

/// Shared, live-updatable EQ state. The decode feeder reads `generation`
/// (cheap atomic) every chunk and only re-locks `config` to rebuild its chain
/// when the generation changes. Mirrors the replay-gain live-update pattern.
pub struct EqShared {
    generation: AtomicU64,
    config: Mutex<EqRuntimeConfig>,
}

impl EqShared {
    pub fn new(config: EqRuntimeConfig) -> Self {
        EqShared {
            generation: AtomicU64::new(1),
            config: Mutex::new(config),
        }
    }

    /// Replace the active config and bump the generation so running feeders rebuild.
    pub fn set(&self, enabled: bool, bands: Vec<EqBand>) {
        *self.config.lock() = EqRuntimeConfig { enabled, bands };
        self.generation.fetch_add(1, Ordering::Release);
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Snapshot the current config (used by a feeder to rebuild its chain).
    pub fn snapshot(&self) -> EqRuntimeConfig {
        self.config.lock().clone()
    }
}
