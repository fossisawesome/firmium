use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use super::super::cover::*;
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
            // Same reasoning as the songs shelf below: only worth windowing
            // once there are more cards than fit in one viewport.
            let cards_per_viewport = (VIEWPORT_H / ALBUM_CARD_W).ceil() as usize;
            let albums_body: Element<'_, Message> = if starred.albums.len() <= cards_per_viewport {
                let mut cards = row![].spacing(if spotify { 16 } else { 12 });
                for a in starred.albums.iter() {
                    cards = cards.push(self.album_card(a));
                }
                scrollable(cards).direction(scrollable::Direction::Horizontal(self.make_scrollbar())).style(thin_scroll_style(t)).into()
            } else {
                let (first, end, before, after) = list_window(starred.albums.len(), self.favorites_albums_scroll, ALBUM_CARD_W);
                let mut cards = row![].spacing(if spotify { 16 } else { 12 });
                if before > 0.0 {
                    cards = cards.push(container(text("")).width(Length::Fixed(before)));
                }
                for a in starred.albums[first..end].iter() {
                    cards = cards.push(self.album_card(a));
                }
                if after > 0.0 {
                    cards = cards.push(container(text("")).width(Length::Fixed(after)));
                }
                scrollable(cards)
                    .direction(scrollable::Direction::Horizontal(self.make_scrollbar()))
                    .style(thin_scroll_style(t))
                    .on_scroll(|v| Message::FavoritesAlbumsScrolled(v.absolute_offset().x))
                    .into()
            };
            sections = sections.push(column![shelf_label("ALBUMS", t, spotify), albums_body].spacing(12));
        }

        if !starred.songs.is_empty() {
            let rows_per_viewport = (VIEWPORT_H / TRACK_ROW_H).ceil() as usize;
            let songs_body: Element<'_, Message> = if starred.songs.len() <= rows_per_viewport {
                let mut list = column![];
                for (i, song) in starred.songs.iter().enumerate() {
                    list = list.push(self.track_row(i, song, Message::PlaySong(song.clone())));
                }
                list.into()
            } else {
                // Windowed like the other long lists (list_window, cover.rs):
                // with Favorites now refetched on every visit, an unbounded
                // eager render here would rebuild every row on every
                // playback tick.
                let (first, end, top, bottom) = list_window(starred.songs.len(), self.favorites_songs_scroll, TRACK_ROW_H);
                let mut list = column![];
                if top > 0.0 {
                    list = list.push(container(text("")).height(Length::Fixed(top)));
                }
                for (i, song) in starred.songs[first..end].iter().enumerate() {
                    list = list.push(self.track_row(first + i, song, Message::PlaySong(song.clone())));
                }
                if bottom > 0.0 {
                    list = list.push(container(text("")).height(Length::Fixed(bottom)));
                }
                scrollable(list)
                    .height(Length::Fixed(VIEWPORT_H))
                    .direction(scrollable::Direction::Vertical(self.make_scrollbar()))
                    .style(thin_scroll_style(t))
                    .on_scroll(|v| Message::FavoritesSongsScrolled(v.absolute_offset().y))
                    .into()
            };
            sections = sections.push(column![shelf_label("SONGS", t, spotify), songs_body].spacing(12));
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

        scrollable(sections)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(self.make_scrollbar()))
            .style(thin_scroll_style(t))
            .into()
    }
}
