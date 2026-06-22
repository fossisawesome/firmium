//! CPU particle simulation for the Scope visualizer's particle field.
//!
//! A pool of particles is spawned at the oscilloscope ring and drifts outward,
//! but instead of dead-straight radial drift each particle is also pushed by a
//! time-evolving **curl-noise flow field** (the organic swirl that separates
//! "particles" from a tech demo — NCS's fractal-displacement idea, done cheaply
//! on the CPU). Particles decelerate, fade, and recycle; audio energy + beats
//! scale launch speed and size. Each carries a gradient colour and a twinkle
//! phase. Positions are resolution-independent normalized "ring space"
//! (radius 1.0 = half the panel); the GPU does the pixel mapping.
//!
//! GPU snapshot: two `vec4`s per particle — (x, y, size, alpha) and
//! (colour_t, _, _, _) — see `particles.wgsl`.

use std::f32::consts::TAU;

// --- Tuning constants (normalized ring-space, per 60 Hz frame) ---
const BASE_SIZE: f32 = 0.016; // particle radius as a fraction of half-panel
const BASE_SPEED: f32 = 0.0032; // idle outward drift per frame
const ENERGY_SPEED: f32 = 0.010; // extra launch speed scaled by onset energy
const BEAT_BURST: f32 = 0.012; // extra launch speed on a beat pulse
const DAMPING: f32 = 0.985; // per-frame velocity decay (embers slow as they fly)
const TANGENTIAL: f32 = 0.0016; // sideways drift at launch (swirl)
// The panel is SQUARE (1.0 = half-side), so its corners are at radius √2 ≈ 1.41.
// Particles fill the whole square (corners included), recycling only once they
// drift past the panel edge + a margin — not at an inscribed circle.
const PANEL_RECYCLE: f32 = 1.3; // recycle once |x| or |y| exceeds this
const INNER_MARGIN: f32 = 0.05; // keep the album-art face inside the ring clear (spawn from ~the ring out)
const MIN_LIFE: f32 = 35.0; // shortest lifetime in frames (~0.6 s)
const MAX_LIFE: f32 = 95.0; // longest lifetime in frames (~1.6 s)
const FADE_IN: f32 = 0.14; // fraction of life spent fading in
const FADE_OUT: f32 = 0.55; // remaining-life fraction over which it fades out

// Curl-noise flow field.
const CURL_FREQ: f32 = 2.2; // spatial scale of the swirl (cells across the field)
const CURL_STRENGTH: f32 = 0.0013; // per-frame velocity nudge from the flow
const CURL_TIME: f32 = 0.012; // how fast the flow field evolves per frame

// Twinkle (per-particle brightness shimmer).
const TWINKLE_RATE: f32 = 0.22; // radians per frame
const TWINKLE_DEPTH: f32 = 0.22; // 0 = steady, up to ~0.5

// Crest-spark emission: a fraction of respawns launch from the LOUD arcs of the
// ring (importance-sampled off the waveform) so sparks fly off the wave where
// the audio swings hardest. The remaining respawns still fill the whole square
// (corners included), so this is ADDITIVE to the ambient dust, not a swap — the
// field still fills the panel the way the corner-fill pass intends.
const CREST_SPARK_BASE: f32 = 0.16; // baseline spark fraction (quiet passages)
const CREST_SPARK_ENERGY: f32 = 0.45; // extra spark fraction scaled by onset energy
const CREST_SPARK_BEAT: f32 = 0.30; // extra spark fraction on a beat pulse
const CREST_SPARK_MAX: f32 = 0.60; // cap so ambient dust keeps a ~40% floor (corners stay filled)
const CREST_PUSH: f32 = 0.18; // how far past the mean ring a loud crest seeds (frac)
const CREST_SPEED: f32 = 0.012; // extra outward launch speed for a loud crest spark

