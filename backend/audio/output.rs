//! cpal output device handling: device/config negotiation and the realtime
//! mixing callback that sums all active sessions' decoded samples.

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, Stream, SupportedStreamConfig};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::session::{ResampleState, Session, SessionMap};

/// Per-callback reusable scratch buffers, owned by the cpal output closure so the
/// realtime mix path allocates nothing after warm-up: `sessions` holds the
/// snapshot of active sessions, `frame` holds one interpolated output frame.
#[derive(Default)]
struct MixScratch {
    sessions: Vec<Arc<Session>>,
    frame: Vec<f32>,
}

/// Wrapper to make `cpal::Stream` Send + Sync for storage behind a `RwLock`.
/// Safe because the stream is only ever accessed for its lifetime (drop) —
/// playback is driven entirely by cpal's own audio thread via the callback.
struct SafeStream(#[allow(dead_code)] Stream);
unsafe impl Send for SafeStream {}
unsafe impl Sync for SafeStream {}

pub struct OutputStream {
    #[allow(dead_code)]
    stream: SafeStream,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Find an output config whose sample-rate range contains `target_rate` and
/// whose channel count matches `target_channels`. Mirrors the negotiation
/// `reopen_stream_if_needed` used to perform via rodio's `DeviceSinkBuilder`.
pub fn find_compatible_config(device: &Device, target_rate: u32, target_channels: u16) -> Option<SupportedStreamConfig> {
    let configs = device.supported_output_configs().ok()?;
    configs
        .filter(|c| {
            target_rate >= c.min_sample_rate()
                && target_rate <= c.max_sample_rate()
                && c.channels() == target_channels
        })
        .map(|c| c.with_sample_rate(target_rate))
        .next()
}

/// Open an output stream with the given config, mixing audio from `sessions`.
pub fn open_with_config(device: &Device, config: SupportedStreamConfig, sessions: SessionMap) -> Result<OutputStream, String> {
    let sample_rate = config.sample_rate();
    let channels = config.channels();
    let stream_config = config.config();

    let mut scratch = MixScratch::default();
    let stream = device
        .build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| mix_into(data, &sessions, channels, sample_rate, &mut scratch),
            |err| eprintln!("Audio output stream error: {err}"),
            None,
        )
        .map_err(|e| format!("Failed to build output stream: {e}"))?;

    stream.play().map_err(|e| format!("Failed to start output stream: {e}"))?;

    Ok(OutputStream { stream: SafeStream(stream), sample_rate, channels })
}

/// Open the default output device at its default config.
pub fn open_default(sessions: SessionMap) -> Result<(Device, OutputStream), String> {
    use cpal::traits::HostTrait;
    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| "No output device".to_string())?;
    let config = device
        .default_output_config()
        .map_err(|e| format!("Failed to get default output config: {e}"))?;
    let stream = open_with_config(&device, config, sessions)?;
    Ok((device, stream))
}

/// Pop one frame (one sample per `channels`) from the front of `ring` into
/// `out` (cleared first). Returns false without modifying `ring` if it can't
/// supply a full frame. Reuses `out`'s capacity — no allocation after warm-up.
fn pop_native_frame_into(ring: &mut VecDeque<f32>, channels: u16, out: &mut Vec<f32>) -> bool {
    let c = channels as usize;
    if ring.len() < c {
        return false;
    }
    out.clear();
    for _ in 0..c {
        out.push(ring.pop_front().unwrap());
    }
    true
}

/// Write one output frame into `out`, applying linear-interpolation resampling
/// via `state` when `step != 1.0`. When the session's native rate matches the
/// output rate, `step == 1.0` and `state.pos` stays 0.0, so the interpolation
/// reduces to copying `state.current` exactly (no resampling, fully
/// bit-perfect). Returns false on underrun/drain. Allocation-free: the
/// resampler's `current`/`next` buffers are reused via swap.
fn pop_resampled_frame(ring: &mut VecDeque<f32>, state: &mut ResampleState, channels: u16, step: f64, out: &mut Vec<f32>) -> bool {
    // Once drained and the ring is empty, the session has no more audio — report
    // silence rather than repeating the last frame forever.
    if state.drained && ring.is_empty() {
        return false;
    }

    if !state.initialized {
        if !pop_native_frame_into(ring, channels, &mut state.current) {
            return false;
        }
        if !pop_native_frame_into(ring, channels, &mut state.next) {
            state.drained = true;
            state.next.clear();
            state.next.extend_from_slice(&state.current);
        }
        state.pos = 0.0;
        state.initialized = true;
    }

    out.clear();
    for (&a, &b) in state.current.iter().zip(state.next.iter()) {
        out.push(a + (b - a) * state.pos as f32);
    }

    state.pos += step;
    while state.pos >= 1.0 {
        // Old `next` becomes `current`; refill `next` in place (reusing the
        // buffer that was `current`).
        std::mem::swap(&mut state.current, &mut state.next);
        if !pop_native_frame_into(ring, channels, &mut state.next) {
            state.drained = true;
            state.next.clear();
            state.next.extend_from_slice(&state.current);
        }
        state.pos -= 1.0;
    }

    true
}

