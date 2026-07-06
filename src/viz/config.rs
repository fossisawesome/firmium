use iced::Color;
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

#[derive(Debug, Clone)]
pub struct VizConfig {
    pub gradient_colors: Vec<Color>,
    pub peak_gradient_colors: Vec<Color>,
    pub border_color: Color,
    pub peak_color: Color,
    pub border_width: f32,
    pub peak_enabled: bool,
    pub peak_thickness: f32,
    pub peak_alpha: f32,
    pub line_thickness: f32,
    pub bar_width: f32,
    pub bar_spacing: f32,
    pub edge_spacing: f32,
    pub led_bars: bool,
    pub led_segment_height: f32,
    pub led_border_opacity: f32,
    pub border_opacity: f32,
    pub gradient_mode: u32,
    pub peak_gradient_mode: u32,
    pub peak_mode: u32,
    pub peak_hold_time: f32,
    pub peak_fade_time: f32,
    pub bar_depth_3d: f32,
    pub gradient_orientation: u32,
    pub global_opacity: f32,
    // --- Bars-only knobs (beyond the generic fields above, which cover
    // border_width/bar_width/bar_spacing/peak_thickness/led_*/peak_hold_time/
    // peak_fade_time/bar_depth_3d already) ---
    pub bars_max_bars: u32,
    pub bars_flash_intensity: f32,
    pub bars_trails: f32,
    pub bars_echo: f32,
    // --- Lines ---
    pub lines_point_count: u32,
    pub lines_outline_thickness: f32,
    pub lines_outline_opacity: f32,
    pub lines_animation_speed: f32,
    pub lines_gradient_mode: u32,
    pub lines_fill_opacity: f32,
    pub lines_mirror: bool,
    pub lines_glow_intensity: f32,
    pub lines_style: u32,
    pub lines_trails: f32,
    pub lines_echo: f32,
    // --- Scope ---
    pub scope_radius: f32,
    pub scope_sensitivity: f32,
    pub scope_point_count: u32,
    pub scope_line_thickness: f32,
    pub scope_fill_opacity: f32,
    pub scope_glow_intensity: f32,
    pub scope_outline_thickness: f32,
    pub scope_outline_opacity: f32,
    pub scope_gradient_mode: u32,
    pub scope_animation_speed: f32,
    pub scope_style: u32,
    pub scope_particles: bool,
    pub scope_particle_count: u32,
    pub scope_particle_speed: f32,
    pub scope_beam: bool,
    pub scope_trails: f32,
    pub scope_echo: f32,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub beat_reactivity: f32,
    pub crt: f32,
}

impl Default for VizConfig {
    fn default() -> Self {
        Self {
            gradient_colors: vec![
                Color::from_rgb(0.1, 0.4, 0.9),
                Color::from_rgb(0.1, 0.6, 1.0),
                Color::from_rgb(0.0, 0.8, 1.0),
                Color::from_rgb(0.0, 1.0, 0.9),
                Color::from_rgb(0.0, 0.9, 0.7),
                Color::from_rgb(0.1, 0.7, 0.8),
                Color::from_rgb(0.2, 0.5, 0.9),
                Color::from_rgb(0.3, 0.4, 1.0),
            ],
            peak_gradient_colors: vec![
                Color::from_rgb(1.0, 0.6, 0.0),
                Color::from_rgb(1.0, 0.4, 0.0),
                Color::from_rgb(1.0, 0.2, 0.1),
                Color::from_rgb(1.0, 0.8, 0.0),
                Color::from_rgb(1.0, 0.5, 0.1),
                Color::from_rgb(0.9, 0.3, 0.0),
                Color::from_rgb(1.0, 0.7, 0.1),
                Color::from_rgb(1.0, 0.3, 0.2),
            ],
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
            peak_color: Color::WHITE,
            border_width: 1.0,
            peak_enabled: true,
            peak_thickness: 2.0,
            peak_alpha: 0.9,
            line_thickness: 2.5,
            bar_width: 0.0,
            bar_spacing: 0.0,
            edge_spacing: 0.0,
            led_bars: false,
            led_segment_height: 6.0,
            led_border_opacity: 0.5,
            border_opacity: 0.0,
            gradient_mode: BarsGradientMode::Static.as_u32(),
            peak_gradient_mode: BarsPeakGradientMode::Static.as_u32(),
            peak_mode: BarsPeakMode::Fall.as_u32(),
            peak_hold_time: 1.5,
            peak_fade_time: 0.5,
            bar_depth_3d: 0.0,
            gradient_orientation: BarsGradientOrientation::Vertical.as_u32(),
            global_opacity: 1.0,
            bars_max_bars: 120,
            bars_flash_intensity: 0.6,
            bars_trails: 0.0,
            bars_echo: 0.0,
            lines_point_count: 120,
            lines_outline_thickness: 0.0,
            lines_outline_opacity: 0.6,
            lines_animation_speed: 0.2,
            lines_gradient_mode: GradientMode::Static.as_u32(),
            lines_fill_opacity: 0.15,
            lines_mirror: false,
            lines_glow_intensity: 0.4,
            lines_style: LineStyle::Smooth.as_u32(),
            lines_trails: 0.0,
            lines_echo: 0.0,
            scope_radius: 0.7,
            scope_sensitivity: 1.5,
            scope_point_count: 120,
            scope_line_thickness: 0.01,
            scope_fill_opacity: 0.5,
            scope_glow_intensity: 0.35,
            scope_outline_thickness: 0.0,
            scope_outline_opacity: 0.0,
            scope_gradient_mode: GradientMode::Static.as_u32(),
            scope_animation_speed: 0.1,
            scope_style: LineStyle::Smooth.as_u32(),
            scope_particles: true,
            scope_particle_count: 192,
            scope_particle_speed: 0.5,
            scope_beam: true,
            scope_trails: 0.0,
            scope_echo: 1.0,
            bloom_enabled: true,
            bloom_intensity: 0.4,
            beat_reactivity: 0.7,
            crt: 0.0,
        }
    }
}