/// Tiny xorshift32 RNG — fast, deterministic, dependency-free. Visual scatter
/// only (not cryptographic); mirrors the LCG the flash effect already uses.
struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    fn new(seed: u32) -> Self {
        Self {
            state: seed | 1, // never zero (xorshift fixed point)
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform `f32` in `[0, 1)`.
    fn unit(&mut self) -> f32 {
        // Top 24 bits → [0, 1); avoids the low-bit weakness of xorshift.
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform `f32` in `[lo, hi)`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

/// Hash a 2D integer lattice point to `[0, 1)` (for the value noise).
fn hash2(x: i32, y: i32) -> f32 {
    let mut h = (x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))) as u32;
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth 2D value noise at `(x, y)` — bilinearly interpolated hashed lattice
/// with a smoothstep fade. Cheap, continuous, no dependency.
fn value_noise(x: f32, y: f32) -> f32 {
    let x0 = x.floor();
    let y0 = y.floor();
    let xf = x - x0;
    let yf = y - y0;
    let (ix, iy) = (x0 as i32, y0 as i32);
    let v00 = hash2(ix, iy);
    let v10 = hash2(ix + 1, iy);
    let v01 = hash2(ix, iy + 1);
    let v11 = hash2(ix + 1, iy + 1);
    let sx = xf * xf * (3.0 - 2.0 * xf);
    let sy = yf * yf * (3.0 - 2.0 * yf);
    let a = v00 + sx * (v10 - v00);
    let b = v01 + sx * (v11 - v01);
    a + sy * (b - a)
}

/// Divergence-free-ish 2D flow vector from the curl of a scalar noise potential
/// (so the field swirls rather than just pushing everything one way). `t` slides
/// the noise so the flow evolves over time.
fn curl_flow(x: f32, y: f32, t: f32) -> (f32, f32) {
    const E: f32 = 0.05;
    let psi = |px: f32, py: f32| value_noise(px + t, py - t * 0.7);
    let dpsi_dy = (psi(x, y + E) - psi(x, y - E)) / (2.0 * E);
    let dpsi_dx = (psi(x + E, y) - psi(x - E, y)) / (2.0 * E);
    (dpsi_dy, -dpsi_dx) // 2D curl of the scalar potential ψ
}

#[derive(Clone, Copy, Default)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,      // remaining life in frames
    life_span: f32, // total life in frames
    size: f32,      // normalized radius
    color_t: f32,   // 0..1 position along the gradient palette
    phase: f32,     // twinkle phase offset
}

/// Per-frame launch context shared by every respawn this frame — the audio
/// envelope plus the live waveform the crest sparks are sampled from. Bundled
/// into one struct so the spawn helpers take a single ref instead of a long
/// positional argument list.
struct SpawnCtx<'a> {
    ring_radius: f32,
    energy: f32,
    beat: f32,
    speed_mul: f32,
    /// The ring's time-domain waveform (signed, ~-1..1), or empty when none is
    /// available (pre-warm) — then every respawn is ambient.
    waveform: &'a [f32],
    /// Scope sensitivity (waveform gain) so a louder crest throws a hotter spark.
    sensitivity: f32,
}

/// Place a particle freshly into the field. A fraction of respawns (scaled by
/// the music) launch as crest sparks off the ring's loud arcs; the rest fill the
/// square panel uniformly so the corners stay populated. Free function (not a
/// method) so `update()` can split the borrow of the pool from the RNG.
fn respawn(p: &mut Particle, rng: &mut XorShift32, ctx: &SpawnCtx<'_>) {
    let spark_frac =
        (CREST_SPARK_BASE + CREST_SPARK_ENERGY * ctx.energy + CREST_SPARK_BEAT * ctx.beat)
            .min(CREST_SPARK_MAX);
    if ctx.waveform.len() >= 2 && rng.unit() < spark_frac {
        respawn_crest_spark(p, rng, ctx);
    } else {
        respawn_ambient(p, rng, ctx);
    }
}

