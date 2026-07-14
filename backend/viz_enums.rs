//! Visualizer style enums shared by config persistence (backend) and the
//! desktop iced canvas renderer (desktop/src/viz/). Split out of the
//! desktop-only viz module because these are pure data — no iced dependency —
//! and backend/config.rs needs them for Config's persisted viz settings.

use serde::{Deserialize, Serialize};


/// Gradient color-cycling mode, shared by Lines and Scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientMode {
    /// Time-based cycling through the full gradient palette.
    Breathing,
    /// Uses the first gradient color only.
    Static,
    /// Color by horizontal position / angle around the ring (bass -> treble rainbow).
    Position,
    /// Color by amplitude (quiet -> loud).
    Height,
    /// Position/angle + amplitude blend (peaks shift the palette).
    Gradient,
}

impl GradientMode {
    pub const ALL: [Self; 5] =
        [Self::Breathing, Self::Static, Self::Position, Self::Height, Self::Gradient];

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

impl std::fmt::Display for GradientMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Breathing => "Breathing",
            Self::Static => "Static",
            Self::Position => "Position",
            Self::Height => "Height",
            Self::Gradient => "Gradient",
        })
    }
}

/// Interpolation style between waveform/line points, shared by Lines and Scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    /// Catmull-Rom spline (curvy).
    Smooth,
    /// Straight line segments.
    Angular,
}

impl LineStyle {
    pub const ALL: [Self; 2] = [Self::Smooth, Self::Angular];

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

impl std::fmt::Display for LineStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Smooth => "Smooth",
            Self::Angular => "Angular",
        })
    }
}

/// Bars gradient mode: static height-based gradient vs. wave stretching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarsGradientMode {
    /// Height-based gradient (bottom to top).
    Static,
    /// Gradient stretching (taller bars show more bottom colors).
    Wave,
}

impl BarsGradientMode {
    pub const ALL: [Self; 2] = [Self::Static, Self::Wave];

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

impl std::fmt::Display for BarsGradientMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Static => "Static",
            Self::Wave => "Wave",
        })
    }
}

/// Axis the bar gradient colors are mapped along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarsGradientOrientation {
    /// Colors map bottom-to-top within each bar.
    Vertical,
    /// Colors map left-to-right across bars (bass to treble).
    Horizontal,
}

impl BarsGradientOrientation {
    pub const ALL: [Self; 2] = [Self::Vertical, Self::Horizontal];

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

impl std::fmt::Display for BarsGradientOrientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Vertical => "Vertical",
            Self::Horizontal => "Horizontal",
        })
    }
}

/// Color mode for the peak indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarsPeakGradientMode {
    /// Uses the first color in the peak gradient only.
    Static,
    /// Time-based animation cycling through all peak colors.
    Cycle,
    /// Color based on peak height position.
    Height,
    /// Uses the same color as the bar gradient at that height.
    Match,
}

impl BarsPeakGradientMode {
    pub const ALL: [Self; 4] = [Self::Static, Self::Cycle, Self::Height, Self::Match];

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

impl std::fmt::Display for BarsPeakGradientMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Static => "Static",
            Self::Cycle => "Cycle",
            Self::Height => "Height",
            Self::Match => "Match",
        })
    }
}

/// Peak indicator falloff behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarsPeakMode {
    /// Peak bars disabled.
    None,
    /// Hold, then fade out in place (opacity decreases).
    Fade,
    /// Hold, then fall at constant speed.
    Fall,
    /// Hold, then fall with gravity acceleration.
    FallAccel,
    /// Hold, then fall at constant speed while fading out.
    FallFade,
}

impl BarsPeakMode {
    pub const ALL: [Self; 5] =
        [Self::None, Self::Fade, Self::Fall, Self::FallAccel, Self::FallFade];

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

impl std::fmt::Display for BarsPeakMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "None",
            Self::Fade => "Fade",
            Self::Fall => "Fall",
            Self::FallAccel => "Fall (accelerate)",
            Self::FallFade => "Fall + fade",
        })
    }
}
