use iced::widget::{button, column, responsive, row, scrollable, text};
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
        .direction(scrollable::Direction::Vertical(self.make_scrollbar()))
        .style(thin_scroll_style(t))
        .into()
    }

    pub(crate) fn home_recent_songs_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        if self.home_recent_plays.is_empty() {
            return column![].into();
        }
        let spotify = self.ui_theme_id == "spotify";
        let mut cards = row![].spacing(if spotify { 16 } else { 12 });
        for play in self.home_recent_plays.iter().take(5) {
            let artist = play.artist_name.clone().unwrap_or_default();
            let mut title_text = text(play.track_title.clone()).size(if spotify { 13 } else { 12 }).style(tstyle(t.text));
            title_text = title_text.font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            });
            let card_content = column![
                self.cover_image(play.cover_art_id.as_deref(), if spotify { 150.0 } else { 130.0 }),
                title_text,
                text(artist).size(11).style(tstyle(t.muted)),
            ]
            .spacing(if spotify { 8 } else { 6 })
            .width(Length::Fixed(if spotify { 150.0 } else { 130.0 }));

            let mut card = button(card_content).padding(if spotify { 10 } else { 4 }).style(move |_th, status| {
                let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if spotify {
                        Some(Background::Color(if h { t.surface2 } else { t.surface }))
                    } else if h {
                        Some(Background::Color(t.surface))
                    } else {
                        None
                    },
                    text_color: t.text,
                    border: Border { radius: if spotify { 8.0 } else { 4.0 }.into(), ..Border::default() },
                    ..button::Style::default()
                }
            });
            if let Some(aid) = play.album_id.clone() {
                card = card.on_press(Message::Navigate(View::AlbumDetail(aid)));
            }
            cards = cards.push(card);
        }
        column![
            shelf_label("RECENTLY PLAYED", t, spotify),
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
        let spotify = self.ui_theme_id == "spotify";

        let mut cards = row![].spacing(if spotify { 16 } else { 12 });
        for (id, name, cover_art_id) in self.home_recent_artists_cache.iter().take(5) {
            let (id, name, cover_art_id) = (id.clone(), name.clone(), cover_art_id.clone());
            let size = if spotify { 150.0 } else { 130.0 };
            cards = cards.push(
                button(
                    column![
                        self.cover_image(cover_art_id.as_deref(), size),
                        text(name).size(if spotify { 13 } else { 12 }).style(tstyle(t.text)).font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::MONOSPACE
                        }),
                    ]
                    .spacing(if spotify { 8 } else { 6 })
                    .width(Length::Fixed(size)),
                )
                .padding(if spotify { 10 } else { 4 })
                .on_press(Message::Navigate(View::ArtistDetail(id)))
                .style(move |_th, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: if spotify {
                            Some(Background::Color(if h { t.surface2 } else { t.surface }))
                        } else if h {
                            Some(Background::Color(t.surface))
                        } else {
                            None
                        },
                        text_color: t.text,
                        border: Border { radius: if spotify { 8.0 } else { 4.0 }.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                }),
            );
        }

        column![
            shelf_label("RECENTLY PLAYED ARTISTS", t, spotify),
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
        let names: Vec<String> = self.genres.iter().take(30).map(|g| g.name.clone()).collect();
        column![
            shelf_label("GENRES", t, self.ui_theme_id == "spotify"),
            responsive(move |size| self.genre_chip_rows(&names, size.width)),
        ]
        .spacing(10)
        .into()
    }

    // Chips don't wrap natively in an iced `row`, so lines are packed by hand
    // using a rough per-glyph width estimate and rebuilt on every resize breakpoint.
    fn genre_chip_rows<'a>(&'a self, names: &[String], available_width: f32) -> Element<'a, Message> {
        let t = self.tokens;
        const GLYPH_WIDTH: f32 = 7.2;
        const CHIP_PADDING: f32 = 24.0;
        const SPACING: f32 = 8.0;

        let mut lines = column![].spacing(SPACING);
        let mut current_row = row![].spacing(SPACING);
        let mut current_width = 0.0f32;

        for name in names {
            let chip_width = name.chars().count() as f32 * GLYPH_WIDTH + CHIP_PADDING;
            if current_width > 0.0 && current_width + SPACING + chip_width > available_width {
                lines = lines.push(current_row);
                current_row = row![].spacing(SPACING);
                current_width = 0.0;
            }
            current_width += if current_width > 0.0 { SPACING } else { 0.0 } + chip_width;

            let owned = name.clone();
            current_row = current_row.push(
                button(text(name.clone()).size(12).style(tstyle(t.text)))
                    .padding([6, 12])
                    .on_press(Message::Navigate(View::GenreDetail(owned)))
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
        lines = lines.push(current_row);
        lines.into()
    }

    pub(crate) fn home_section(&self, title: &'static str, albums: &[Album]) -> Element<'_, Message> {
        let t = self.tokens;
        if albums.is_empty() {
            return column![].into();
        }
        let spotify = self.ui_theme_id == "spotify";
        let mut cards = row![].spacing(if spotify { 16 } else { 12 });
        for a in albums.iter().take(5) {
            cards = cards.push(self.album_card(a));
        }
        column![
            shelf_label(title, t, spotify),
            cards,
        ]
        .spacing(12)
        .into()
    }

    pub(crate) fn album_card(&self, album: &Album) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let size = if spotify { 150.0 } else { 130.0 };
        button(
            column![
                self.cover_image(album.cover_art_id.as_deref(), size),
                text(album.name.clone()).size(if spotify { 13 } else { 12 }).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::MONOSPACE
                }),
                text(album.album_artist.clone()).size(11).style(tstyle(t.muted)),
            ]
            .spacing(if spotify { 8 } else { 6 })
            .width(Length::Fixed(size)),
        )
        .padding(if spotify { 10 } else { 4 })
        .on_press(Message::Navigate(View::AlbumDetail(album.id.clone())))
        .style(move |_th, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if spotify {
                    Some(Background::Color(if h { t.surface2 } else { t.surface }))
                } else if h {
                    Some(Background::Color(t.surface))
                } else {
                    None
                },
                text_color: t.text,
                border: Border { radius: if spotify { 8.0 } else { 4.0 }.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
    }
}
