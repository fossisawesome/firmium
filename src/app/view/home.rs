use iced::widget::{button, column, row, scrollable, text};
use iced::{Background, Border, Element, Length};

use crate::commands::mappers::Album;

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn home_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let username = {
            let conn = self.backend.app_state.connection.read();
            conn.username.clone().unwrap_or_default()
        };
        scrollable(
            column![
                column![
                    text(format!("GOOD {},", time_of_day().to_uppercase()))
                        .size(13)
                        .style(tstyle(t.muted)),
                    text(username).size(36).style(tstyle(t.accent)).font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::MONOSPACE
                    }),
                ]
                .spacing(4),
                self.home_recent_songs_view(),
                self.home_recent_artists(),
                self.home_section("RANDOM PICKS", &self.home_random),
                self.home_genres(),
            ]
            .spacing(28)
            .padding(iced::Padding { right: 16.0, ..iced::Padding::ZERO }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(thin_scrollbar()))
        .style(thin_scroll_style(t))
        .into()
    }

    pub(crate) fn home_recent_songs_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        if self.home_recent_plays.is_empty() {
            return column![].into();
        }
        let mut cards = row![].spacing(12);
        for play in self.home_recent_plays.iter().take(5) {
            let artist = play.artist_name.clone().unwrap_or_default();
            let card_content = column![
                self.cover_image(play.cover_art_id.as_deref(), 130.0),
                text(play.track_title.clone()).size(12).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::MONOSPACE
                }),
                text(artist).size(11).style(tstyle(t.muted)),
            ]
            .spacing(6)
            .width(Length::Fixed(130.0));

            let card: Element<'_, Message> = if let Some(aid) = play.album_id.clone() {
                button(card_content)
                    .padding(4)
                    .on_press(Message::Navigate(View::AlbumDetail(aid)))
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if h { Some(Background::Color(t.surface)) } else { None },
                            text_color: t.text,
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    })
                    .into()
            } else {
                button(card_content)
                    .padding(4)
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if h { Some(Background::Color(t.surface)) } else { None },
                            text_color: t.text,
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    })
                    .into()
            };
            cards = cards.push(card);
        }
        column![
            text("RECENTLY PLAYED").size(11).style(tstyle(t.muted)),
            cards,
        ]
        .spacing(12)
        .into()
    }

    pub(crate) fn home_recent_artists(&self) -> Element<'_, Message> {
        let t = self.tokens;
        if self.home_recent_artists_cache.is_empty() {
            return column![].into();
        }

        let mut cards = row![].spacing(12);
        for (id, name, cover_art_id) in self.home_recent_artists_cache.iter().take(5) {
            let (id, name, cover_art_id) = (id.clone(), name.clone(), cover_art_id.clone());
            cards = cards.push(
                button(
                    column![
                        self.cover_image(cover_art_id.as_deref(), 130.0),
                        text(name).size(12).style(tstyle(t.text)).font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::MONOSPACE
                        }),
                    ]
                    .spacing(6)
                    .width(Length::Fixed(130.0)),
                )
                .padding(4)
                .on_press(Message::Navigate(View::ArtistDetail(id)))
                .style(move |_th, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: if h { Some(Background::Color(t.surface)) } else { None },
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                }),
            );
        }

        column![
            text("RECENTLY PLAYED ARTISTS").size(11).style(tstyle(t.muted)),
            cards,
        ]
        .spacing(12)
        .into()
    }

    pub(crate) fn home_genres(&self) -> Element<'_, Message> {
        let t = self.tokens;
        if self.genres.is_empty() {
            return column![].into();
        }
        let mut chips = row![].spacing(8);
        for g in self.genres.iter().take(30) {
            let name = g.name.clone();
            chips = chips.push(
                button(text(g.name.clone()).size(12).style(tstyle(t.text)))
                    .padding([6, 12])
                    .on_press(Message::Navigate(View::GenreDetail(name)))
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                            text_color: if h { t.text } else { t.muted },
                            border: Border { radius: 2.0.into(), color: t.border, width: 1.0 },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        column![
            text("GENRES").size(11).style(tstyle(t.muted)),
            chips,
        ]
        .spacing(10)
        .into()
    }

    pub(crate) fn home_section(&self, title: &'static str, albums: &[Album]) -> Element<'_, Message> {
        let t = self.tokens;
        if albums.is_empty() {
            return column![].into();
        }
        let mut cards = row![].spacing(12);
        for a in albums.iter().take(5) {
            cards = cards.push(self.album_card(a));
        }
        column![
            text(title).size(11).style(tstyle(t.muted)),
            cards,
        ]
        .spacing(12)
        .into()
    }

    pub(crate) fn album_card(&self, album: &Album) -> Element<'_, Message> {
        let t = self.tokens;
        button(
            column![
                self.cover_image(album.cover_art_id.as_deref(), 130.0),
                text(album.name.clone()).size(12).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::MONOSPACE
                }),
                text(album.album_artist.clone()).size(11).style(tstyle(t.muted)),
            ]
            .spacing(6)
            .width(Length::Fixed(130.0)),
        )
        .padding(4)
        .on_press(Message::Navigate(View::AlbumDetail(album.id.clone())))
        .style(move |_th, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if h { Some(Background::Color(t.surface)) } else { None },
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
    }
}