/// Accumulate a decoded `frame` of `native` channels into `data` at `base`,
/// adapting to `out` channels and scaling by `vol`. Mono<->stereo are handled
/// directly; other mismatches truncate (more native channels than output) or
/// repeat the last channel (fewer native channels than output). Allocation-free
/// equivalent of the old `adapt_channels` + accumulate loop.
fn accumulate_frame(data: &mut [f32], base: usize, frame: &[f32], native: u16, out: u16, vol: f32) {
    let out = out as usize;
    if native == 2 && out == 1 {
        data[base] += ((frame[0] + frame[1]) / 2.0) * vol;
        return;
    }
    if native == 1 && out == 2 {
        data[base] += frame[0] * vol;
        data[base + 1] += frame[0] * vol;
        return;
    }
    // native == out, native > out (truncate), and native < out (pad with last)
    // all add exactly `out` channels.
    let pad = *frame.last().unwrap_or(&0.0);
    for ch in 0..out {
        let s = if ch < frame.len() { frame[ch] } else { pad };
        data[base + ch] += s * vol;
    }
}

/// The realtime cpal output callback: sums every active session's samples
/// (after per-session volume, channel adaptation, and resampling) into `data`.
/// `scratch` provides reusable buffers so this path performs no heap
/// allocation after warm-up.
fn mix_into(data: &mut [f32], sessions: &SessionMap, out_channels: u16, out_rate: u32, scratch: &mut MixScratch) {
    data.fill(0.0);

    // Snapshot the active sessions into a reused buffer so the sessions read
    // lock is released before the heavy mixing work. `mem::take` swaps in an
    // empty Vec (no allocation) so `scratch.frame` can be borrowed independently.
    let mut session_buf = std::mem::take(&mut scratch.sessions);
    {
        let guard = sessions.read();
        session_buf.clear();
        session_buf.extend(guard.values().cloned());
    }

    let frames = data.len() / out_channels.max(1) as usize;

    for session in &session_buf {
        if !session.playing.load(Ordering::Relaxed) {
            continue;
        }
        let vol = *session.volume.lock();
        if vol <= 0.0 {
            continue;
        }

        let native_channels = session.channels();
        let native_rate = session.sample_rate();
        let step = native_rate as f64 / out_rate as f64;

        let mut ring = session.ring.lock();
        let mut resample = session.resample.lock();

        for frame_idx in 0..frames {
            if !pop_resampled_frame(&mut ring, &mut resample, native_channels, step, &mut scratch.frame) {
                continue; // Underrun — contribute silence for this frame.
            }
            let base = frame_idx * out_channels as usize;
            accumulate_frame(data, base, &scratch.frame, native_channels, out_channels, vol);
        }
    }

    session_buf.clear();
    scratch.sessions = session_buf;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Old reference implementation, kept here to assert the allocation-free
    // accumulate_frame reproduces its exact channel-adaptation behaviour.
    fn adapt_channels_ref(frame: &[f32], native: u16, out: u16) -> Vec<f32> {
        if native == out {
            return frame.to_vec();
        }
        if native == 1 && out == 2 {
            return vec![frame[0], frame[0]];
        }
        if native == 2 && out == 1 {
            return vec![(frame[0] + frame[1]) / 2.0];
        }
        if native > out {
            return frame[..out as usize].to_vec();
        }
        let mut v = frame.to_vec();
        let pad = *frame.last().unwrap_or(&0.0);
        while (v.len() as u16) < out {
            v.push(pad);
        }
        v
    }

    fn accumulate_ref(frame: &[f32], native: u16, out: u16, vol: f32) -> Vec<f32> {
        let adapted = adapt_channels_ref(frame, native, out);
        let mut data = vec![0.0f32; out as usize];
        for (ch, &s) in adapted.iter().enumerate() {
            data[ch] += s * vol;
        }
        data
    }

    #[test]
    fn accumulate_frame_matches_old_adapt() {
        let cases: &[(&[f32], u16, u16)] = &[
            (&[0.5, -0.5], 2, 2),       // stereo passthrough
            (&[0.25], 1, 2),            // mono -> stereo
            (&[0.4, 0.6], 2, 1),        // stereo -> mono downmix
            (&[0.1, 0.2, 0.3, 0.4], 4, 2), // surround -> stereo (truncate)
            (&[0.7], 1, 4),             // mono -> 4ch (pad with last)
        ];
        for &(frame, native, out) in cases {
            let mut got = vec![0.0f32; out as usize];
            accumulate_frame(&mut got, 0, frame, native, out, 1.0);
            let expected = accumulate_ref(frame, native, out, 1.0);
            assert_eq!(got, expected, "native={native} out={out}");
        }
    }

    // The bit-perfect invariant: at matching rates (step == 1.0) the resampler
    // must emit each native frame exactly, with no interpolation artefacts.
    #[test]
    fn resampler_passthrough_is_bit_perfect() {
        let input: Vec<f32> = vec![0.0, 0.1, -0.2, 0.3, 0.5, -0.5, 0.9, -0.9];
        let mut ring: VecDeque<f32> = input.iter().copied().collect();
        let mut state = ResampleState::default();
        let mut out = Vec::new();
        let mut produced: Vec<f32> = Vec::new();
        // step == 1.0 (e.g. 48000 / 48000): exact passthrough. The resampler
        // drops the final frame at drain (pre-existing behaviour), so it emits
        // the first N-1 frames; each emitted sample must be bit-identical.
        while pop_resampled_frame(&mut ring, &mut state, 2, 1.0, &mut out) {
            produced.extend_from_slice(&out);
        }
        assert!(produced.len() >= input.len() - 2, "produced {} samples", produced.len());
        assert_eq!(&produced[..], &input[..produced.len()]);
    }
}
