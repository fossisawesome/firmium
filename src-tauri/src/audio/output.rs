//! cpal output device handling: device/config negotiation and the realtime
//! mixing callback that sums all active sessions' decoded samples.

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, Stream, SupportedStreamConfig};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;

use super::session::{ResampleState, SessionMap};

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

    let stream = device
        .build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| mix_into(data, &sessions, channels, sample_rate),
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

/// Pop one frame (one sample per `channels`) from the front of `ring`.
fn pop_native_frame(ring: &mut VecDeque<f32>, channels: u16) -> Option<Vec<f32>> {
    let c = channels as usize;
    if ring.len() < c {
        return None;
    }
    Some((0..c).map(|_| ring.pop_front().unwrap()).collect())
}

/// Pop one output frame from `ring`, applying linear-interpolation resampling
/// via `state` when `step != 1.0`. When the session's native rate matches the
/// output rate, `step == 1.0` and this is an exact passthrough (no
/// resampling, fully bit-perfect).
fn pop_resampled_frame(ring: &mut VecDeque<f32>, state: &mut ResampleState, channels: u16, step: f64) -> Option<Vec<f32>> {
    // Once drained and the ring is empty, the session has no more audio — report
    // silence rather than repeating the last frame forever.
    if state.drained && ring.is_empty() {
        return None;
    }

    if !state.initialized {
        state.current = pop_native_frame(ring, channels)?;
        state.next = pop_native_frame(ring, channels).unwrap_or_else(|| {
            state.drained = true;
            state.current.clone()
        });
        state.pos = 0.0;
        state.initialized = true;
    }

    let frame: Vec<f32> = state
        .current
        .iter()
        .zip(state.next.iter())
        .map(|(&a, &b)| a + (b - a) * state.pos as f32)
        .collect();

    state.pos += step;
    while state.pos >= 1.0 {
        state.current = std::mem::take(&mut state.next);
        state.next = match pop_native_frame(ring, channels) {
            Some(f) => f,
            None => {
                state.drained = true;
                state.current.clone()
            }
        };
        state.pos -= 1.0;
    }

    Some(frame)
}

/// Adapt a decoded frame of `native` channels to `out` channels.
/// Mono<->stereo are handled directly; other mismatches fall back to
/// truncation (more native channels than output) or last-channel repeat
/// (fewer native channels than output).
fn adapt_channels(frame: &[f32], native: u16, out: u16) -> Vec<f32> {
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

/// The realtime cpal output callback: sums every active session's samples
/// (after per-session volume, channel adaptation, and resampling) into `data`.
fn mix_into(data: &mut [f32], sessions: &SessionMap, out_channels: u16, out_rate: u32) {
    data.fill(0.0);

    let snapshot: Vec<_> = sessions.read().values().cloned().collect();
    let frames = data.len() / out_channels.max(1) as usize;

    for session in &snapshot {
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
            let Some(native_frame) = pop_resampled_frame(&mut ring, &mut resample, native_channels, step) else {
                continue; // Underrun — contribute silence for this frame.
            };
            let adapted = adapt_channels(&native_frame, native_channels, out_channels);
            let base = frame_idx * out_channels as usize;
            for (ch, &s) in adapted.iter().enumerate() {
                data[base + ch] += s * vol;
            }
        }
    }
}
