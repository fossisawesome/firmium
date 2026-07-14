use iced::Task;

use super::super::message::Message;
use super::super::App;

impl App {
    pub(crate) fn update_visualizer(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetBarsMonstercat(v) => {
                self.bars_monstercat = v;
                self.bars_waves = false;
                let viz = self.backend.audio_player.visualizer();
                viz.set_monstercat(v);
                viz.set_waves(false, self.bars_waves_smoothing);
                self.save_config();
            }
            Message::SetBarsWaves(on) => {
                self.bars_waves = on;
                let viz = self.backend.audio_player.visualizer();
                if on {
                    viz.set_waves(true, self.bars_waves_smoothing);
                } else {
                    viz.set_monstercat(self.bars_monstercat);
                }
                self.save_config();
            }
            Message::SetBarsWavesSmoothing(v) => {
                self.bars_waves_smoothing = v.clamp(2, 16);
                if self.bars_waves {
                    self.backend.audio_player.visualizer().set_waves(true, self.bars_waves_smoothing);
                }
                self.save_config();
            }
            Message::SetBarsGradientMode(v) => {
                self.bars_gradient_mode = v;
                self.save_config();
            }
            Message::SetBarsGradientOrientation(v) => {
                self.bars_gradient_orientation = v;
                self.save_config();
            }
            Message::SetBarsPeakGradientMode(v) => {
                self.bars_peak_gradient_mode = v;
                self.save_config();
            }
            Message::SetBarsPeakMode(v) => {
                self.bars_peak_mode = v;
                self.save_config();
            }
            Message::SetBarsPeakHoldTime(v) => {
                self.bars_peak_hold_time = v;
                self.save_config();
            }
            Message::SetBarsPeakFadeTime(v) => {
                self.bars_peak_fade_time = v;
                self.save_config();
            }
            Message::SetBarsPeakHeight(v) => {
                self.bars_peak_height = v;
                self.save_config();
            }
            Message::SetBarsBorderWidth(v) => {
                self.bars_border_width = v;
                self.save_config();
            }
            Message::SetBarsLedBars(on) => {
                self.bars_led_bars = on;
                self.save_config();
            }
            Message::SetBarsLedSegmentHeight(v) => {
                self.bars_led_segment_height = v;
                self.save_config();
            }
            Message::SetBarsDepth3d(v) => {
                self.bars_depth_3d = v;
                self.save_config();
            }
            Message::SetBarsFlashIntensity(v) => {
                self.bars_flash_intensity = v;
                self.save_config();
            }
            Message::SetBarsMaxBars(v) => {
                self.bars_max_bars = v.clamp(16, 120);
                self.save_config();
            }
            Message::SetBarsTrails(v) => {
                self.bars_trails = v;
                self.save_config();
            }
            Message::SetBarsEcho(v) => {
                self.bars_echo = v;
                self.save_config();
            }

            Message::SetLinesPointCount(v) => {
                self.lines_point_count = v.clamp(8, 120);
                self.save_config();
            }
            Message::SetLinesLineThickness(v) => {
                self.lines_line_thickness = v;
                self.save_config();
            }
            Message::SetLinesOutlineThickness(v) => {
                self.lines_outline_thickness = v;
                self.save_config();
            }
            Message::SetLinesOutlineOpacity(v) => {
                self.lines_outline_opacity = v;
                self.save_config();
            }
            Message::SetLinesAnimationSpeed(v) => {
                self.lines_animation_speed = v;
                self.save_config();
            }
            Message::SetLinesGradientMode(v) => {
                self.lines_gradient_mode = v;
                self.save_config();
            }
            Message::SetLinesFillOpacity(v) => {
                self.lines_fill_opacity = v;
                self.save_config();
            }
            Message::SetLinesGlowIntensity(v) => {
                self.lines_glow_intensity = v;
                self.save_config();
            }
            Message::SetLinesMirror(on) => {
                self.lines_mirror = on;
                self.save_config();
            }
            Message::SetLinesStyle(v) => {
                self.lines_style = v;
                self.save_config();
            }
            Message::SetLinesTrails(v) => {
                self.lines_trails = v;
                self.save_config();
            }
            Message::SetLinesEcho(v) => {
                self.lines_echo = v;
                self.save_config();
            }

            Message::SetScopeRadius(v) => {
                self.scope_radius = v;
                self.save_config();
            }
            Message::SetScopeSensitivity(v) => {
                self.scope_sensitivity = v;
                self.save_config();
            }
            Message::SetScopePointCount(v) => {
                self.scope_point_count = v.clamp(16, 120);
                self.save_config();
            }
            Message::SetScopeLineThickness(v) => {
                self.scope_line_thickness = v;
                self.save_config();
            }
            Message::SetScopeFillOpacity(v) => {
                self.scope_fill_opacity = v;
                self.save_config();
            }
            Message::SetScopeGlowIntensity(v) => {
                self.scope_glow_intensity = v;
                self.save_config();
            }
            Message::SetScopeOutlineThickness(v) => {
                self.scope_outline_thickness = v;
                self.save_config();
            }
            Message::SetScopeOutlineOpacity(v) => {
                self.scope_outline_opacity = v;
                self.save_config();
            }
            Message::SetScopeGradientMode(v) => {
                self.scope_gradient_mode = v;
                self.save_config();
            }
            Message::SetScopeAnimationSpeed(v) => {
                self.scope_animation_speed = v;
                self.save_config();
            }
            Message::SetScopeStyle(v) => {
                self.scope_style = v;
                self.save_config();
            }
            Message::SetScopeParticles(on) => {
                self.scope_particles = on;
                self.save_config();
            }
            Message::SetScopeParticleCount(v) => {
                self.scope_particle_count = v.clamp(0, 2048);
                self.save_config();
            }
            Message::SetScopeParticleSpeed(v) => {
                self.scope_particle_speed = v;
                self.save_config();
            }
            Message::SetScopeBeam(on) => {
                self.scope_beam = on;
                self.save_config();
            }
            Message::SetScopeTrails(v) => {
                self.scope_trails = v;
                self.save_config();
            }
            Message::SetScopeEcho(v) => {
                self.scope_echo = v;
                self.save_config();
            }

            _ => {}
        }
        Task::none()
    }
}
