use iced::Task;

use super::cover::{ramp8, rgb_to_color};
use super::message::Message;
use super::types::Panel;
use super::App;

impl App {
    /// Build the visualizer config, overriding its gradient with either the
    /// current cover-art palette or the active theme's colors.
    pub(crate) fn viz_config(&self) -> crate::viz::VizConfig {
        let gradient_colors = match (self.viz_cover_colors, &self.viz_palette) {
            (true, Some(p)) => ramp8(
                rgb_to_color(p.primary),
                rgb_to_color(p.secondary),
                rgb_to_color(p.tertiary),
            ),
            _ => self.theme_gradient(),
        };
        crate::viz::VizConfig {
            gradient_colors,
            gradient_mode: self.bars_gradient_mode.as_u32(),
            gradient_orientation: self.bars_gradient_orientation.as_u32(),
            peak_gradient_mode: self.bars_peak_gradient_mode.as_u32(),
            peak_mode: self.bars_peak_mode.as_u32(),
            peak_hold_time: self.bars_peak_hold_time,
            peak_fade_time: self.bars_peak_fade_time,
            peak_thickness: self.bars_peak_height,
            border_width: self.bars_border_width,
            led_bars: self.bars_led_bars,
            led_segment_height: self.bars_led_segment_height,
            bar_depth_3d: self.bars_depth_3d,
            bars_flash_intensity: self.bars_flash_intensity,
            bars_max_bars: self.bars_max_bars,
            bars_trails: self.bars_trails,
            bars_echo: self.bars_echo,

            lines_point_count: self.lines_point_count,
            line_thickness: self.lines_line_thickness,
            lines_outline_thickness: self.lines_outline_thickness,
            lines_outline_opacity: self.lines_outline_opacity,
            lines_animation_speed: self.lines_animation_speed,
            lines_gradient_mode: self.lines_gradient_mode.as_u32(),
            lines_fill_opacity: self.lines_fill_opacity,
            lines_glow_intensity: self.lines_glow_intensity,
            lines_mirror: self.lines_mirror,
            lines_style: self.lines_style.as_u32(),
            lines_trails: self.lines_trails,
            lines_echo: self.lines_echo,

            scope_radius: self.scope_radius,
            scope_sensitivity: self.scope_sensitivity,
            scope_point_count: self.scope_point_count,
            scope_line_thickness: self.scope_line_thickness,
            scope_fill_opacity: self.scope_fill_opacity,
            scope_glow_intensity: self.scope_glow_intensity,
            scope_outline_thickness: self.scope_outline_thickness,
            scope_outline_opacity: self.scope_outline_opacity,
            scope_gradient_mode: self.scope_gradient_mode.as_u32(),
            scope_animation_speed: self.scope_animation_speed,
            scope_style: self.scope_style.as_u32(),
            scope_particles: self.scope_particles,
            scope_particle_count: self.scope_particle_count,
            scope_particle_speed: self.scope_particle_speed,
            scope_beam: self.scope_beam,
            scope_trails: self.scope_trails,
            scope_echo: self.scope_echo,

            ..crate::viz::VizConfig::default()
        }
    }

    /// An 8-stop gradient derived from the active theme's accent colors, used
    /// when cover coloring is off or no cover palette is available.
    pub(crate) fn theme_gradient(&self) -> Vec<iced::Color> {
        let t = self.tokens;
        let lighten = |c: iced::Color, k: f32| iced::Color {
            r: c.r + (1.0 - c.r) * k,
            g: c.g + (1.0 - c.g) * k,
            b: c.b + (1.0 - c.b) * k,
            a: c.a,
        };
        ramp8(t.accent, lighten(t.accent, 0.35), t.accent_dim)
    }

    /// Extract the cover-art palette for the current track when the Visualizer
    /// panel is open, the cover-color option is on, and the track changed.
    pub(crate) fn maybe_fetch_viz_colors(&mut self) -> Task<Message> {
        if self.right_panel != Some(Panel::Visualizer) || !self.viz_cover_colors {
            return Task::none();
        }
        let song = if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).cloned()
        } else {
            None
        };
        let Some(song) = song else {
            self.viz_palette = None;
            self.viz_palette_track = None;
            return Task::none();
        };
        if self.viz_palette_track.as_deref() == Some(song.id.as_str()) {
            return Task::none();
        }
        self.viz_palette_track = Some(song.id.clone());
        // No cover → fall back to the theme gradient (handled in viz_config).
        let Some(cover_id) = song.cover_art_id.clone() else {
            self.viz_palette = None;
            return Task::none();
        };
        let Ok(url) = crate::commands::subsonic::build_cover_url(&self.backend.app_state, &cover_id, 300) else {
            return Task::none();
        };
        let track_id = song.id.clone();
        Task::perform(
            crate::commands::cover_colors::extract_cover_colors(cover_id, url),
            move |res| Message::VizColorsLoaded(track_id.clone(), res),
        )
    }
}
