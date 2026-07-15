use iced::widget::{column, container, row, slider, text};
use iced::{Alignment, Element, Length};

use crate::{icons, PlaybackState};

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn player_bar(&self) -> Element<'_, Message> {
        if self.ui_theme_id == "spotify" {
            self.player_bar_spotify()
        } else {
            self.player_bar_default()
        }
    }

    fn player_bar_default(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let song = if self.queue_idx >= 0 { self.queue.get(self.queue_idx as usize) } else { None };

        let cover = self.cover_image(song.and_then(|s| s.cover_art_id.as_deref()), 44.0);
        let title_text = song.map(|s| s.title.clone()).unwrap_or_else(|| "No track selected".to_string());
        let mut title_col = column![text(title_text).size(13).style(tstyle(t.text))].spacing(2);
        if let Some(s) = song {
            let subtitle = match &s.track_info {
                Some(info) if !info.is_empty() => format!("{} · {}", s.artist, info),
                _ => s.artist.clone(),
            };
            title_col = title_col.push(text(subtitle).size(11).style(tstyle(t.muted)));
        }
        let title_col = title_col.width(Length::Fill);
        let volume = row![
            icons::icon(icons::VOLUME, 16.0, t.muted),
            slider(0.0..=1.0, self.volume, Message::SetVolume).step(0.01_f32).width(Length::Fixed(55.0)).style(slider_style(t)),
            text(format!("{:.0}%", self.volume * 100.0)).size(10).style(tstyle(t.muted)).width(Length::Fixed(30.0)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        let left = container(row![cover, title_col, volume].spacing(10).align_y(Alignment::Center))
            .width(Length::Fixed(320.0));

        let dur = self.duration.unwrap_or(0.0).max(0.1) as f32;
        let pos = (self.position as f32).clamp(0.0, dur);
        let center = container(
            row![
                text(fmt_time(self.position)).size(11).style(tstyle(t.muted)),
                slider(0.0..=dur, pos, Message::SeekTo).step(0.5_f32).width(Length::Fill).style(slider_style(t)),
                text(fmt_time(self.duration.unwrap_or(0.0))).size(11).style(tstyle(t.muted)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill);

        let playing = matches!(self.playback_state, PlaybackState::Playing);
        let pp_icon = if playing { icons::PAUSE } else { icons::PLAY };
        let repeat_active = self.repeat_one || self.repeat_all;
        let shuffle_active = self.shuffle;
        let viz_active = self.right_panel == Some(Panel::Visualizer);
        let lyr_active = self.right_panel == Some(Panel::Lyrics);
        let q_active = self.right_panel == Some(Panel::Queue);
        let sim_active = self.right_panel == Some(Panel::Similar);
        let stats_active = self.right_panel == Some(Panel::AudioStats);
        let repeat_color = if repeat_active { t.accent } else { t.muted };
        let shuffle_color = if shuffle_active { t.accent } else { t.muted };
        let viz_color = if viz_active { t.accent } else { t.muted };
        let lyr_color = if lyr_active { t.accent } else { t.muted };
        let q_color = if q_active { t.accent } else { t.muted };
        let sim_color = if sim_active { t.accent } else { t.muted };
        let stats_color = if stats_active { t.accent } else { t.muted };

        let controls = row![
            with_tooltip(ctrl_button(icons::SHUFFLE, 15.0, shuffle_color, shuffle_active, t, Message::ToggleShuffle), "Shuffle", t),
            with_tooltip(ctrl_button(icons::PREV, 15.0, t.text, false, t, Message::Prev), "Previous", t),
            with_tooltip(main_ctrl_button(pp_icon, 20.0, t, false, Message::TogglePlay), if playing { "Pause" } else { "Play" }, t),
            with_tooltip(ctrl_button(icons::NEXT, 15.0, t.text, false, t, Message::Next), "Next", t),
            with_tooltip(ctrl_button(icons::REPEAT, 16.0, repeat_color, repeat_active, t, Message::CycleRepeat), "Repeat", t),
            ctrl_button(icons::LYRICS, 16.0, lyr_color, lyr_active, t, Message::TogglePanel(Panel::Lyrics)),
            ctrl_button(icons::QUEUE, 16.0, q_color, q_active, t, Message::TogglePanel(Panel::Queue)),
            ctrl_button(icons::HEXAGON, 16.0, sim_color, sim_active, t, Message::TogglePanel(Panel::Similar)),
            ctrl_button(icons::BAR_CHART, 16.0, stats_color, stats_active, t, Message::TogglePanel(Panel::AudioStats)),
            ctrl_button(icons::WAVEFORM, 16.0, viz_color, viz_active, t, Message::TogglePanel(Panel::Visualizer)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let right = container(controls).width(Length::Shrink);

        column![
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(fill_bg(t.border)),
            container(row![left, center, right].spacing(12).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fixed(60.0))
                .padding(iced::Padding { top: 8.0, right: 30.0, bottom: 8.0, left: 30.0 })
                .style(fill_bg(t.surface)),
        ]
        .into()
    }

    /// Spotify-style player bar: larger cover art, title/artist stacked on the left,
    /// transport controls centered above a full-width seek bar (rather than the
    /// default's inline time-slider-time row), extra panel toggles + volume on the right.
    fn player_bar_spotify(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let song = if self.queue_idx >= 0 { self.queue.get(self.queue_idx as usize) } else { None };

        let cover = self.cover_image(song.and_then(|s| s.cover_art_id.as_deref()), 56.0);
        let title_text = song.map(|s| s.title.clone()).unwrap_or_else(|| "No track selected".to_string());
        let artist_text = song.map(|s| s.artist.clone()).unwrap_or_default();
        let liked = song.is_some_and(|s| s.starred);
        let heart = if let Some(s) = song {
            let id = s.id.clone();
            icon_button(
                if liked { icons::HEART_FILLED } else { icons::HEART_OUTLINE },
                15.0,
                if liked { t.accent } else { t.muted },
                t,
                true,
                Message::ToggleStar(id, firmium_backend::commands::subsonic::StarKind::Song),
            )
        } else {
            icons::icon(icons::HEART_OUTLINE, 15.0, t.muted)
        };
        let title_col = column![
            text(title_text).size(14).style(tstyle(t.text)),
            row![text(artist_text).size(12).style(tstyle(t.muted)), heart]
                .spacing(10)
                .align_y(Alignment::Center),
        ]
        .spacing(2)
        .width(Length::Fixed(170.0));
        let left = container(row![cover, title_col].spacing(12).align_y(Alignment::Center))
            .width(Length::Fixed(280.0));

        let playing = matches!(self.playback_state, PlaybackState::Playing);
        let pp_icon = if playing { icons::PAUSE } else { icons::PLAY };
        let repeat_active = self.repeat_one || self.repeat_all;
        let shuffle_active = self.shuffle;
        let repeat_color = if repeat_active { t.accent } else { t.muted };
        let shuffle_color = if shuffle_active { t.accent } else { t.muted };

        let transport = row![
            with_tooltip(ctrl_button(icons::SHUFFLE, 15.0, shuffle_color, shuffle_active, t, Message::ToggleShuffle), "Shuffle", t),
            with_tooltip(ctrl_button(icons::PREV, 15.0, t.text, false, t, Message::Prev), "Previous", t),
            with_tooltip(main_ctrl_button(pp_icon, 20.0, t, true, Message::TogglePlay), if playing { "Pause" } else { "Play" }, t),
            with_tooltip(ctrl_button(icons::NEXT, 15.0, t.text, false, t, Message::Next), "Next", t),
            with_tooltip(ctrl_button(icons::REPEAT, 16.0, repeat_color, repeat_active, t, Message::CycleRepeat), "Repeat", t),
        ]
        .spacing(14)
        .align_y(Alignment::Center);

        let dur = self.duration.unwrap_or(0.0).max(0.1) as f32;
        let pos = (self.position as f32).clamp(0.0, dur);
        let seek = row![
            text(fmt_time(self.position)).size(11).style(tstyle(t.muted)),
            slider(0.0..=dur, pos, Message::SeekTo).step(0.5_f32).width(Length::Fill).style(slider_style(t)),
            text(fmt_time(self.duration.unwrap_or(0.0))).size(11).style(tstyle(t.muted)),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let center = container(column![transport, seek].spacing(6).align_x(Alignment::Center)).width(Length::Fill);

        let viz_active = self.right_panel == Some(Panel::Visualizer);
        let lyr_active = self.right_panel == Some(Panel::Lyrics);
        let q_active = self.right_panel == Some(Panel::Queue);
        let sim_active = self.right_panel == Some(Panel::Similar);
        let stats_active = self.right_panel == Some(Panel::AudioStats);
        let viz_color = if viz_active { t.accent } else { t.muted };
        let lyr_color = if lyr_active { t.accent } else { t.muted };
        let q_color = if q_active { t.accent } else { t.muted };
        let sim_color = if sim_active { t.accent } else { t.muted };
        let stats_color = if stats_active { t.accent } else { t.muted };

        let right_controls = row![
            ctrl_button(icons::LYRICS, 16.0, lyr_color, lyr_active, t, Message::TogglePanel(Panel::Lyrics)),
            ctrl_button(icons::QUEUE, 16.0, q_color, q_active, t, Message::TogglePanel(Panel::Queue)),
            ctrl_button(icons::HEXAGON, 16.0, sim_color, sim_active, t, Message::TogglePanel(Panel::Similar)),
            ctrl_button(icons::BAR_CHART, 16.0, stats_color, stats_active, t, Message::TogglePanel(Panel::AudioStats)),
            ctrl_button(icons::WAVEFORM, 16.0, viz_color, viz_active, t, Message::TogglePanel(Panel::Visualizer)),
            icons::icon(icons::VOLUME, 16.0, t.muted),
            text(format!("{:.0}%", self.volume * 100.0)).size(11).style(tstyle(t.muted)),
            slider(0.0..=1.0, self.volume, Message::SetVolume).step(0.01_f32).width(Length::Fixed(70.0)).style(slider_style(t)),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let right = container(right_controls).width(Length::Fixed(280.0)).align_x(Alignment::End);

        column![
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(fill_bg(t.border)),
            container(row![left, center, right].spacing(16).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fixed(78.0))
                .padding(iced::Padding { top: 10.0, right: 24.0, bottom: 10.0, left: 24.0 })
                .style(fill_bg(t.surface)),
        ]
        .into()
    }
}
