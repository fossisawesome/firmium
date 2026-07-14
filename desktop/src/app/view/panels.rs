use iced::widget::{button, column, container, row, scrollable, slider, text, text_input};
use iced::{Alignment, Background, Border, Element, Length};

use crate::viz::{Visualizer, VizMode};
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn viz_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let mut modes = row![].spacing(6);
        for m in [VizMode::Bars, VizMode::Lines, VizMode::Scope] {
            let active = self.visualizer_mode == m;
            modes = modes.push(
                button(text(m.label()).size(11).style(tstyle(if active { t.bg } else { t.text })))
                    .padding(6)
                    .on_press(Message::SetVizMode(m))
                    .style(move |_th, status| {
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
                    }),
            );
        }
        let close = button(icons::icon(icons::CLOSE, 14.0, t.muted))
            .padding(6)
            .on_press(Message::TogglePanel(Panel::Visualizer))
            .style(move |_th, status| {
                let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if h { Some(Background::Color(t.surface2)) } else { None },
                    text_color: t.muted,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            });
        let header = row![
            text("VISUALIZER").size(11).style(tstyle(t.muted)).width(Length::Fill),
            modes,
            close,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let canvas = iced::widget::shader(Visualizer::new(
            self.backend.audio_player.visualizer(),
            self.visualizer_mode,
            self.viz_config(),
        ))
        .width(Length::Fill)
        .height(Length::Fill);

        container(column![header, canvas].spacing(12))
            .width(Length::Fixed(360.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    pub(crate) fn queue_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let header = row![
            text("QUEUE").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, spotify, Message::TogglePanel(Panel::Queue)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = if self.queue.is_empty() {
            text("Queue is empty").size(12).style(tstyle(t.muted)).into()
        } else {
            let mut list = column![].spacing(2);
            for (i, song) in self.queue.iter().enumerate() {
                let is_current = i as i32 == self.queue_idx;
                let tc = if is_current { t.accent } else { t.text };
                list = list.push(
                    button(
                        row![
                            self.cover_image(song.cover_art_id.as_deref(), 32.0),
                            column![
                                text(song.title.clone()).size(12).style(tstyle(tc)),
                                text(song.artist.clone()).size(10).style(tstyle(t.muted)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding(6)
                    .on_press(Message::PlayQueueIndex(i))
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if is_current {
                                Some(Background::Color(t.accent_dim))
                            } else if h {
                                Some(Background::Color(t.surface2))
                            } else {
                                None
                            },
                            text_color: t.text,
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
                );
            }
            scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)).into()
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(320.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    pub(crate) fn lyrics_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let header = row![
            text("LYRICS").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, spotify, Message::TogglePanel(Panel::Lyrics)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match &self.lyrics {
            None => text("Loading lyrics…").size(12).style(tstyle(t.muted)).into(),
            Some(lr) if lr.lines.is_empty() => {
                text("No lyrics available").size(12).style(tstyle(t.muted)).into()
            }
            Some(lr) => {
                let pos_ms = (self.position * 1000.0) as i64;
                let cur = if lr.synced {
                    lr.lines.iter().rposition(|l| l.start <= pos_ms)
                } else {
                    None
                };
                let mut col = column![].spacing(8);
                for (i, line) in lr.lines.iter().enumerate() {
                    let active = Some(i) == cur;
                    let value = if line.value.trim().is_empty() { "♪".to_string() } else { line.value.clone() };
                    // Karaoke word-fill: LRC only carries line-level timing, so
                    // approximate per-word progress by distributing the line's
                    // window evenly across its words.
                    if active && self.lyrics_word_fill && lr.synced && value.split_whitespace().next().is_some() {
                        let end = lr.lines.get(i + 1).map(|n| n.start).unwrap_or(line.start + 4000);
                        let span = (end - line.start).max(1) as f64;
                        let frac = ((pos_ms - line.start) as f64 / span).clamp(0.0, 1.0);
                        let words: Vec<&str> = value.split_whitespace().collect();
                        let filled = (frac * words.len() as f64).ceil() as usize;
                        let mut wr = row![].spacing(6);
                        for (wi, w) in words.iter().enumerate() {
                            let wc = if wi < filled { t.accent } else { t.muted };
                            wr = wr.push(text(w.to_string()).size(16).style(tstyle(wc)));
                        }
                        col = col.push(wr);
                    } else {
                        let (sz, c) = if active { (16.0_f32, t.accent) } else { (13.0_f32, t.muted) };
                        col = col.push(text(value).size(sz).style(tstyle(c)));
                    }
                }
                scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)).into()
            }
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(320.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    pub(crate) fn similar_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let header = row![
            text("SIMILAR TRACKS").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, spotify, Message::TogglePanel(Panel::Similar)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = if self.similar_results.is_empty() {
            text("No similar tracks found").size(12).style(tstyle(t.muted)).into()
        } else {
            let mut col = column![].spacing(2);
            for m in &self.similar_results {
                col = col.push(self.song_row(&m.song));
            }
            scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)).into()
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(320.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    pub(crate) fn audio_stats_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let header = row![
            text("AUDIO STATS").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, spotify, Message::TogglePanel(Panel::AudioStats)),
        ]
        .align_y(Alignment::Center);

        let song = if self.queue_idx >= 0 { self.queue.get(self.queue_idx as usize) } else { None };
        let body: Element<'_, Message> = match song {
            None => text("Nothing playing").size(12).style(tstyle(t.muted)).into(),
            Some(s) => {
                let mut col = column![].spacing(8);
                col = col.push(stat_row("Format", s.track_info.clone().unwrap_or_else(|| "—".to_string()), t));
                col = col.push(stat_row("BPM", s.bpm.map(|b| format!("{b:.0}")).unwrap_or_else(|| "—".to_string()), t));
                if let Some(sr) = s.sampling_rate {
                    col = col.push(stat_row("Sample rate", format!("{:.1} kHz", sr as f32 / 1000.0), t));
                }
                if let Some(bd) = s.bit_depth {
                    col = col.push(stat_row("Bit depth", format!("{bd}-bit"), t));
                }
                if let Some(br) = s.bit_rate {
                    col = col.push(stat_row("Bitrate", format!("{br} kbps"), t));
                }
                if let Some(rg) = &s.replay_gain {
                    if let Some(v) = rg.get("trackGain").and_then(|v| v.as_f64()) {
                        col = col.push(stat_row("Track gain", format!("{v:+.1} dB"), t));
                    }
                    if let Some(v) = rg.get("albumGain").and_then(|v| v.as_f64()) {
                        col = col.push(stat_row("Album gain", format!("{v:+.1} dB"), t));
                    }
                    if let Some(v) = rg.get("trackPeak").and_then(|v| v.as_f64()) {
                        col = col.push(stat_row("Track peak", format!("{v:.3}"), t));
                    }
                }
                scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)).into()
            }
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(300.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    pub(crate) fn eq_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let header = row![
            text("EQUALIZER").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, spotify, Message::TogglePanel(Panel::Equalizer)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match &self.eq_state {
            None => text("Loading…").size(12).style(tstyle(t.muted)).into(),
            Some(eq) => {
                let enabled = setting_toggle("Enabled", eq.enabled, Message::SetEqEnabled, t);

                let mut profs = column![].spacing(4);
                for p in &eq.profiles {
                    let active = eq.active_profile.as_deref() == Some(p.name.as_str());
                    let label = if p.imported {
                        format!("{} (imported)", p.name)
                    } else {
                        p.name.clone()
                    };
                    let name = p.name.clone();
                    profs = profs.push(
                        button(text(label).size(12).style(tstyle(if active { t.bg } else { t.text })))
                            .width(Length::Fill)
                            .padding(8)
                            .on_press(Message::SetEqProfile(name))
                            .style(move |_th, status| {
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
                            }),
                    );
                }

                let bands_ui: Element<'_, Message> = match eq
                    .active_profile
                    .as_ref()
                    .and_then(|a| eq.profiles.iter().find(|p| &p.name == a))
                {
                    Some(p) => {
                        let mut col = column![].spacing(8);
                        for (i, b) in p.bands.iter().enumerate() {
                            col = col.push(
                                row![
                                    text(fmt_freq(b.freq)).size(10).style(tstyle(t.muted)).width(Length::Fixed(46.0)),
                                    slider(-12.0..=12.0, b.gain, move |g| Message::EqBandChanged(i, g)).step(0.5_f32).width(Length::Fill).style(slider_style(t)),
                                    text(format!("{:+.1}", b.gain)).size(10).style(tstyle(t.muted)).width(Length::Fixed(40.0)),
                                ]
                                .spacing(8)
                                .align_y(Alignment::Center),
                            );
                        }
                        col.into()
                    }
                    None => text("Select a profile").size(12).style(tstyle(t.muted)).into(),
                };

                // Save the current bands as a new named profile.
                let save_row = row![
                    text_input("New profile name…", &self.eq_new_profile_name)
                        .on_input(Message::EqNewProfileInput)
                        .on_submit(Message::SaveEqProfile)
                        .padding(6)
                        .size(12)
                        .width(Length::Fill)
                        .style(text_input_style(t, spotify)),
                    button(text("Save").size(12).style(tstyle(t.bg)))
                        .padding(6)
                        .on_press(Message::SaveEqProfile)
                        .style(primary_button(t, spotify)),
                ]
                .spacing(6)
                .align_y(Alignment::Center);

                // Delete control for the active profile, unless it's read-only.
                let active_custom = eq
                    .active_profile
                    .as_ref()
                    .and_then(|a| eq.profiles.iter().find(|p| &p.name == a))
                    .filter(|p| !p.imported)
                    .map(|p| p.name.clone());
                let delete_row: Element<'_, Message> = match active_custom {
                    Some(name) => button(text(format!("Delete \"{name}\"")).size(12).style(tstyle(t.error)))
                        .padding(6)
                        .on_press(Message::DeleteEqProfile(name))
                        .style(move |_th, status| {
                            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                            button::Style {
                                background: if h { Some(Background::Color(t.surface)) } else { None },
                                text_color: t.error,
                                border: Border { color: t.error, width: 1.0, radius: 4.0.into() },
                                ..button::Style::default()
                            }
                        })
                        .into(),
                    None => container(text("")).into(),
                };

                column![
                    enabled,
                    section_label("PROFILES", t),
                    profs,
                    save_row,
                    delete_row,
                    section_label("BANDS", t),
                    bands_ui,
                ]
                .spacing(10)
                .into()
            }
        };

        container(column![header, scrollable(body).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t))].spacing(12))
            .width(Length::Fixed(340.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }
}
