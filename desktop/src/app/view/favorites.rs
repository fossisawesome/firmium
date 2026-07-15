use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Length};

use super::super::message::Message;
use super::super::styles::*;
use super::super::types::View;
use super::super::App;

impl App {
    pub(crate) fn favorites_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let header = page_header("Favorites", t, spotify);

        let Some(starred) = &self.favorites else {
            return column![header, text("Loading…").size(13).style(tstyle(t.muted))].spacing(16).into();
        };

        let mut sections = column![header].spacing(28);

        if !starred.albums.is_empty() {
            let mut cards = row![].spacing(if spotify { 16 } else { 12 });
            for a in starred.albums.iter().take(20) {
                cards = cards.push(self.album_card(a));
            }
            sections = sections.push(
                column![
                    shelf_label("ALBUMS", t, spotify),
                    scrollable(cards).direction(scrollable::Direction::Horizontal(Default::default())),
                ]
                .spacing(12),
            );
        }

        if !starred.songs.is_empty() {
            let mut list = column![];
            for (i, song) in starred.songs.iter().enumerate() {
                list = list.push(self.track_row(i, song, Message::PlaySong(song.clone())));
            }
            sections = sections.push(column![shelf_label("SONGS", t, spotify), list].spacing(12));
        }

        if !starred.artists.is_empty() {
            let mut list = column![];
            for artist in &starred.artists {
                list = list.push(
                    button(text(artist.name.clone()).size(14).style(tstyle(t.text)))
                        .on_press(Message::Navigate(View::ArtistDetail(artist.id.clone())))
                        .style(list_row_style(t, spotify)),
                );
            }
            sections = sections.push(column![shelf_label("ARTISTS", t, spotify), list].spacing(12));
        }

        if starred.albums.is_empty() && starred.songs.is_empty() && starred.artists.is_empty() {
            sections = sections.push(
                text("No favorites yet — tap the heart on any song, album, or artist.")
                    .size(13)
                    .style(tstyle(t.muted)),
            );
        }

        scrollable(sections).width(Length::Fill).height(Length::Fill).style(thin_scroll_style(t)).into()
    }
}