/// Ambient dust: spawn uniformly across the SQUARE panel (so the corners fill,
/// not just a disc), rejecting the inner disc just inside the ring so the
/// album-art face stays clearer. Bounded rejection sample.
fn respawn_ambient(p: &mut Particle, rng: &mut XorShift32, ctx: &SpawnCtx<'_>) {
    let inner = (ctx.ring_radius - INNER_MARGIN).max(0.0);
    let inner2 = inner * inner;
    let (mut x, mut y) = (0.0_f32, 0.0_f32);
    for _ in 0..8 {
        x = rng.range(-1.0, 1.0);
        y = rng.range(-1.0, 1.0);
        if x * x + y * y >= inner2 {
            break;
        }
    }
    p.x = x;
    p.y = y;

    // Gentle outward drift (from the centre) + a perpendicular swirl — the field
    // is already distributed, so the launch is light; curl noise does the rest.
    let len = (x * x + y * y).sqrt().max(1e-4);
    let (cos, sin) = (x / len, y / len);
    let speed = (BASE_SPEED + ENERGY_SPEED * ctx.energy + BEAT_BURST * ctx.beat)
        * ctx.speed_mul
        * rng.range(0.3, 1.0);
    let tang = rng.range(-TANGENTIAL, TANGENTIAL);
    p.vx = cos * speed - sin * tang;
    p.vy = sin * speed + cos * tang;

    p.life_span = rng.range(MIN_LIFE, MAX_LIFE);
    p.life = p.life_span;

    // Power-law size (many small dust, a few large sparks), bigger on a loud
    // launch — gives a visual hierarchy instead of uniform mush.
    let u = rng.unit();
    p.size = BASE_SIZE * (0.5 + 2.0 * u * u) * (1.0 + 0.4 * ctx.energy + 0.6 * ctx.beat);

    p.color_t = rng.unit(); // spread across the whole gradient palette
    p.phase = rng.range(0.0, TAU);
}

/// Crest spark: launch from a LOUD azimuth of the ring (importance-sampled from
/// the waveform) so sparks fly off the wave's crests. The spark inherits the
/// ring's colour at that azimuth (`color_t = angle fraction`), so the dust and
/// the position-coloured ring share one palette story.
fn respawn_crest_spark(p: &mut Particle, rng: &mut XorShift32, ctx: &SpawnCtx<'_>) {
    let wave = ctx.waveform;
    let n = wave.len();
    // Importance-sample a loud azimuth: loudest of a few cheap random taps.
    let mut best = (rng.next_u32() as usize) % n;
    let mut best_mag = wave[best].abs();
    for _ in 0..3 {
        let cand = (rng.next_u32() as usize) % n;
        let mag = wave[cand].abs();
        if mag > best_mag {
            best = cand;
            best_mag = mag;
        }
    }
    let loud = (best_mag * ctx.sensitivity).min(1.0);
    let frac = best as f32 / n as f32; // position around the ring (0..1)
    let angle = TAU * frac;
    let (cos, sin) = (angle.cos(), angle.sin());

    // Seed just outside the ring crest at that azimuth, then fly outward.
    let crest_r = ctx.ring_radius * (1.0 + loud * CREST_PUSH);
    p.x = cos * crest_r;
    p.y = sin * crest_r;
    let speed =
        (BASE_SPEED + ENERGY_SPEED * ctx.energy + BEAT_BURST * ctx.beat + CREST_SPEED * loud)
            * ctx.speed_mul
            * rng.range(0.5, 1.0);
    let tang = rng.range(-TANGENTIAL, TANGENTIAL);
    p.vx = cos * speed - sin * tang;
    p.vy = sin * speed + cos * tang;

    p.life_span = rng.range(MIN_LIFE, MAX_LIFE);
    p.life = p.life_span;

    // Loud crests throw bigger, brighter sparks.
    let u = rng.unit();
    p.size = BASE_SIZE * (0.6 + 2.2 * u * u) * (1.0 + 0.5 * loud + 0.5 * ctx.beat);

    p.color_t = frac; // match the ring's position-gradient colour at this azimuth
    p.phase = rng.range(0.0, TAU);
}

/// Birth/death alpha envelope: ramps up over the first `FADE_IN` of life, holds,
/// then ramps down over the final `FADE_OUT`.
fn life_alpha(life: f32, life_span: f32) -> f32 {
    if life_span <= 0.0 {
        return 0.0;
    }
    let remaining = (life / life_span).clamp(0.0, 1.0); // 1 at birth → 0 at death
    let age = 1.0 - remaining; // 0 at birth → 1 at death
    let fade_in = (age / FADE_IN).min(1.0);
    let fade_out = (remaining / FADE_OUT).min(1.0);
    (fade_in * fade_out).clamp(0.0, 1.0)
}

