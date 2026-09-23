use iced::widget::{button, column, container, row, scrollable, stack, text};
use iced::{Alignment, Background, Border, Element, Length, Padding};

use firmium_backend::commands::mappers::Artist;
use firmium_backend::commands::subsonic::ArtistDetails;
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::cover::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn artists_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = page_header(format!("Artists ({})", self.artists.len()), t, self.ui_theme_id == "spotify");
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
        column![header, scrollable(list).height(Length::Fill).id(MAIN_SCROLL_ID).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)).on_scroll(|v| Message::ArtistsScrolled(v.absolute_offset().y))].spacing(16).into()
    }

    pub(crate) fn artist_row(&self, artist: &Artist) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let avatar_size = if spotify { 56.0 } else { 44.0 };
        let avatar = container(icons::icon(icons::USER, avatar_size * 0.5, t.muted))
            .center_x(Length::Fixed(avatar_size))
            .center_y(Length::Fixed(avatar_size))
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface2)),
                border: Border { radius: (avatar_size / 2.0).into(), ..Border::default() },
                ..container::Style::default()
            });
        let mut name_text = text(artist.name.clone()).size(if spotify { 14 } else { 13 }).style(tstyle(t.text));
        if spotify {
            name_text = name_text.font(iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::MONOSPACE });
        }
        button(
            row![
                avatar,
                column![
                    name_text,
                    text(format!("{} albums", artist.album_count)).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(if spotify { 10 } else { 8 })
        .on_press(Message::Navigate(View::ArtistDetail(artist.id.clone())))
        .style(list_row_style(t, spotify))
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

        let spotify = self.ui_theme_id == "spotify";
        let more = icon_button(icons::MORE, 18.0, t.muted, t, spotify, Message::ToggleArtistMenu);
        let mut head = column![
            row![container(back_button(t)).width(Length::Fill), more].align_y(Alignment::Center),
            text(d.name.clone()).size(26).style(tstyle(t.accent)).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            }),
            text(format!("{} albums", d.albums.len())).size(12).style(tstyle(t.muted)),
        ]
        .spacing(12);

        if let Some(dl) = self.open_artist_download() {
            let total = dl.album_ids.len();
            let status = if !dl.is_finished() {
                format!("Downloading albums… {}/{}", dl.done, total)
            } else if dl.failed > 0 {
                format!("Downloaded {} of {} albums ({} failed)", total - dl.failed, total, dl.failed)
            } else {
                format!("Downloaded all {total} albums")
            };
            head = head.push(text(status).size(12).style(tstyle(t.accent)));
        }

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

        let page = column![head, scrollable(list).height(Length::Fill).id(MAIN_SCROLL_ID).on_scroll(|v| Message::MainScrolled(v.absolute_offset().y)).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t))]
            .spacing(12);
        if !self.artist_menu_open {
            return page.into();
        }

        // Invisible click-catcher: a click anywhere outside the menu closes it.
        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::CloseArtistMenu)
            .style(|_th, _status| button::Style::default());
        let menu = container(self.artist_menu(d))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::Start)
            // Drops the menu just below the ⋯ button.
            .padding(Padding { top: 44.0, ..Padding::ZERO });
        stack![page, backdrop, menu].into()
    }

    /// The running or finished "Download all" when it belongs to the open artist.
    fn open_artist_download(&self) -> Option<&ArtistDownload> {
        self.artist_download
            .as_ref()
            .filter(|dl| self.artist_detail_id.as_deref() == Some(dl.artist_id.as_str()))
    }

    fn artist_menu(&self, d: &ArtistDetails) -> Element<'_, Message> {
        let t = self.tokens;
        let has_albums = !d.albums.is_empty();
        let item = move |icon: &'static str, label: String, msg: Option<Message>| -> Element<'static, Message> {
            let color = if msg.is_some() { t.text } else { t.muted };
            button(
                row![icons::icon(icon, 14.0, color), text(label).size(13).style(tstyle(color))]
                    .spacing(10)
                    .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([8, 10])
            .on_press_maybe(msg)
            .style(move |_th, status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                    text_color: color,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            })
            .into()
        };

        let mut items: Vec<Element<'_, Message>> = vec![
            item(icons::PLAY, "Play all".into(), has_albums.then_some(Message::PlayArtist(false))),
            item(icons::SHUFFLE, "Shuffle all".into(), has_albums.then_some(Message::PlayArtist(true))),
        ];
        // Local-library artists are already on disk; nothing to download.
        let is_local = self.artist_detail_id.as_deref().is_some_and(|id| id.starts_with("local:"));
        if !is_local {
            let running = self.artist_download.as_ref().filter(|dl| !dl.is_finished());
            let (label, msg) = match (running, self.open_artist_download()) {
                (Some(dl), Some(_)) => (format!("Downloading… {}/{}", dl.done, dl.album_ids.len()), None),
                // One artist download at a time.
                (Some(_), None) => ("Download all (another is running)".to_string(), None),
                _ => ("Download all".to_string(), has_albums.then_some(Message::DownloadArtist)),
            };
            items.push(item(icons::DOWNLOAD, label, msg));
        }

        container(column(items).spacing(2))
            .width(Length::Fixed(240.0))
            .padding(6)
            .style(move |_th| container::Style {
                background: Some(Background::Color(t.surface)),
                border: Border { radius: 8.0.into(), width: 1.0, color: t.border },
                ..container::Style::default()
            })
            .into()
    }
}
