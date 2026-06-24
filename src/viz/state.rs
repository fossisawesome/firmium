use std::sync::Arc;

use crate::visualizer::VisualizerState as BackendState;

use super::particles::ParticleSystem;

const BAR_COUNT: usize = crate::visualizer::BAR_COUNT;
const PARTICLE_COUNT: usize = 256;

pub struct VizState {
    pub backend: Arc<BackendState>,
    pub peak_bars: Vec<f32>,
    pub peak_vel: Vec<f32>,
    pub peak_hold: Vec<f32>,
    pub peak_alphas: Vec<f32>,
    pub flash_intensities: Vec<f32>,
    pub prev_bars: Vec<f32>,
    pub beat_pulse: f32,
    pub beat_max: f32,
    pub particles: ParticleSystem,
}

impl VizState {
    pub fn new(backend: Arc<BackendState>) -> Self {
        Self {
            particles: ParticleSystem::new(PARTICLE_COUNT, 0.55, 12345),
            backend,
            peak_bars: vec![0.0; BAR_COUNT],
            peak_vel: vec![0.0; BAR_COUNT],
            peak_hold: vec![0.0; BAR_COUNT],
            peak_alphas: vec![1.0; BAR_COUNT],
            flash_intensities: vec![0.0; BAR_COUNT],
            prev_bars: vec![0.0; BAR_COUNT],
            beat_pulse: 0.0,
            beat_max: 0.001,
        }
    }

    /// Advance peak tracking, flash decay, beat detection, and particles by one
    /// frame (~16 ms at 60 Hz). Call from `shader::Program::update()`.
    pub fn tick(
        &mut self,
        peak_hold_time: f32,
        peak_fade_time: f32,
        scope_radius: f32,
        scope_sensitivity: f32,
        scope_particles: bool,
    ) {
        let bars = self.backend.bars();
        let bass = self.backend.bass();

        // Beat detection: normalized bass drives a smoothed pulse.
        self.beat_max = self.beat_max * 0.995 + bass * 0.005;
        let norm_bass = (bass / self.beat_max.max(0.001)).min(1.0);
        self.beat_pulse = self.beat_pulse * 0.85 + norm_bass * 0.15;

        let bar_count = bars.len().min(BAR_COUNT);

        // Flash: detect onset (bar rose significantly), decay at ~0.88/frame.
        for (i, &bar) in bars.iter().enumerate().take(bar_count) {
            let onset = (bar - self.prev_bars[i]).max(0.0);
            if onset > 0.15 {
                self.flash_intensities[i] =
                    (self.flash_intensities[i] + onset * 2.0).min(1.0);
            }
            self.flash_intensities[i] *= 0.88;
        }

        // Peak tracking.
        let hold_frames = (peak_hold_time * 60.0).max(1.0);
        let fade_rate = if peak_fade_time > 0.001 {
            1.0 / (peak_fade_time * 60.0)
        } else {
            1.0
        };
        for (i, &v) in bars.iter().enumerate().take(bar_count) {
            if v >= self.peak_bars[i] {
                self.peak_bars[i] = v;
                self.peak_hold[i] = hold_frames;
                self.peak_vel[i] = 0.0;
                self.peak_alphas[i] = 1.0;
            } else if self.peak_hold[i] > 0.0 {
                self.peak_hold[i] -= 1.0;
            } else {
                self.peak_vel[i] += 0.002;
                self.peak_bars[i] = (self.peak_bars[i] - self.peak_vel[i]).max(0.0);
                self.peak_alphas[i] = (self.peak_alphas[i] - fade_rate).max(0.0);
            }
        }

        self.prev_bars[..bar_count].copy_from_slice(&bars[..bar_count]);

        if scope_particles {
            let wave = self.backend.wave();
            let energy = (bass + self.backend.mid()) * 0.5;
            self.particles.update(
                scope_radius,
                energy,
                self.beat_pulse,
                1.0,
                &wave,
                scope_sensitivity,
            );
        }
    }

    pub fn get_bars(&self) -> Vec<f32> {
        self.backend.bars()
    }

    pub fn get_waveform(&self) -> Vec<f32> {
        self.backend.wave()
    }

    pub fn get_peak_bars(&self) -> Vec<f32> {
        self.peak_bars.clone()
    }

    pub fn get_peak_alphas(&self) -> Vec<f32> {
        self.peak_alphas.clone()
    }

    pub fn get_flash_intensities(&self) -> Vec<f32> {
        self.flash_intensities.clone()
    }

    #[allow(dead_code)]
    pub fn bar_count(&self) -> usize {
        self.backend.bars().len()
    }

    pub fn current_bands(&self) -> (f32, f32, f32) {
        (self.backend.bass(), self.backend.mid(), self.backend.treble())
    }

    pub fn current_beat_pulse(&self) -> f32 {
        self.beat_pulse
    }

    pub fn is_dirty(&self) -> bool {
        self.backend.is_dirty()
    }

    #[allow(dead_code)]
    pub fn clear_dirty(&self) {
        self.backend.clear_dirty();
    }

    pub fn get_particles(&self) -> Vec<[f32; 8]> {
        self.particles.gpu_data().to_vec()
    }

    /// Returns true when motion trails / echo are still fading after audio
    /// stopped (so the renderer keeps running until the trail clears).
    pub fn trail_draining(&self) -> bool {
        self.flash_intensities.iter().any(|&v| v > 0.005)
            || self.beat_pulse > 0.005
    }
}
