use iced::widget::{button, column, container, pick_list, row, scrollable, slider, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::commands::themes::ThemeEntry;
use crate::fonts::FONT_OPTIONS;
use crate::theme::Tokens;
use crate::icons;
use crate::viz::config::{
    BarsGradientMode, BarsGradientOrientation, BarsPeakGradientMode, BarsPeakMode, GradientMode,
    LineStyle,
};

/// A settings row with a numeric slider + live value readout.
#[allow(clippy::too_many_arguments)] // thin positional wrapper used ~35 times below; a builder struct would add more noise than it removes
fn viz_num_row<'a>(
    label: &'static str,
    desc: &'static str,
    t: Tokens,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    suffix: &'static str,
    on_change: fn(f32) -> Message,
) -> Element<'a, Message> {
    sett_row(
        label,
        desc,
        t,
        row![
            slider(min..=max, value, on_change)
                .step(step)
                .width(Length::Fixed(150.0))
                .style(slider_style(t)),
            text(format!("{value:.3}{suffix}")).size(12).style(tstyle(t.text)).width(Length::Fixed(65.0)),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// A settings row with an integer slider + live value readout.
#[allow(clippy::too_many_arguments)] // thin positional wrapper used ~15 times below; a builder struct would add more noise than it removes
fn viz_int_row<'a>(
    label: &'static str,
    desc: &'static str,
    t: Tokens,
    min: u32,
    max: u32,
    step: u32,
    value: u32,
    suffix: &'static str,
    on_change: fn(u32) -> Message,
) -> Element<'a, Message> {
    sett_row(
        label,
        desc,
        t,
        row![
            slider(min..=max, value, on_change)
                .step(step)
                .width(Length::Fixed(150.0))
                .style(slider_style(t)),
            text(format!("{value}{suffix}")).size(12).style(tstyle(t.text)).width(Length::Fixed(50.0)),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// A settings row with a boolean toggle switch.
fn viz_bool_row<'a>(
    label: &'static str,
    desc: &'static str,
    t: Tokens,
    value: bool,
    on_change: fn(bool) -> Message,
) -> Element<'a, Message> {
    sett_row(label, desc, t, toggler(value).on_toggle(on_change).style(toggler_style(t)).into())
}

/// A settings row with a dropdown over a fixed set of enum options.
fn viz_enum_row<'a, E>(
    label: &'static str,
    desc: &'static str,
    t: Tokens,
    options: &'static [E],
    value: E,
    on_change: fn(E) -> Message,
) -> Element<'a, Message>
where
    E: Copy + PartialEq + std::fmt::Display + 'static,
{
    sett_row(
        label,
        desc,
        t,
        pick_list(options, Some(value), on_change).width(Length::Fixed(180.0)).into(),
    )
}

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn settings_view(&self) -> Element<'_, Message> {
        let t = self.tokens;

        // Left rail: category nav.
        let cats = [
            (SettingsCategory::Appearance, icons::PALETTE, "Appearance"),
            (SettingsCategory::Playback, icons::PLAY, "Playback"),
            (SettingsCategory::Visualizer, icons::WAVEFORM, "Visualizer"),
            (SettingsCategory::Equalizer, icons::EQUALIZER, "Equalizer"),
            (SettingsCategory::Downloads, icons::DOWNLOAD, "Downloads"),
            (SettingsCategory::Services, icons::GLOBE, "Services"),
            (SettingsCategory::Account, icons::USER, "Account"),
            (SettingsCategory::Debug, icons::INFO, "Debug"),
        ];
        let mut nav = column![text("SETTINGS").size(11).style(tstyle(t.muted))]
            .spacing(2)
            .padding([4, 8]);
        for (cat, icon_src, label_str) in cats {
            let active = self.settings_category == cat;
            nav = nav.push(
                button(
                    row![
                        icons::icon(icon_src, 16.0, if active { t.accent } else { t.muted }),
                        text(label_str).size(13).style(tstyle(if active { t.accent } else { t.text })),
                    ]
                    .spacing(9)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([7, 10])
                .on_press(Message::SetSettingsCategory(cat))
                .style(move |_theme, status| {
                    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if active {
                            t.accent_dim
                        } else if hovered {
                            t.surface
                        } else {
                            Color::TRANSPARENT
                        })),
                        text_color: if active { t.accent } else { t.text },
                        border: Border { radius: 6.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                }),
            );
        }
        let sidebar = container(nav)
            .width(Length::Fixed(180.0))
            .height(Length::Fill)
            .style(fill_bg(t.bg));

        let sep = container(text(""))
            .width(Length::Fixed(1.0))
            .height(Length::Fill)
            .style(fill_bg(t.border));

        let panel = scrollable(match self.settings_category {
            SettingsCategory::Appearance => self.settings_appearance(t),
            SettingsCategory::Playback => self.settings_playback(t),
            SettingsCategory::Visualizer => self.settings_visualizer(t),
            SettingsCategory::Equalizer => self.settings_equalizer(t),
            SettingsCategory::Downloads => self.settings_downloads(t),
            SettingsCategory::Services => self.settings_services(t),
            SettingsCategory::Account => self.settings_account(t),
            SettingsCategory::Debug => self.settings_debug(t),
        })
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(self.make_scrollbar()))
        .style(thin_scroll_style(t));

        row![sidebar, sep, container(panel).padding([0, 4]).width(Length::Fill)]
            .height(Length::Fill)
            .into()
    }

    pub(crate) fn settings_appearance(&self, t: Tokens) -> Element<'_, Message> {
        let selected = self.themes.iter().find(|e| e.id == self.theme_id).cloned();
        let theme_picker = pick_list(self.themes.clone(), selected, |entry: ThemeEntry| {
            Message::SelectTheme(entry.id)
        })
        .width(Length::Fixed(200.0))
        .into();
        let ui_theme_options = ["Default", "Spotify"];
        let ui_theme_selected = if self.ui_theme_id == "spotify" { "Spotify" } else { "Default" };
        let ui_theme_picker = pick_list(ui_theme_options, Some(ui_theme_selected), |label: &'static str| {
            Message::SelectUiTheme(if label == "Spotify" { "spotify".to_string() } else { "default".to_string() })
        })
        .width(Length::Fixed(200.0))
        .into();
        let font_selected = FONT_OPTIONS.iter().find(|f| **f == self.font_family.as_str()).copied();
        let font_picker = column![
            pick_list(FONT_OPTIONS, font_selected, |name: &str| {
                Message::SelectFont(name.to_string())
            })
            .width(Length::Fixed(200.0)),
            text("Restart to apply").size(11).style(tstyle(t.muted)),
        ]
        .spacing(4)
        .align_x(Alignment::End)
        .into();
        column![
            sett_panel_title("Appearance", t),
            sett_row(
                "Window Decorations",
                "Show native title bar and borders",
                t,
                toggler(self.window_decorations).on_toggle(Message::SetDecorations).style(toggler_style(t)).into(),
            ),
            sett_row(
                "Cover-Colored Visualizer",
                "Tint the visualizer with the current album's artwork. When off, it follows your theme colors.",
                t,
                toggler(self.viz_cover_colors).on_toggle(Message::SetVizCoverColors).style(toggler_style(t)).into(),
            ),
            sett_row("Theme", "Color scheme for the interface", t, theme_picker),
            sett_row("UI Theme", "Layout style: nav, player bar, and screen structure", t, ui_theme_picker),
            sett_row("Font", "Interface font, applies after restart", t, font_picker),
            sett_row(
                "Scrollbar Width",
                "Adjust scrollbar thickness (6-20px)",
                t,
                row![
                    slider(6.0..=20.0, self.scrollbar_width as f32, |v| {
                        Message::SetScrollbarWidth(v as u32)
                    })
                    .width(Length::Fixed(150.0))
                    .style(slider_style(t)),
                    text(format!("{} px", self.scrollbar_width))
                        .size(12)
                        .style(tstyle(t.text))
                        .width(Length::Fixed(50.0)),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
            ),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_playback(&self, t: Tokens) -> Element<'_, Message> {
        let bp = |label: &'static str, mode: &'static str| -> Element<'_, Message> {
            let active = self.bit_perfect_mode == mode;
            button(text(label).size(12).style(tstyle(if active { t.bg } else { t.text })))
                .padding(8)
                .on_press(Message::SetBitPerfect(mode.to_string()))
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if active {
                            t.accent
                        } else if h {
                            t.surface
                        } else {
                            t.surface2
                        })),
                        text_color: if active { t.bg } else { t.text },
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        let crossfade_dur: Element<'_, Message> = if self.crossfade_enabled {
            sett_row(
                "Crossfade Duration",
                "Length of the blend in seconds",
                t,
                row![
                    slider(1.0..=12.0, self.crossfade_duration, Message::SetCrossfadeDuration)
                        .step(1.0)
                        .width(Length::Fixed(100.0))
                        .style(slider_style(t)),
                    text(format!("{:.0}s", self.crossfade_duration)).size(12).style(tstyle(t.muted)),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            )
        } else {
            column![].into()
        };
        column![
            sett_panel_title("Playback", t),
            sett_row("Crossfade", "Smoothly blend between tracks", t,
                toggler(self.crossfade_enabled).on_toggle(Message::SetCrossfadeEnabled).style(toggler_style(t)).into()),
            crossfade_dur,
            sett_row("Gapless Playback", "Pre-buffer the next track for seamless transitions", t,
                toggler(self.gapless_enabled).on_toggle(Message::SetGapless).style(toggler_style(t)).into()),
            sett_row("ReplayGain", "Normalize track loudness using server-provided gain values", t,
                toggler(self.replay_gain_enabled).on_toggle(Message::SetReplayGain).style(toggler_style(t)).into()),
            sett_row("Continue playing after queue ends", "Smart Radio keeps the music going by adding similar tracks when the queue runs out", t,
                toggler(self.auto_continue).on_toggle(Message::SetAutoContinue).style(toggler_style(t)).into()),
            sett_row("Bit-Perfect Audio", "Matches native sample rate; crossfade is disabled", t,
                row![bp("Off", "off"), bp("Relaxed", "relaxed"), bp("Strict", "strict")].spacing(4).into()),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_visualizer(&self, t: Tokens) -> Element<'_, Message> {
        column![
            sett_panel_title("Bars", t),
            viz_bool_row(
                "Waves Smoothing",
                "Catmull-Rom spline smoothing across bars for smooth rolling hills. Mutually exclusive with Monstercat.",
                t, self.bars_waves, Message::SetBarsWaves,
            ),
            viz_int_row(
                "Waves Intensity",
                "Control-point spacing for the spline. Higher = smoother (fewer control points).",
                t, 2, 16, 1, self.bars_waves_smoothing, "", Message::SetBarsWavesSmoothing,
            ),
            viz_num_row(
                "Monstercat Smoothing",
                "Spreads each bar's energy into its neighbors via exponential decay. Lower = wider, smoother spread; higher = sharper, narrower peaks. Mutually exclusive with Waves.",
                t, 0.0, 10.0, 0.1, self.bars_monstercat, "", Message::SetBarsMonstercat,
            ),
            viz_int_row(
                "Max Bar Count",
                "Maximum number of bars to fit in the window.",
                t, 16, 120, 4, self.bars_max_bars, "", Message::SetBarsMaxBars,
            ),
            viz_num_row(
                "Border Width",
                "Outline around each bar; also sets LED gap size.",
                t, 0.0, 5.0, 1.0, self.bars_border_width, " px", Message::SetBarsBorderWidth,
            ),
            viz_bool_row(
                "LED Mode",
                "Render bars as stacked LED segments like a VU meter.",
                t, self.bars_led_bars, Message::SetBarsLedBars,
            ),
            viz_num_row(
                "LED Segment Height",
                "Height of each LED segment in pixels.",
                t, 2.0, 20.0, 1.0, self.bars_led_segment_height, " px", Message::SetBarsLedSegmentHeight,
            ),
            viz_enum_row(
                "Gradient Mode",
                "static: height-based gradient. wave: gradient stretches with bar height.",
                t, &BarsGradientMode::ALL, self.bars_gradient_mode, Message::SetBarsGradientMode,
            ),
            viz_enum_row(
                "Gradient Orientation",
                "vertical: colors map bottom-to-top. horizontal: colors map across bars (bass to treble).",
                t, &BarsGradientOrientation::ALL, self.bars_gradient_orientation, Message::SetBarsGradientOrientation,
            ),
            viz_enum_row(
                "Peak Gradient Mode",
                "Color mode for peak indicators.",
                t, &BarsPeakGradientMode::ALL, self.bars_peak_gradient_mode, Message::SetBarsPeakGradientMode,
            ),
            viz_enum_row(
                "Peak Mode",
                "Falloff behavior for peak indicators after a hold.",
                t, &BarsPeakMode::ALL, self.bars_peak_mode, Message::SetBarsPeakMode,
            ),
            viz_num_row(
                "Peak Hold Time",
                "How long peaks stay before falling/fading.",
                t, 0.0, 5.0, 0.1, self.bars_peak_hold_time, "s", Message::SetBarsPeakHoldTime,
            ),
            viz_num_row(
                "Peak Fade Time",
                "Duration of the fade-out in Fade/Fall+fade modes.",
                t, 0.0, 5.0, 0.1, self.bars_peak_fade_time, "s", Message::SetBarsPeakFadeTime,
            ),
            viz_num_row(
                "Peak Height",
                "Peak bar size as a fraction of bar width (ignored in LED mode).",
                t, 0.1, 1.0, 0.05, self.bars_peak_height, "", Message::SetBarsPeakHeight,
            ),
            viz_num_row(
                "Isometric Depth",
                "3D top/side face depth in pixels, 0 = flat.",
                t, 0.0, 20.0, 1.0, self.bars_depth_3d, " px", Message::SetBarsDepth3d,
            ),
            viz_num_row(
                "Peak Flash",
                "Bars bloom toward the peak color on a beat. 0 = disabled.",
                t, 0.0, 1.0, 0.05, self.bars_flash_intensity, "", Message::SetBarsFlashIntensity,
            ),
            viz_num_row(
                "Motion Trails",
                "Bars leave a fading after-image. 0 = off, 1 = long comet trails.",
                t, 0.0, 1.0, 0.05, self.bars_trails, "", Message::SetBarsTrails,
            ),
            viz_num_row(
                "Echo",
                "Milkdrop feedback: bars spiral and tunnel into themselves with the beat. 0 = off.",
                t, 0.0, 1.0, 0.05, self.bars_echo, "", Message::SetBarsEcho,
            ),

            sett_panel_title("Lines", t),
            viz_int_row(
                "Point Count",
                "More points = finer waveform detail.",
                t, 8, 120, 8, self.lines_point_count, "", Message::SetLinesPointCount,
            ),
            viz_num_row(
                "Line Thickness",
                "Stroke thickness in pixels.",
                t, 0.5, 8.0, 0.5, self.lines_line_thickness, " px", Message::SetLinesLineThickness,
            ),
            viz_num_row(
                "Outline Thickness",
                "Border behind the line in pixels, 0 = disabled.",
                t, 0.0, 5.0, 0.5, self.lines_outline_thickness, " px", Message::SetLinesOutlineThickness,
            ),
            viz_num_row(
                "Outline Opacity",
                "0.0 = invisible, 1.0 = fully opaque.",
                t, 0.0, 1.0, 0.1, self.lines_outline_opacity, "", Message::SetLinesOutlineOpacity,
            ),
            viz_num_row(
                "Animation Speed",
                "Color cycling speed for the Breathing gradient mode.",
                t, 0.05, 1.0, 0.05, self.lines_animation_speed, "", Message::SetLinesAnimationSpeed,
            ),
            viz_enum_row(
                "Gradient Mode",
                "How gradient colors are mapped across the line.",
                t, &GradientMode::ALL, self.lines_gradient_mode, Message::SetLinesGradientMode,
            ),
            viz_num_row(
                "Fill Opacity",
                "Fills under the curve with a gradient. 0 = disabled.",
                t, 0.0, 1.0, 0.05, self.lines_fill_opacity, "", Message::SetLinesFillOpacity,
            ),
            viz_num_row(
                "Glow Intensity",
                "Neon halo around the line. 0 = disabled, brightens with loudness.",
                t, 0.0, 1.0, 0.05, self.lines_glow_intensity, "", Message::SetLinesGlowIntensity,
            ),
            viz_bool_row(
                "Mirror",
                "Symmetric oscilloscope — line extends from center.",
                t, self.lines_mirror, Message::SetLinesMirror,
            ),
            viz_enum_row(
                "Line Style",
                "Interpolation between data points.",
                t, &LineStyle::ALL, self.lines_style, Message::SetLinesStyle,
            ),
            viz_num_row(
                "Motion Trails",
                "The line leaves a fading after-image. 0 = off, 1 = long comet trails.",
                t, 0.0, 1.0, 0.05, self.lines_trails, "", Message::SetLinesTrails,
            ),
            viz_num_row(
                "Echo",
                "Milkdrop feedback: the line spirals and tunnels into itself with the beat. 0 = off.",
                t, 0.0, 1.0, 0.05, self.lines_echo, "", Message::SetLinesEcho,
            ),

            sett_panel_title("Scope", t),
            viz_num_row(
                "Ring Size",
                "Mean ring radius over the cover. 0.1 = small inner ring, 0.95 = nearly fills the panel.",
                t, 0.1, 0.95, 0.05, self.scope_radius, "", Message::SetScopeRadius,
            ),
            viz_num_row(
                "Sensitivity",
                "How hard loud audio swings the ring in and out.",
                t, 0.5, 5.0, 0.1, self.scope_sensitivity, "\u{d7}", Message::SetScopeSensitivity,
            ),
            viz_int_row(
                "Point Count",
                "Points around the ring. Lower = chunkier, higher = finer waveform.",
                t, 16, 120, 8, self.scope_point_count, "", Message::SetScopePointCount,
            ),
            viz_num_row(
                "Line Thickness",
                "Ring stroke as a fraction of panel size.",
                t, 0.005, 0.1, 0.005, self.scope_line_thickness, "", Message::SetScopeLineThickness,
            ),
            viz_num_row(
                "Fill Opacity",
                "Radial gradient fill from the ring toward the center. 0 = outline only, 1 = solid rim.",
                t, 0.0, 1.0, 0.05, self.scope_fill_opacity, "", Message::SetScopeFillOpacity,
            ),
            viz_num_row(
                "Glow Intensity",
                "Neon halo around the ring. 0 = disabled, brightens with loudness.",
                t, 0.0, 1.0, 0.05, self.scope_glow_intensity, "", Message::SetScopeGlowIntensity,
            ),
            viz_num_row(
                "Outline Thickness",
                "Darker border behind the ring in pixels, 0 = disabled.",
                t, 0.0, 5.0, 0.5, self.scope_outline_thickness, " px", Message::SetScopeOutlineThickness,
            ),
            viz_num_row(
                "Outline Opacity",
                "0.0 = invisible, 1.0 = fully opaque.",
                t, 0.0, 1.0, 0.1, self.scope_outline_opacity, "", Message::SetScopeOutlineOpacity,
            ),
            viz_enum_row(
                "Gradient Mode",
                "How gradient colors are mapped around the ring.",
                t, &GradientMode::ALL, self.scope_gradient_mode, Message::SetScopeGradientMode,
            ),
            viz_num_row(
                "Animation Speed",
                "Color cycling speed for the Breathing gradient mode.",
                t, 0.05, 1.0, 0.05, self.scope_animation_speed, "", Message::SetScopeAnimationSpeed,
            ),
            viz_enum_row(
                "Line Style",
                "Interpolation around the ring.",
                t, &LineStyle::ALL, self.scope_style, Message::SetScopeStyle,
            ),
            viz_bool_row(
                "Particles",
                "Glowing particles drifting out from the ring (NCS-style).",
                t, self.scope_particles, Message::SetScopeParticles,
            ),
            viz_int_row(
                "Particle Count",
                "How many particles fill the field. 0 = none, 2048 = dense.",
                t, 0, 2048, 64, self.scope_particle_count, "", Message::SetScopeParticleCount,
            ),
            viz_num_row(
                "Particle Speed",
                "How fast particles fly out. Lower = lazy drift, higher = energetic.",
                t, 0.1, 4.0, 0.1, self.scope_particle_speed, "\u{d7}", Message::SetScopeParticleSpeed,
            ),
            viz_bool_row(
                "Beam Glow",
                "Additive luminous beam (woscope-style) — the ring glows brighter over the cover. Pair with Glow.",
                t, self.scope_beam, Message::SetScopeBeam,
            ),
            viz_num_row(
                "Motion Trails",
                "The ring leaves a fading after-image. 0 = off, 1 = long comet trails.",
                t, 0.0, 1.0, 0.05, self.scope_trails, "", Message::SetScopeTrails,
            ),
            viz_num_row(
                "Echo",
                "Milkdrop feedback: the ring spirals and tunnels inward with the beat. 0 = off.",
                t, 0.0, 1.0, 0.05, self.scope_echo, "", Message::SetScopeEcho,
            ),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_equalizer(&self, t: Tokens) -> Element<'_, Message> {
        column![
            sett_panel_title("Equalizer", t),
            sett_row(
                "Graphic Equalizer",
                "Open the multi-band EQ in the side panel",
                t,
                button(text("Open Equalizer").size(13).style(tstyle(t.text)))
                    .padding(10)
                    .on_press(Message::TogglePanel(Panel::Equalizer))
                    .style(move |_t, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                            text_color: t.text,
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    })
                    .into(),
            ),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_downloads(&self, t: Tokens) -> Element<'_, Message> {
        fn fmt_label(id: &str) -> &'static str {
            match id {
                "mp3" => "MP3",
                "flac" => "FLAC",
                "wav" => "WAV",
                "opus" => "Opus",
                _ => "Original",
            }
        }
        let selected = fmt_label(&self.download_format);
        let fmt_picker = pick_list(
            ["Original", "MP3", "FLAC", "WAV", "Opus"],
            Some(selected),
            |label: &'static str| {
                let id = match label {
                    "MP3" => "mp3",
                    "FLAC" => "flac",
                    "WAV" => "wav",
                    "Opus" => "opus",
                    _ => "raw",
                };
                Message::SetDownloadFormat(id.to_string())
            },
        )
        .width(Length::Fixed(200.0))
        .into();
        column![
            sett_panel_title("Downloads", t),
            sett_row(
                "Download Format",
                "Format used when downloading tracks and albums. \"Original\" saves the file exactly as stored on the server.",
                t,
                fmt_picker,
            ),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_services(&self, t: Tokens) -> Element<'_, Message> {
        let mut col = column![sett_panel_title("Services", t)].spacing(0);
        col = col.push(sett_row(
            "Last.fm Integration",
            "Fetch richer artist bio and photo using your own Last.fm API key",
            t,
            toggler(self.lastfm_enabled).on_toggle(Message::SetLastfmEnabled).style(toggler_style(t)).into(),
        ));
        if self.lastfm_enabled {
            col = col.push(sett_row(
                "Last.fm API Key",
                "From your Last.fm API account",
                t,
                text_input("API key…", &self.lastfm_key)
                    .on_input(Message::SetLastfmKey)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .style(text_input_style(t))
                    .into(),
            ));
            col = col.push(sett_row(
                "Last.fm Secret",
                "Shared secret for your API account",
                t,
                text_input("Secret…", &self.lastfm_secret)
                    .on_input(Message::SetLastfmSecret)
                    .secure(true)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .style(text_input_style(t))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "ListenBrainz Scrobbling",
            "Submit each completed track to ListenBrainz using your user token",
            t,
            toggler(self.listenbrainz_enabled).on_toggle(Message::SetListenbrainzEnabled).style(toggler_style(t)).into(),
        ));
        if self.listenbrainz_enabled {
            col = col.push(sett_row(
                "ListenBrainz Token",
                "From your ListenBrainz profile settings",
                t,
                text_input("User token…", &self.listenbrainz_token)
                    .on_input(Message::SetListenbrainzToken)
                    .secure(true)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .style(text_input_style(t))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "External Lyrics (LRCLIB)",
            "Fetch synced lyrics from lrclib.net when your server has none. Sends song title and artist name.",
            t,
            toggler(self.lrclib_enabled).on_toggle(Message::SetLrclibEnabled).style(toggler_style(t)).into(),
        ));
        col = col.push(sett_row(
            "Word-by-Word Lyrics Animation",
            "Karaoke-style fill on the active lyric line, with per-word timing estimated from the line's timestamps. Disable for plain line-by-line highlighting.",
            t,
            toggler(self.lyrics_word_fill).on_toggle(Message::SetLyricsWordFill).style(toggler_style(t)).into(),
        ));
        col.into()
    }

    pub(crate) fn settings_account(&self, t: Tokens) -> Element<'_, Message> {
        let (server, username) = {
            let conn = self.backend.app_state.connection.read();
            (conn.server.clone().unwrap_or_default(), conn.username.clone().unwrap_or_default())
        };
        let conn_desc = if self.authed {
            format!("{username} @ {server}")
        } else {
            "Not connected".to_string()
        };
        let conn_btn: Element<'_, Message> = if self.authed {
            button(text("Log out").size(13).style(tstyle(t.error)))
                .padding(10)
                .on_press(Message::Logout)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: if h { Some(Background::Color(t.surface)) } else { None },
                        text_color: t.error,
                        border: Border { color: t.error, width: 1.0, radius: 4.0.into() },
                        ..button::Style::default()
                    }
                })
                .into()
        } else {
            button(text("Connect").size(13).style(tstyle(t.text)))
                .padding(10)
                .on_press(Message::ToggleAccountSwitcher)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };

        let sec_btn = |label: &'static str, msg: Message| -> Element<'_, Message> {
            button(text(label).size(13).style(tstyle(t.text)))
                .padding(10)
                .on_press(msg)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        let stats_section: Element<'_, Message> = match &self.history_summary {
            Some(s) if s.total_plays > 0 => column![
                stat_row("Total plays", s.total_plays.to_string(), t),
                stat_row("Listening time", fmt_hours(s.total_seconds), t),
                stat_row("Unique tracks", s.unique_tracks.to_string(), t),
                stat_row("Unique artists", s.unique_artists.to_string(), t),
                stat_row("Unique albums", s.unique_albums.to_string(), t),
                row![
                    sec_btn("Export CSV", Message::ExportStats("csv".to_string())),
                    sec_btn("Export JSON", Message::ExportStats("json".to_string())),
                    sec_btn("View Recap", Message::Navigate(View::Recap)),
                ]
                .spacing(8),
            ]
            .spacing(10)
            .into(),
            _ => text("No play history yet — start listening to build your stats.")
                .size(12)
                .style(tstyle(t.muted))
                .into(),
        };

        column![
            sett_panel_title("Account", t),
            sett_row("Connection", conn_desc, t, conn_btn),
            sett_panel_title("Listening Stats", t),
            container(stats_section).padding([15, 10]),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_debug(&self, t: Tokens) -> Element<'_, Message> {
        let version = crate::commands::app_info::get_app_version();
        let debug_btn = |label: &'static str, msg: Message, danger: bool| -> Element<'_, Message> {
            button(text(label).size(13).style(tstyle(if danger { t.error } else { t.text })))
                .padding(10)
                .on_press(msg)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: if danger { t.error } else { t.text },
                        border: Border {
                            color: if danger { t.error } else { Color::TRANSPARENT },
                            width: if danger { 1.0 } else { 0.0 },
                            radius: 4.0.into(),
                        },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        column![
            sett_panel_title("Debug", t),
            sett_row("App Version", version, t, text("").into()),
            sett_row("Wipe Cache", "Clear cached cover art", t,
                debug_btn("Wipe", Message::WipeCoverCache, false)),
            sett_row("Delete Settings", "Reset all preferences to defaults", t,
                debug_btn("Delete", Message::DeleteSettings, true)),
        ]
        .spacing(0)
        .into()
    }
}
