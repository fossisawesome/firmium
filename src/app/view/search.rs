use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Element, Length};

use crate::commands::mappers::Song;
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::App;

impl App {
    pub(crate) fn search_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let bar = row![
            text_input("Search your library…", &self.search_query)
                .on_input(Message::SearchInput)
                .on_submit(Message::SubmitSearch)
                .padding(10)
                .width(Length::Fill)
                .style(text_input_style(t)),
            button(text("Search").size(13))
                .on_press(Message::SubmitSearch)
                .padding(10)
                .style(primary_button(t)),
        ]
        .spacing(10);

        let results: Element<'_, Message> = if let Some(res) = &self.search_results {
            let mut col = column![].spacing(4);
            col = col.push(self.rating_filter_row());
            if !res.albums.is_empty() {
                col = col.push(text("Albums").size(13).style(tstyle(t.muted)));
                for a in res.albums.iter().take(40) {
                    col = col.push(self.album_row(a));
                }
            }
            let filter = self.search_rating_filter;
            let filtered_songs: Vec<&Song> = res
                .songs
                .iter()
                .filter(|s| {
                    filter == 0
                        || s.user_rating.unwrap_or(0) >= filter
                        || s.average_rating.unwrap_or(0.0) >= filter as f32
                })
                .take(100)
                .collect();
            if !filtered_songs.is_empty() {
                col = col.push(text("Songs").size(13).style(tstyle(t.muted)));
                for s in filtered_songs {
                    col = col.push(self.song_row(s));
                }
            }
            scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)).into()
        } else {
            text("Type a query and press Enter").size(12).style(tstyle(t.muted)).into()
        };

        column![page_header("Search", t, self.ui_theme_id == "spotify"), bar, results]
            .spacing(16)
            .into()
    }

    pub(crate) fn song_row(&self, song: &Song) -> Element<'_, Message> {
        let t = self.tokens;
        let is_current = self.current_song_id() == Some(song.id.as_str());
        let title_color = if is_current { t.accent } else { t.text };
        let play_area = button(
            row![
                self.cover_image(song.cover_art_id.as_deref(), 36.0),
                column![
                    text(song.title.clone()).size(13).style(tstyle(title_color)),
                    text(format!("{} · {}", song.artist, song.album)).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2)
                .width(Length::Fill),
                text(fmt_time(song.duration)).size(11).style(tstyle(t.muted)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::PlaySong(song.clone()))
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if h { Some(Background::Color(t.surface)) } else { None },
                text_color: t.text,
                border: Border { radius: 2.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });
        row![
            play_area,
            self.star_rating(song),
            self.avg_rating_badge(song),
            icon_button(icons::PLUS, 14.0, t.muted, t, Message::OpenAddToPlaylist(song.clone())),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    pub(crate) fn star_rating(&self, song: &Song) -> Element<'_, Message> {
        let t = self.tokens;
        let rating = song.user_rating.unwrap_or(0);
        let id = song.id.clone();
        let mut stars = row![].spacing(1);
        for i in 1..=5u32 {
            let filled = i <= rating;
            let src = if filled { icons::STAR_FILLED } else { icons::STAR_EMPTY };
            let color = if filled { t.accent } else { t.muted };
            let sid = id.clone();
            stars = stars.push(
                button(icons::icon(src, 12.0, color))
                    .padding(4)
                    .on_press(Message::SetRating(sid, i))
                    .style(move |_th, status| {
                        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        stars.into()
    }

    pub(crate) fn avg_rating_badge(&self, song: &Song) -> Element<'_, Message> {
        let t = self.tokens;
        match song.average_rating {
            Some(r) if r > 0.0 => row![
                icons::icon(icons::STAR_FILLED, 11.0, t.muted),
                text(format!("{r:.1}")).size(11).style(tstyle(t.muted)),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into(),
            _ => row![].into(),
        }
    }

    pub(crate) fn rating_filter_row(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let active = self.search_rating_filter;
        let mut stars = row![].spacing(1);
        for i in 1..=5u32 {
            let filled = i <= active;
            let src = if filled { icons::STAR_FILLED } else { icons::STAR_EMPTY };
            let color = if filled { t.accent } else { t.muted };
            stars = stars.push(
                button(icons::icon(src, 14.0, color))
                    .padding(4)
                    .on_press(Message::SetSearchRatingFilter(i))
                    .style(move |_th, status| {
                        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        row![
            text("Min rating:").size(12).style(tstyle(t.muted)),
            stars,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }
}
