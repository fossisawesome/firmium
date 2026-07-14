use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Element, Length};

use firmium_backend::commands::mappers::{Album, Song};
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::cover::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn album_list_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = page_header(format!("Albums ({})", self.albums.len()), t, self.ui_theme_id == "spotify");

        // Windowed (virtual) rendering: only the visible rows are built; the
        // scrolled-past and remaining heights are filled with spacers so the
        // scrollbar stays correct for libraries with thousands of albums.
        let (first, end, top, bottom) = list_window(self.albums.len(), self.albums_scroll, ALBUM_ROW_H);
        let mut list = column![];
        if top > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(top)));
        }
        for album in &self.albums[first..end] {
            list = list.push(self.album_row(album));
        }
        if bottom > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(bottom)));
        }

        let scroller = scrollable(list)
            .height(Length::Fill)
            .on_scroll(|v| Message::AlbumsScrolled(v.absolute_offset().y));

        column![header, scroller].spacing(16).into()
    }

    pub(crate) fn album_row(&self, album: &Album) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let cover = self.cover_image(album.cover_art_id.as_deref(), if spotify { 56.0 } else { 44.0 });
        let mut name_text = text(album.name.clone()).size(if spotify { 14 } else { 13 }).style(tstyle(t.text));
        if spotify {
            name_text = name_text.font(iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::MONOSPACE });
        }
        let info = column![
            name_text,
            text(album.album_artist.clone()).size(11).style(tstyle(t.muted)),
        ]
        .spacing(2);

        button(row![cover, info].spacing(12).align_y(Alignment::Center))
            .width(Length::Fill)
            .padding(if spotify { 10 } else { 8 })
            .on_press(Message::Navigate(View::AlbumDetail(album.id.clone())))
            .style(move |_theme, status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if hovered { Some(Background::Color(t.surface)) } else { None },
                    text_color: t.text,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            })
            .into()
    }

    pub(crate) fn album_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let Some(at) = &self.album_detail else {
            return text("Loading…").size(13).style(tstyle(t.muted)).into();
        };

        let back = button(
            row![
                icons::icon(icons::BACK, 14.0, t.muted),
                text("Back").size(12).style(tstyle(t.muted)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(6)
        .on_press(Message::NavigateBack)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if h { Some(Background::Color(t.surface)) } else { None },
                text_color: t.muted,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        let play_btn = button(
            row![
                icons::icon(icons::PLAY, 14.0, t.bg),
                text("Play").size(12).style(tstyle(t.bg)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::PlayAlbumAt(0))
        .style(primary_button(t, spotify));

        let shuffle_btn = button(
            row![
                icons::icon(icons::SHUFFLE, 14.0, t.text),
                text("Shuffle").size(12).style(tstyle(t.text)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::ShuffleAlbum)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        let album_download_btn = button(
            row![
                icons::icon(icons::DOWNLOAD, 14.0, t.text),
                text("Download").size(12).style(tstyle(t.text)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::DownloadAlbum)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        let header = row![
            self.cover_image(at.cover_art_id.as_deref(), 80.0),
            column![
                text(at.album_name.clone()).size(18).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::MONOSPACE
                }),
                text(at.album_artist.clone()).size(13).style(tstyle(t.muted)),
                text(format!("{} tracks", at.tracks.len())).size(11).style(tstyle(t.muted)),
                row![play_btn, shuffle_btn, album_download_btn].spacing(8),
            ]
            .spacing(10),
        ]
        .spacing(20);

        let (first, end, top, bottom) = list_window(at.tracks.len(), self.album_tracks_scroll, TRACK_ROW_H);
        let mut list = column![];
        if top > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(top)));
        }
        for (i, track) in at.tracks[first..end].iter().enumerate() {
            list = list.push(self.track_row(first + i, track, Message::PlayAlbumAt(first + i)));
        }
        if bottom > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(bottom)));
        }

        column![back, header, scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)).on_scroll(|v| Message::AlbumTracksScrolled(v.absolute_offset().y))]
            .spacing(16)
            .into()
    }

    pub(crate) fn track_row(&self, idx: usize, song: &Song, on_press: Message) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let is_current = self.current_song_id() == Some(song.id.as_str());
        let title_color = if is_current { t.accent } else { t.text };
        let num = song
            .track_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| (idx + 1).to_string());

        let play_area = button(
            row![
                text(num).size(11).style(tstyle(t.muted)).width(Length::Fixed(24.0)),
                self.cover_image(song.cover_art_id.as_deref(), 36.0),
                column![
                    text(song.title.clone()).size(13).style(tstyle(title_color)),
                    text(song.artist.clone()).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(on_press)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if is_current {
                    Some(Background::Color(t.accent_dim))
                } else if h {
                    Some(Background::Color(t.surface))
                } else {
                    None
                },
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        row![
            play_area,
            self.star_rating(song),
            self.avg_rating_badge(song),
            icon_button(icons::PLUS, 14.0, t.muted, t, spotify, Message::OpenAddToPlaylist(song.clone())),
            icon_button(icons::DOWNLOAD, 14.0, t.muted, t, spotify, Message::DownloadTrack(song.clone())),
            text(fmt_time(song.duration)).size(11).style(tstyle(t.muted)).width(Length::Fixed(44.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }
}