/// A pool of drifting particles plus the per-frame GPU snapshot.
pub(crate) struct ParticleSystem {
    particles: Vec<Particle>,
    rng: XorShift32,
    /// Monotonic frame counter driving the curl flow field's time evolution
    /// (`t = frame * CURL_TIME`). Kept as `u64` (not `f32`) so the increment
    /// never saturates: an `f32` counter stops advancing past 2^24 (~77 h at
    /// 60 Hz) and would freeze the swirl. As `u64` it keeps advancing for the
    /// lifetime of any session; the `as f32` cast loses only sub-integer
    /// resolution at extreme counts, so the field keeps evolving smoothly.
    frame: u64,
    /// Twinkle accumulator, wrapped to `[0, TAU)` every frame so the per-particle
    /// brightness shimmer stays precise over arbitrarily long playback — a bare
    /// `frame * TWINKLE_RATE` fed to `sin()` loses f32 resolution after ~2 days
    /// of continuous play.
    twinkle_phase: f32,
    /// Two `vec4`s per particle: (x, y, size, alpha) + (colour_t, _, _, _) in
    /// normalized ring-space.
    gpu: Vec<[f32; 8]>,
}

impl ParticleSystem {
    /// Build a pre-warmed pool of `count` particles at the given ring radius so
    /// the field is already full the first frame it's shown (no spawn pop).
    pub(crate) fn new(count: usize, ring_radius: f32, seed: u32) -> Self {
        let mut rng = XorShift32::new(seed);
        let mut particles = Vec::with_capacity(count);
        for _ in 0..count {
            particles.push(Self::scattered(&mut rng, ring_radius));
        }
        let gpu = Vec::with_capacity(count);
        let mut sys = Self {
            particles,
            rng,
            frame: 0,
            twinkle_phase: 0.0,
            gpu,
        };
        sys.rebuild_gpu();
        sys
    }

    /// A particle spawned at the ring then advanced a random fraction through
    /// its life (and trajectory) so a fresh pool looks already-running.
    fn scattered(rng: &mut XorShift32, ring_radius: f32) -> Particle {
        let mut p = Particle::default();
        // Pre-warm has no live waveform → all ambient (fills the square).
        let ctx = SpawnCtx {
            ring_radius,
            energy: 0.0,
            beat: 0.0,
            speed_mul: 1.0,
            waveform: &[],
            sensitivity: 1.0,
        };
        respawn(&mut p, rng, &ctx);
        let elapsed = rng.unit() * p.life_span;
        p.life = (p.life_span - elapsed).max(1.0);
        // Linear advance (ignoring damping) is close enough to fill the field.
        p.x += p.vx * elapsed;
        p.y += p.vy * elapsed;
        p
    }

    /// Resize the pool, pre-warming any new particles at the current ring.
    pub(crate) fn set_count(&mut self, count: usize, ring_radius: f32) {
        if count == self.particles.len() {
            return;
        }
        if count < self.particles.len() {
            self.particles.truncate(count);
        } else {
            while self.particles.len() < count {
                let p = Self::scattered(&mut self.rng, ring_radius);
                self.particles.push(p);
            }
        }
    }

    /// Advance the simulation one frame. `energy`/`beat` are the visualizer's
    /// onset envelope + beat pulse (`~[0, 1]`); `speed_mul` is the user's
    /// particle-speed setting; `waveform` is the ring's signed time-domain trace
    /// (crest sparks are sampled from it) and `sensitivity` its gain.
    pub(crate) fn update(
        &mut self,
        ring_radius: f32,
        energy: f32,
        beat: f32,
        speed_mul: f32,
        waveform: &[f32],
        sensitivity: f32,
    ) {
        let t = self.frame as f32 * CURL_TIME;
        // Swirl intensifies with the music AND scales with the user's speed
        // setting, so `particle_speed` governs the ongoing drift — not just the
        // launch kick (which damps away). Without this, the unscaled swirl floors
        // the slider's low end and the setting reads as doing almost nothing.
        let flow = CURL_STRENGTH * (1.0 + energy) * speed_mul;
        let ctx = SpawnCtx {
            ring_radius,
            energy,
            beat,
            speed_mul,
            waveform,
            sensitivity,
        };
        {
            // Split the borrow so respawn can take &mut rng while we hold &mut pool.
            let Self { particles, rng, .. } = self;
            for p in particles.iter_mut() {
                // Curl-noise flow nudge, then integrate.
                let (fx, fy) = curl_flow(p.x * CURL_FREQ, p.y * CURL_FREQ, t);
                p.vx += fx * flow;
                p.vy += fy * flow;
                p.x += p.vx;
                p.y += p.vy;
                p.vx *= DAMPING;
                p.vy *= DAMPING;
                p.life -= 1.0;
                // Recycle on the SQUARE panel bound so particles can reach the
                // corners (a circular bound would leave them empty).
                if p.life <= 0.0 || p.x.abs() > PANEL_RECYCLE || p.y.abs() > PANEL_RECYCLE {
                    respawn(p, rng, &ctx);
                }
            }
        }
        self.frame = self.frame.wrapping_add(1);
        self.twinkle_phase = (self.twinkle_phase + TWINKLE_RATE).rem_euclid(TAU);
        self.rebuild_gpu();
    }

