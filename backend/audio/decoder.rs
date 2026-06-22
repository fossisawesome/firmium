//! Symphonia probe/decode wrapper. Opens a `MediaSource`, probes the
//! container format, and decodes packets to interleaved f32 sample chunks.

use std::time::Duration;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatReader, FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// Sample rate assumed when the container/codec doesn't report one up front.
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

pub struct DecoderHandle {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    pub sample_rate: u32,
    pub channels: u16,
}

impl DecoderHandle {
    /// Open and probe `source`, returning the decoder plus the track's
    /// total duration in seconds (if known from container metadata).
    pub fn open(source: Box<dyn MediaSource>) -> Result<(Self, Option<f64>), String> {
        let mss = MediaSourceStream::new(source, MediaSourceStreamOptions::default());

        let probed = symphonia::default::get_probe()
            .format(&Hint::new(), mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| format!("Failed to probe format: {e}"))?;

        let format = probed.format;
        let track = format.default_track().ok_or("No default track found")?;
        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create decoder: {e}"))?;

        // If the container doesn't report a sample rate, default to 48kHz
        // rather than failing the track outright.
        let sample_rate = codec_params.sample_rate.unwrap_or(DEFAULT_SAMPLE_RATE);
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let duration = codec_params
            .n_frames
            .filter(|&n| n > 0)
            .map(|n| n as f64 / sample_rate as f64);

        Ok((Self { format, decoder, track_id, sample_rate, channels }, duration))
    }

    /// Decode the next packet for our track and return its samples as
    /// interleaved f32. Returns `Ok(None)` at end of stream.
    pub fn next_samples(&mut self) -> Result<Option<Vec<f32>>, String> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(format!("Read error: {e}")),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity() as u64;
                    let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
                    sample_buf.copy_interleaved_ref(decoded);
                    return Ok(Some(sample_buf.samples().to_vec()));
                }
                // Recoverable: skip the bad packet and keep going.
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => return Err(format!("Decode error: {e}")),
            }
        }
    }

    /// Seek to `pos` using the format's native seek support. On success the
    /// decoder is reset (required by symphonia after a seek) and the caller
    /// must discard any buffered samples decoded before the seek.
    pub fn seek(&mut self, pos: Duration) -> Result<(), String> {
        let time = Time::new(pos.as_secs(), pos.subsec_nanos() as f64 / 1_000_000_000.0);
        self.format
            .seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(self.track_id) })
            .map_err(|e| format!("Seek error: {e}"))?;
        self.decoder.reset();
        Ok(())
    }
}
