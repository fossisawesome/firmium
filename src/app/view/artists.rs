use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Element, Length};

use crate::commands::mappers::Artist;
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::cover::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn artists_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = text(format!("Artists ({})", self.artists.len())).size(22).style(tstyle(t.text));
        let (first, end, top, bottom) = list_window(self.artists.len(), self.artists_scroll, ARTIST_ROW_H);
        let mut list = column![];
        if top > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(top)));
        }
        for artist in &self.artists[first..end] {
            list = list.push(self.artist_row(artist));
        }
        if bottom > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(bottom)));
        }
        column![header, scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).on_scroll(|v| Message::ArtistsScrolled(v.absolute_offset().y))].spacing(16).into()
    }

    pub(crate) fn artist_row(&self, artist: &Artist) -> Element<'_, Message> {
        let t = self.tokens;
        let avatar = container(icons::icon(icons::USER, 22.0, t.muted))
            .center_x(Length::Fixed(44.0))
            .center_y(Length::Fixed(44.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface2)),
                border: Border { radius: 22.0.into(), ..Border::default() },
                ..container::Style::default()
            });
        button(
            row![
                avatar,
                column![
                    text(artist.name.clone()).size(13).style(tstyle(t.text)),
                    text(format!("{} albums", artist.album_count)).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::Navigate(View::ArtistDetail(artist.id.clone())))
        .style(list_row_style(t))
        .into()
    }

    pub(crate) fn artist_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let Some(d) = &self.artist_detail else {
            return text("Loading…").size(13).style(tstyle(t.muted)).into();
        };
        let mut list = column![].spacing(2);
        for album in &d.albums {
            list = list.push(self.album_row(album));
        }

        let mut head = column![
            back_button(t),
            text(d.name.clone()).size(26).style(tstyle(t.accent)).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            }),
            text(format!("{} albums", d.albums.len())).size(12).style(tstyle(t.muted)),
        ]
        .spacing(12);

        // Bio (strip any trailing Last.fm "Read more" HTML link).
        if let Some(bio) = self.artist_info.as_ref().and_then(|i| i.bio.as_deref()) {
            let bio = bio.split("<a ").next().unwrap_or(bio).trim();
            if !bio.is_empty() {
                head = head.push(text(bio.to_string()).size(12).style(tstyle(t.muted)));
            }
        }

        if !self.similar_artists.is_empty() {
            let names = self
                .similar_artists
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(" · ");
            head = head.push(
                column![
                    section_label("YOU MIGHT ALSO LIKE", t),
                    text(names).size(12).style(tstyle(t.text)),
                ]
                .spacing(4),
            );
        }

        column![head, scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t))]
            .spacing(12)
            .into()
    }
}