    fn rebuild_gpu(&mut self) {
        let twinkle_phase = self.twinkle_phase;
        let Self { particles, gpu, .. } = self;
        gpu.clear();
        for p in particles.iter() {
            let twinkle = 1.0 - TWINKLE_DEPTH + TWINKLE_DEPTH * (twinkle_phase + p.phase).sin();
            let alpha = life_alpha(p.life, p.life_span) * twinkle.max(0.0);
            gpu.push([p.x, p.y, p.size, alpha, p.color_t, 0.0, 0.0, 0.0]);
        }
    }

    /// The per-particle GPU snapshot (two `vec4`s each, see struct docs).
    pub(crate) fn gpu_data(&self) -> &[[f32; 8]] {
        &self.gpu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RING: f32 = 0.7;

    #[test]
    fn new_fills_the_pool_with_valid_snapshots() {
        let sys = ParticleSystem::new(128, RING, 12345);
        let gpu = sys.gpu_data();
        assert_eq!(gpu.len(), 128);
        for p in gpu {
            assert!(p[2] > 0.0, "size positive");
            assert!((0.0..=1.0).contains(&p[3]), "alpha in [0,1]: {}", p[3]);
            assert!((0.0..=1.0).contains(&p[4]), "colour_t in [0,1]: {}", p[4]);
            assert!(p[0].is_finite() && p[1].is_finite());
        }
    }

    #[test]
    fn update_keeps_count_and_respawns_dead() {
        let mut sys = ParticleSystem::new(64, RING, 7);
        // A loud waveform so crest sparks fire too (exercises that spawn path).
        let wave: Vec<f32> = (0..64).map(|i| (i as f32 * 0.3).sin() * 0.8).collect();
        // Many frames: every particle must have recycled at least once and the
        // pool size + snapshot length stay fixed (no leaks, no growth).
        for _ in 0..600 {
            sys.update(RING, 0.5, 0.2, 1.0, &wave, 2.0);
        }
        assert_eq!(sys.gpu_data().len(), 64);
        // The curl frame counter advances once per update and is an integer, so
        // it never saturates the way an f32 counter would past 2^24 frames
        // (which froze the flow field). 600 updates from a fresh pool → 600.
        assert_eq!(sys.frame, 600);
        // The twinkle accumulator stays wrapped to [0, TAU) no matter how many
        // frames elapse (precision guard against unbounded growth).
        assert!(
            (0.0..TAU).contains(&sys.twinkle_phase),
            "got {}",
            sys.twinkle_phase
        );
        // No particle is left dead after an update (respawn fires in-loop).
        assert!(sys.particles.iter().all(|p| p.life > 0.0));
        // Recycling fires the same frame a particle crosses the panel bound, so
        // none is left beyond it (plus a frame's worth of movement).
        assert!(
            sys.particles
                .iter()
                .all(|p| p.x.abs() <= PANEL_RECYCLE + 0.05 && p.y.abs() <= PANEL_RECYCLE + 0.05)
        );
    }

    #[test]
    fn set_count_resizes_both_ways() {
        let mut sys = ParticleSystem::new(32, RING, 99);
        sys.set_count(80, RING);
        sys.update(RING, 0.0, 0.0, 1.0, &[], 1.0);
        assert_eq!(sys.gpu_data().len(), 80);
        sys.set_count(10, RING);
        sys.update(RING, 0.0, 0.0, 1.0, &[], 1.0);
        assert_eq!(sys.gpu_data().len(), 10);
    }

    #[test]
    fn crest_spark_seeds_just_outside_the_ring() {
        // A uniformly loud waveform → every importance sample is loud, so each
        // spark seeds at the crest radius and flies outward.
        let mut rng = XorShift32::new(42);
        let wave = vec![0.8f32; 64];
        let ctx = SpawnCtx {
            ring_radius: RING,
            energy: 1.0,
            beat: 1.0,
            speed_mul: 1.0,
            waveform: &wave,
            sensitivity: 2.0,
        };
        for _ in 0..50 {
            let mut p = Particle::default();
            respawn_crest_spark(&mut p, &mut rng, &ctx);
            let r = (p.x * p.x + p.y * p.y).sqrt();
            // The setup forces loud = (0.8 * 2.0).min(1.0) = 1.0, so the spark
            // must seed at the FULL crest radius RING*(1 + CREST_PUSH) — pin it
            // tightly so a regression that dropped the `(1 + loud*CREST_PUSH)`
            // push (seeding at the bare ring) fails instead of slipping through.
            let want = RING * (1.0 + CREST_PUSH);
            assert!(
                (want - 1e-3..=want + 1e-3).contains(&r),
                "spark seeds at the loud crest radius {want}, got r={r}"
            );
            // Velocity has a net outward component.
            assert!(p.x * p.vx + p.y * p.vy > 0.0, "spark flies outward");
            // color_t encodes the spawn azimuth, so the spark inherits the ring's
            // position-gradient colour there: atan2(y, x) == TAU * color_t (mod TAU).
            let ang = p.y.atan2(p.x).rem_euclid(TAU);
            let want = (p.color_t * TAU).rem_euclid(TAU);
            let d = (ang - want).abs();
            assert!(
                d < 1e-3 || (TAU - d) < 1e-3,
                "color_t tracks the spawn azimuth: ang={ang} from_color_t={want}"
            );
        }
    }

    #[test]
    fn particle_speed_scales_field_velocity() {
        // `particle_speed` must visibly change how fast the dust MOVES, not just
        // the launch kick (which damps away) — so it scales the ongoing curl
        // swirl too. Same seed + same audio, two speeds: the faster setting must
        // leave the field moving clearly quicker. Deterministic (fixed seed).
        let mean_speed = |sys: &ParticleSystem| -> f32 {
            let n = sys.particles.len().max(1) as f32;
            sys.particles
                .iter()
                .map(|p| (p.vx * p.vx + p.vy * p.vy).sqrt())
                .sum::<f32>()
                / n
        };
        // Identical seed → identical starting pool; only `speed_mul` differs.
        let mut slow = ParticleSystem::new(256, RING, 2024);
        let mut fast = ParticleSystem::new(256, RING, 2024);
        for _ in 0..12 {
            slow.update(RING, 0.5, 0.0, 0.5, &[], 1.0);
            fast.update(RING, 0.5, 0.0, 4.0, &[], 1.0);
        }
        let (sv, fv) = (mean_speed(&slow), mean_speed(&fast));
        assert!(
            fv > sv * 1.3,
            "higher particle_speed must move the field faster: fast={fv}, slow={sv}"
        );
    }

    #[test]
    fn life_alpha_fades_in_and_out() {
        // Birth (full life) and death (no life) are transparent; mid-life opaque.
        assert!(life_alpha(100.0, 100.0) < 0.05, "fades in from birth");
        assert!(life_alpha(0.0, 100.0) < 1e-6, "zero at death");
        assert!(life_alpha(50.0, 100.0) > 0.9, "opaque mid-life");
    }

    #[test]
    fn value_noise_is_smooth_and_bounded() {
        // Bounded to [0,1] and continuous: adjacent samples are close.
        let a = value_noise(3.21, 7.65);
        let b = value_noise(3.22, 7.65);
        assert!((0.0..=1.0).contains(&a));
        assert!((a - b).abs() < 0.1, "noise is continuous");
    }
}
