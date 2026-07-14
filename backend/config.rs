//! User config persisted at `~/.config/<id>/config.toml`. Replaces the bits of
//! state the Svelte app kept in localStorage (server, last theme, volume).
//! Passwords stay in the OS keyring, not here.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A server + username pair the user has logged into. Passwords for each live in
/// the OS keyring (keyed by server + username), never here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedAccount {
    pub server: String,
    pub username: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub server: Option<String>,
    pub username: Option<String>,
    pub theme_id: Option<String>,
    #[serde(default)]
    pub ui_theme_id: Option<String>,
    pub volume: Option<f32>,
    #[serde(default)]
    pub accounts: Vec<SavedAccount>,
    #[serde(default)]
    pub download_format: Option<String>,
    #[serde(default)]
    pub lrclib_enabled: Option<bool>,
    #[serde(default)]
    pub lyrics_word_fill: Option<bool>,
    #[serde(default)]
    pub window_decorations: Option<bool>,
    #[serde(default)]
    pub viz_cover_colors: Option<bool>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub scrollbar_width: Option<u32>,

    // ── Visualizer: Bars ──────────────────────────────────────────────────────
    #[serde(default)]
    pub bars_monstercat: Option<f32>,
    #[serde(default)]
    pub bars_waves: Option<bool>,
    #[serde(default)]
    pub bars_waves_smoothing: Option<u32>,
    #[serde(default)]
    pub bars_gradient_mode: Option<crate::viz_enums::BarsGradientMode>,
    #[serde(default)]
    pub bars_gradient_orientation: Option<crate::viz_enums::BarsGradientOrientation>,
    #[serde(default)]
    pub bars_peak_gradient_mode: Option<crate::viz_enums::BarsPeakGradientMode>,
    #[serde(default)]
    pub bars_peak_mode: Option<crate::viz_enums::BarsPeakMode>,
    #[serde(default)]
    pub bars_peak_hold_time: Option<f32>,
    #[serde(default)]
    pub bars_peak_fade_time: Option<f32>,
    #[serde(default)]
    pub bars_peak_height: Option<f32>,
    #[serde(default)]
    pub bars_border_width: Option<f32>,
    #[serde(default)]
    pub bars_led_bars: Option<bool>,
    #[serde(default)]
    pub bars_led_segment_height: Option<f32>,
    #[serde(default)]
    pub bars_depth_3d: Option<f32>,
    #[serde(default)]
    pub bars_flash_intensity: Option<f32>,
    #[serde(default)]
    pub bars_max_bars: Option<u32>,
    #[serde(default)]
    pub bars_trails: Option<f32>,
    #[serde(default)]
    pub bars_echo: Option<f32>,

    // ── Visualizer: Lines ─────────────────────────────────────────────────────
    #[serde(default)]
    pub lines_point_count: Option<u32>,
    #[serde(default)]
    pub lines_line_thickness: Option<f32>,
    #[serde(default)]
    pub lines_outline_thickness: Option<f32>,
    #[serde(default)]
    pub lines_outline_opacity: Option<f32>,
    #[serde(default)]
    pub lines_animation_speed: Option<f32>,
    #[serde(default)]
    pub lines_gradient_mode: Option<crate::viz_enums::GradientMode>,
    #[serde(default)]
    pub lines_fill_opacity: Option<f32>,
    #[serde(default)]
    pub lines_glow_intensity: Option<f32>,
    #[serde(default)]
    pub lines_mirror: Option<bool>,
    #[serde(default)]
    pub lines_style: Option<crate::viz_enums::LineStyle>,
    #[serde(default)]
    pub lines_trails: Option<f32>,
    #[serde(default)]
    pub lines_echo: Option<f32>,

    // ── Visualizer: Scope ─────────────────────────────────────────────────────
    #[serde(default)]
    pub scope_radius: Option<f32>,
    #[serde(default)]
    pub scope_sensitivity: Option<f32>,
    #[serde(default)]
    pub scope_point_count: Option<u32>,
    #[serde(default)]
    pub scope_line_thickness: Option<f32>,
    #[serde(default)]
    pub scope_fill_opacity: Option<f32>,
    #[serde(default)]
    pub scope_glow_intensity: Option<f32>,
    #[serde(default)]
    pub scope_outline_thickness: Option<f32>,
    #[serde(default)]
    pub scope_outline_opacity: Option<f32>,
    #[serde(default)]
    pub scope_gradient_mode: Option<crate::viz_enums::GradientMode>,
    #[serde(default)]
    pub scope_animation_speed: Option<f32>,
    #[serde(default)]
    pub scope_style: Option<crate::viz_enums::LineStyle>,
    #[serde(default)]
    pub scope_particles: Option<bool>,
    #[serde(default)]
    pub scope_particle_count: Option<u32>,
    #[serde(default)]
    pub scope_particle_speed: Option<f32>,
    #[serde(default)]
    pub scope_beam: Option<bool>,
    #[serde(default)]
    pub scope_trails: Option<f32>,
    #[serde(default)]
    pub scope_echo: Option<f32>,
}

fn config_path() -> PathBuf {
    crate::paths::config_dir().join("config.toml")
}

impl Config {
    pub fn load() -> Self {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(config_path(), s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_family_round_trips_through_toml() {
        let cfg = Config {
            font_family: Some("Inter".to_string()),
            ..Config::default()
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        let parsed: Config = toml::from_str(&s).unwrap();
        assert_eq!(parsed.font_family, Some("Inter".to_string()));
    }

    #[test]
    fn font_family_defaults_to_none_when_absent() {
        let parsed: Config = toml::from_str("").unwrap();
        assert_eq!(parsed.font_family, None);
    }
}
