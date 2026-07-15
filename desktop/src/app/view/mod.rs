mod albums;
mod artists;
mod favorites;
mod genres;
mod home;
mod mix;
mod overlays;
mod panels;
mod player_bar;
mod playlists;
mod podcasts;
mod recap;
mod search;
mod settings;

use iced::Element;


use super::message::Message;
use super::types::*;
use super::App;

impl App {
    pub(crate) fn content_view(&self) -> Element<'_, Message> {
        match &self.view {
            View::Home => self.home_view(),
            View::Favorites => self.favorites_view(),
            View::Albums => self.album_list_view(),
            View::AlbumDetail(_) => self.album_detail_view(),
            View::Artists => self.artists_view(),
            View::ArtistDetail(_) => self.artist_detail_view(),
            View::Playlists => self.playlists_view(),
            View::PlaylistDetail(_) => self.playlist_detail_view(),
            View::Search => self.search_view(),
            View::Mix => self.mix_view(),
            View::GenreDetail(_) => self.genre_detail_view(),
            View::Recap => self.recap_view(),
            View::Settings => self.settings_view(),
            View::Podcasts => self.podcasts_view(),
            View::PodcastDetail(_) => self.podcast_detail_view(),
        }
    }
}
