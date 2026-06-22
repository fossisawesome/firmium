//! Shared playback types. Previously defined at the crate root in `lib.rs`;
//! re-exported at the crate root by `main.rs` so existing `crate::PlaybackState`
//! / `crate::AudioDevice` references keep resolving.

/// Playback state reported by the audio engine.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

/// Audio output device information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub default: bool,
}
