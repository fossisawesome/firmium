use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Element, Length};

use crate::commands::mappers::Song;
use crate::commands::subsonic::PlaylistTracks;
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::cover::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn playlists_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = row![
            text(format!("Playlists ({})", self.playlist_items.len()))
                .size(22)
                .style(tstyle(t.text))
                .width(Length::Fill),
            button(
                row![icons::icon(icons::PLUS, 12.0, t.accent), text("New").size(12).style(tstyle(t.accent))]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
            .padding([6, 14])
            .on_press(Message::OpenCreatePlaylist)
            .style(list_row_style(t)),
        ]
        .align_y(Alignment::Center);

        if self.playlist_items.is_empty() {
            return column![header, text("No playlists yet").size(13).style(tstyle(t.muted))]
                .spacing(16)
                .into();
        }

        let mut list = column![].spacing(2);
        for item in &self.playlist_items {
            list = list.push(self.playlist_row(item));
        }
        column![
            header,
            scrollable(list)
                .height(Length::Fill)
                .direction(scrollable::Direction::Vertical(thin_scrollbar()))
                .style(thin_scroll_style(t))
        ]
        .spacing(16)
        .into()
    }

    pub(crate) fn rebuild_playlist_items(&mut self) {
        let claimed: std::collections::HashSet<&str> = self
            .playlists
            .iter()
            .filter_map(|p| p.server_id.as_deref())
            .collect();
        let mut items: Vec<PlaylistListItem> =
            (0..self.playlists.len()).map(PlaylistListItem::Local).collect();
        for (i, sp) in self.server_playlists.iter().enumerate() {
            let sid = sp.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if !claimed.contains(sid) {
                items.push(PlaylistListItem::ServerOnly(i));
            }
        }
        self.playlist_items = items;
    }

    pub(crate) fn refresh_local_detail(&mut self, local_id: &str) {
        if self.playlist_detail_id.as_deref() != Some(local_id) {
            return;
        }
        if let Some(p) = self.playlists.iter().find(|p| p.id == local_id) {
            self.playlist_detail = Some(PlaylistTracks {
                id: p.id.clone(),
                name: p.name.clone(),
                comment: String::new(),
                song_count: Some(p.tracks.len() as u32),
                tracks: p.tracks.clone(),
            });
        }
    }

    pub(crate) fn playlist_cover(&self, item: &PlaylistListItem) -> Element<'_, Message> {
        let t = self.tokens;
        // Up to 4 distinct cover ids.
        let cover_ids: Vec<String> = match item {
            PlaylistListItem::Local(i) => {
                let mut seen = std::collections::HashSet::new();
                self.playlists[*i]
                    .tracks
                    .iter()
                    .filter_map(|s| s.cover_art_id.clone())
                    .filter(|c| seen.insert(c.clone()))
                    .take(4)
                    .collect()
            }
            PlaylistListItem::ServerOnly(i) => self.server_playlists[*i]
                .get("coverArt")
                .and_then(|v| v.as_str())
                .map(|s| vec![s.to_string()])
                .unwrap_or_default(),
        };

        let inner: Element<'_, Message> = match cover_ids.len() {
            0 => icons::icon(icons::LIST, 22.0, t.muted),
            1 => self.cover_image(Some(cover_ids[0].as_str()), 44.0),
            _ => {
                let cell = |idx: usize| -> Element<'_, Message> {
                    self.cover_image(Some(cover_ids[idx % cover_ids.len()].as_str()), 22.0)
                };
                column![row![cell(0), cell(1)].spacing(0), row![cell(2), cell(3)].spacing(0)]
                    .spacing(0)
                    .into()
            }
        };

        container(inner)
            .center_x(Length::Fixed(44.0))
            .center_y(Length::Fixed(44.0))
            .clip(true)
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface2)),
                border: Border { radius: 6.0.into(), ..Border::default() },
                ..container::Style::default()
            })
            .into()
    }

    pub(crate) fn playlist_row(&self, item: &PlaylistListItem) -> Element<'_, Message> {
        let t = self.tokens;
        let (nav_id, name, count, synced, local_id): (String, String, usize, bool, Option<String>) =
            match item {
                PlaylistListItem::Local(i) => {
                    let p = &self.playlists[*i];
                    (p.id.clone(), p.name.clone(), p.tracks.len(), p.server_id.is_some(), Some(p.id.clone()))
                }
                PlaylistListItem::ServerOnly(i) => {
                    let sp = &self.server_playlists[*i];
                    let sid = sp.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let nm = sp.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
                    let c = sp.get("songCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    (format!("server-{sid}"), nm, c, true, None)
                }
            };

        let mut name_row = row![text(name).size(13).style(tstyle(t.text))]
            .spacing(6)
            .align_y(Alignment::Center);
        if synced {
            name_row = name_row.push(icons::icon(icons::CLOUD, 12.0, t.muted));
        }

        let open = button(
            row![
                self.playlist_cover(item),
                column![name_row, text(format!("{count} tracks")).size(11).style(tstyle(t.muted))].spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::Navigate(View::PlaylistDetail(nav_id)))
        .style(list_row_style(t));

        let mut trailing = row![].spacing(4).align_y(Alignment::Center);
        if let Some(lid) = &local_id {
            if !synced {
                trailing = trailing.push(icon_button(
                    icons::CLOUD, 16.0, t.accent, t, Message::SyncPlaylistNow(lid.clone()),
                ));
            }
            trailing = trailing.push(icon_button(
                icons::TRASH, 16.0, t.error, t, Message::DeleteLocalPlaylist(lid.clone()),
            ));
        }

        row![open, trailing].spacing(8).align_y(Alignment::Center).into()
    }

    pub(crate) fn playlist_track_row(
        &self,
        idx: usize,
        total: usize,
        song: &Song,
        local_id: &Option<String>,
        server_id: &Option<String>,
    ) -> Element<'_, Message> {
        let t = self.tokens;
        let base = self.track_row(idx, song, Message::PlayPlaylistAt(idx));

        let up_msg = match (local_id, server_id) {
            (Some(id), _) => Some(Message::MovePlaylistTrack(id.clone(), idx, idx.saturating_sub(1))),
            (None, Some(sid)) => Some(Message::MoveServerTrack(sid.clone(), idx, idx.saturating_sub(1))),
            _ => None,
        };
        let down_msg = match (local_id, server_id) {
            (Some(id), _) => Some(Message::MovePlaylistTrack(id.clone(), idx, idx + 1)),
            (None, Some(sid)) => Some(Message::MoveServerTrack(sid.clone(), idx, idx + 1)),
            _ => None,
        };
        let remove_msg = match (local_id, server_id) {
            (Some(id), _) => Some(Message::RemovePlaylistTrack(id.clone(), song.id.clone())),
            (None, Some(sid)) => Some(Message::RemoveServerTrack(sid.clone(), idx)),
            _ => None,
        };

        let up = button(icons::icon(icons::CHEVRON_UP, 14.0, t.muted))
            .padding(4)
            .on_press_maybe((idx > 0).then_some(up_msg).flatten())
            .style(list_row_style(t));
        let down = button(icons::icon(icons::CHEVRON_DOWN, 14.0, t.muted))
            .padding(4)
            .on_press_maybe((idx + 1 < total).then_some(down_msg).flatten())
            .style(list_row_style(t));
        let remove = button(icons::icon(icons::CLOSE, 14.0, t.error))
            .padding(4)
            .on_press_maybe(remove_msg)
            .style(list_row_style(t));

        row![base, up, down, remove].spacing(6).align_y(Alignment::Center).into()
    }

    pub(crate) fn playlist_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let Some(pt) = &self.playlist_detail else {
            return text("Loading…").size(13).style(tstyle(t.muted)).into();
        };
        let detail_id = self.playlist_detail_id.clone().unwrap_or_default();
        let server_id = detail_id.strip_prefix("server-").map(String::from);
        let local_id = server_id.is_none().then(|| detail_id.clone());

        let play = button(
            row![
                icons::icon(icons::PLAY, 14.0, t.bg),
                text("Play").size(12).style(tstyle(t.bg)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::PlayPlaylistAt(0))
        .style(primary_button(t));

        // Title row: editable for local playlists when renaming.
        let renaming = local_id
            .as_ref()
            .map(|id| self.renaming_playlist.as_deref() == Some(id.as_str()))
            .unwrap_or(false);
        let title: Element<'_, Message> = if renaming {
            row![
                text_input("Playlist name…", &self.create_playlist_name)
                    .on_input(Message::CreatePlaylistNameInput)
                    .on_submit(Message::CommitRenamePlaylist)
                    .padding(8)
                    .size(20)
                    .width(Length::Fixed(360.0))
                    .style(text_input_style(t)),
                icon_button(icons::PLAY, 16.0, t.accent, t, Message::CommitRenamePlaylist),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            let mut tr = row![text(pt.name.clone()).size(24).style(tstyle(t.text))]
                .spacing(10)
                .align_y(Alignment::Center);
            if let Some(id) = &local_id {
                tr = tr.push(icon_button(icons::PENCIL, 16.0, t.muted, t, Message::StartRenamePlaylist(id.clone())));
            }
            tr.into()
        };

        let (first, end, top, bottom) = list_window(pt.tracks.len(), self.playlist_tracks_scroll, TRACK_ROW_H);
        let mut list = column![];
        if top > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(top)));
        }
        for (i, track) in pt.tracks[first..end].iter().enumerate() {
            list = list.push(self.playlist_track_row(first + i, pt.tracks.len(), track, &local_id, &server_id));
        }
        if bottom > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(bottom)));
        }

        column![
            back_button(t),
            column![
                title,
                text(format!("{} tracks", pt.tracks.len())).size(11).style(tstyle(t.muted)),
                play,
            ]
            .spacing(8),
            scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).on_scroll(|v| Message::PlaylistTracksScrolled(v.absolute_offset().y)),
        ]
        .spacing(16)
        .into()
    }
}
