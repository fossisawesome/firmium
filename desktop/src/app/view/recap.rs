use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Length};

use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn recap_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let mut ranges = row![].spacing(6);
        for r in [
            RecapRange::Week,
            RecapRange::Month,
            RecapRange::ThreeMonths,
            RecapRange::Year,
            RecapRange::All,
        ] {
            let active = self.recap_range == r;
            ranges = ranges.push(
                button(text(r.label()).size(11).style(tstyle(if active { t.bg } else { t.text })))
                    .padding([6, 10])
                    .on_press(Message::SetRecapRange(r))
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
        let header = row![
            text("Firmium Recap").size(22).style(tstyle(t.text)).width(Length::Fill),
            ranges,
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match &self.recap {
            Some(stats) if stats.total_plays > 0 => {
                let idx = self.recap_card.min(RECAP_CARDS - 1);
                let card = container(self.recap_card_content(stats, idx))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(24)
                    .style(move |_th| container::Style {
                        background: Some(Background::Color(t.surface)),
                        border: Border { radius: 12.0.into(), width: 1.0, color: t.border },
                        ..container::Style::default()
                    });

                let mut dots = row![].spacing(6);
                for i in 0..RECAP_CARDS {
                    let c = if i == idx { t.accent } else { t.surface2 };
                    dots = dots.push(
                        container(text(""))
                            .width(Length::Fixed(8.0))
                            .height(Length::Fixed(8.0))
                            .style(move |_th| container::Style {
                                background: Some(Background::Color(c)),
                                border: Border { radius: 4.0.into(), ..Border::default() },
                                ..container::Style::default()
                            }),
                    );
                }
                let nav = row![
                    icon_button(icons::BACK, 16.0, t.text, t, spotify, Message::RecapPrev),
                    container(dots).center_x(Length::Fill),
                    icon_button(icons::CHEVRON_RIGHT, 16.0, t.text, t, spotify, Message::RecapNext),
                ]
                .align_y(Alignment::Center);

                column![card, nav].spacing(16).height(Length::Fill).into()
            }
            _ => container(
                text("No listening history yet — play some music and check back.")
                    .size(13)
                    .style(tstyle(t.muted)),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        };

        column![header, body].spacing(16).height(Length::Fill).into()
    }

    pub(crate) fn recap_card_content(&self, stats: &firmium_backend::db::RecapStats, idx: usize) -> Element<'_, Message> {
        let t = self.tokens;
        let label = |s: &'static str| text(s).size(12).style(tstyle(t.muted));
        match idx {
            0 => column![
                label("OVERVIEW"),
                text(format!("{} plays", stats.total_plays)).size(40).style(tstyle(t.accent)),
                text(format!("{} of listening", fmt_hours(stats.total_seconds))).size(16).style(tstyle(t.text)),
                text(format!("over the last {}", self.recap_range.label().to_lowercase()))
                    .size(12).style(tstyle(t.muted)),
            ]
            .spacing(10)
            .into(),
            1 => {
                let mut col = column![label("TOP TRACKS")].spacing(8);
                for (i, s) in stats.top_tracks.iter().take(5).enumerate() {
                    col = col.push(
                        row![
                            text(format!("{}", i + 1)).size(13).style(tstyle(t.muted)).width(Length::Fixed(20.0)),
                            self.cover_image(s.cover_art_id.as_deref(), 40.0),
                            column![
                                text(s.title.clone()).size(13).style(tstyle(t.text)),
                                text(s.artist.clone().unwrap_or_default()).size(11).style(tstyle(t.muted)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                            text(format!("{}×", s.count)).size(12).style(tstyle(t.accent)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
                col.into()
            }
            2 => {
                let mut col = column![label("TOP ARTISTS")].spacing(8);
                for (i, s) in stats.top_artists.iter().take(5).enumerate() {
                    col = col.push(
                        row![
                            text(format!("{}", i + 1)).size(13).style(tstyle(t.muted)).width(Length::Fixed(20.0)),
                            text(s.name.clone()).size(14).style(tstyle(t.text)).width(Length::Fill),
                            text(format!("{}×", s.count)).size(12).style(tstyle(t.accent)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
                col.into()
            }
            3 => {
                let mut col = column![label("TOP ALBUMS")].spacing(8);
                for (i, s) in stats.top_albums.iter().take(5).enumerate() {
                    col = col.push(
                        row![
                            text(format!("{}", i + 1)).size(13).style(tstyle(t.muted)).width(Length::Fixed(20.0)),
                            self.cover_image(s.cover_art_id.as_deref(), 40.0),
                            column![
                                text(s.name.clone()).size(13).style(tstyle(t.text)),
                                text(s.artist.clone().unwrap_or_default()).size(11).style(tstyle(t.muted)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                            text(format!("{}×", s.count)).size(12).style(tstyle(t.accent)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
                col.into()
            }
            4 => {
                let body: Element<'_, Message> = match &stats.top_genre {
                    Some(g) => column![
                        text(g.genre.clone()).size(32).style(tstyle(t.accent)),
                        text(format!("{} plays", g.count)).size(14).style(tstyle(t.muted)),
                    ]
                    .spacing(8)
                    .into(),
                    None => text("No genre data").size(13).style(tstyle(t.muted)).into(),
                };
                column![label("TOP GENRE"), body].spacing(12).into()
            }
            5 => {
                let tod = &stats.by_time_of_day;
                column![
                    label("TIME OF DAY"),
                    stat_row("Morning (5–11)", format!("{}", tod.morning), t),
                    stat_row("Afternoon (12–16)", format!("{}", tod.afternoon), t),
                    stat_row("Evening (17–20)", format!("{}", tod.evening), t),
                    stat_row("Night (21–4)", format!("{}", tod.night), t),
                ]
                .spacing(10)
                .into()
            }
            6 => {
                const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
                let mut col = column![label("DAY OF WEEK")].spacing(10);
                for (i, d) in DAYS.iter().enumerate() {
                    col = col.push(stat_row(d, format!("{}", stats.by_day_of_week[i]), t));
                }
                col.into()
            }
            7 => {
                let body: Element<'_, Message> = match &stats.biggest_discovery {
                    Some(d) => row![
                        self.cover_image(d.cover_art_id.as_deref(), 56.0),
                        column![
                            text(d.title.clone()).size(16).style(tstyle(t.text)),
                            text(d.artist.clone().unwrap_or_default()).size(12).style(tstyle(t.muted)),
                            text(format!("{} plays since you found it", d.count)).size(11).style(tstyle(t.accent)),
                        ]
                        .spacing(4),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .into(),
                    None => text("No standout discovery this period").size(13).style(tstyle(t.muted)).into(),
                };
                column![label("BIGGEST DISCOVERY"), body].spacing(12).into()
            }
            _ => column![
                label("STREAK"),
                text(format!("{} days active", stats.streak.days_active)).size(24).style(tstyle(t.text)),
                text(format!("Longest streak: {} days", stats.streak.longest_streak)).size(14).style(tstyle(t.accent)),
            ]
            .spacing(10)
            .into(),
        }
    }
}
