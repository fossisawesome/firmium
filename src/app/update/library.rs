use iced::Task;


use super::super::cover::{ALBUM_ROW_H, load_rounded_cover};
use super::super::message::Message;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn update_library(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AlbumsLoaded(Ok(albums)) => {
                self.albums = albums;
                self.load_covers()
            }
            Message::AlbumsLoaded(Err(e)) => {
                eprintln!("get_albums failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::HomeAlbumsLoaded(section, Ok(albums)) => {
                let ids: Vec<String> = albums.iter().filter_map(|a| a.cover_art_id.clone()).collect();
                match section {
                    HomeSection::Recent => self.home_recent = albums,
                    HomeSection::Newest => self.home_newest = albums,
                    HomeSection::Random => self.home_random = albums,
                }
                self.load_cover_ids(ids)
            }
            Message::HomeAlbumsLoaded(_, Err(e)) => {
                eprintln!("home albums failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::CoverLoaded(id, Ok(path)) => {
                self.cache_cover(id, load_rounded_cover(&path));
                Task::none()
            }
            Message::CoverLoaded(_, Err(_)) => Task::none(),
            Message::AlbumsScrolled(y) => {
                self.albums_scroll = y;
                // Load covers for the rows scrolling into view.
                let first = ((y / ALBUM_ROW_H).floor().max(0.0) as usize).min(self.albums.len());
                let end = (first + 16).min(self.albums.len());
                let ids: Vec<String> = self.albums[first..end]
                    .iter()
                    .filter_map(|a| a.cover_art_id.clone())
                    .collect();
                self.load_cover_ids(ids)
            }
            Message::ArtistsScrolled(y) => {
                self.artists_scroll = y;
                Task::none()
            }
            Message::AlbumTracksScrolled(y) => {
                self.album_tracks_scroll = y;
                Task::none()
            }
            Message::AlbumTracksLoaded(Ok(at)) => {
                let ids: Vec<String> = at
                    .cover_art_id
                    .clone()
                    .into_iter()
                    .chain(at.tracks.iter().filter_map(|s| s.cover_art_id.clone()))
                    .collect();
                self.album_detail = Some(at);
                self.load_cover_ids(ids)
            }
            Message::AlbumTracksLoaded(Err(e)) => {
                eprintln!("get_album_tracks failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::ArtistsLoaded(Ok(a)) => {
                self.artists = a;
                Task::none()
            }
            Message::ArtistsLoaded(Err(e)) => {
                eprintln!("get_artists failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::ArtistDetailLoaded(Ok(d)) => {
                let ids: Vec<String> = d.albums.iter().filter_map(|a| a.cover_art_id.clone()).collect();
                self.artist_detail = Some(d);
                self.load_cover_ids(ids)
            }
            Message::ArtistInfoLoaded(Ok(info)) => {
                self.artist_info = info;
                Task::none()
            }
            Message::ArtistInfoLoaded(Err(e)) => {
                eprintln!("get_artist_info failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::SimilarArtistsLoaded(Ok(names)) => {
                self.similar_artists = names;
                Task::none()
            }
            Message::SimilarArtistsLoaded(Err(e)) => {
                eprintln!("get_similar_artists failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::ArtistDetailLoaded(Err(e)) => {
                eprintln!("get_artist_details failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::PlayAlbumAt(idx) => {
                if let Some(at) = &self.album_detail {
                    let songs = at.tracks.clone();
                    Task::perform(
                        crate::commands::queue::set_queue(
                            self.backend.queue_state.clone(),
                            self.backend.app_state.clone(),
                            self.backend.audio_player.clone(),
                            songs,
                            idx,
                        ),
                        Message::PlaybackDone,
                    )
                } else {
                    Task::none()
                }
            }
            Message::ShuffleAlbum => {
                if let Some(at) = &self.album_detail {
                    let songs = at.tracks.clone();
                    Task::perform(
                        crate::commands::queue::shuffle_and_play(
                            self.backend.queue_state.clone(),
                            self.backend.app_state.clone(),
                            self.backend.audio_player.clone(),
                            songs,
                        ),
                        Message::PlaybackDone,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PlaySong(song) => Task::perform(
                crate::commands::queue::set_queue(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                    vec![song],
                    0,
                ),
                Message::PlaybackDone,
            ),
            Message::SetRating(id, rating) => {
                // Optimistic local update so the stars fill immediately.
                if let Some(at) = &mut self.album_detail {
                    for s in &mut at.tracks {
                        if s.id == id {
                            s.user_rating = Some(rating);
                        }
                    }
                }
                if let Some(pt) = &mut self.playlist_detail {
                    for s in &mut pt.tracks {
                        if s.id == id {
                            s.user_rating = Some(rating);
                        }
                    }
                }
                if let Some(res) = &mut self.search_results {
                    for s in &mut res.songs {
                        if s.id == id {
                            s.user_rating = Some(rating);
                        }
                    }
                }
                for m in &mut self.similar_results {
                    if m.song.id == id {
                        m.song.user_rating = Some(rating);
                    }
                }
                Task::perform(
                    crate::commands::subsonic::set_rating(self.backend.app_state.clone(), id, rating),
                    |_| Message::DownloadDone(Ok(())),
                )
            }
            Message::DownloadTrack(song) => Task::perform(
                crate::commands::downloads::download_track(
                    self.backend.app_state.clone(),
                    song.id.clone(),
                    self.download_format.clone(),
                    song.artist.clone(),
                    song.album.clone(),
                    song.title.clone(),
                    song.track_number,
                    song.suffix.clone(),
                ),
                Message::DownloadDone,
            ),
            Message::DownloadDone(Ok(())) => Task::none(),
            Message::DownloadDone(Err(e)) => {
                eprintln!("download failed: {e}");
                Task::none()
            }

            // ── Add-to-playlist overlay ─────────────────────────────────────────
            Message::GenresLoaded(Ok(g)) => {
                self.genres = g;
                Task::none()
            }
            Message::GenresLoaded(Err(e)) => {
                eprintln!("get_genres_list failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::GenreSongsLoaded(Ok(songs)) => {
                let ids: Vec<String> = songs.iter().filter_map(|s| s.cover_art_id.clone()).collect();
                self.genre_songs = songs;
                self.load_cover_ids(ids)
            }
            Message::GenreSongsLoaded(Err(e)) => {
                eprintln!("get_songs_by_genre failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::PlayGenreAt(idx) => {
                if self.genre_songs.is_empty() {
                    Task::none()
                } else {
                    Task::perform(
                        crate::commands::queue::set_queue(
                            self.backend.queue_state.clone(),
                            self.backend.app_state.clone(),
                            self.backend.audio_player.clone(),
                            self.genre_songs.clone(),
                            idx,
                        ),
                        Message::PlaybackDone,
                    )
                }
            }

            Message::DownloadAlbum => {
                let Some(id) = self.album_detail_id.clone() else { return Task::none() };
                Task::perform(
                    crate::commands::downloads::download_album(
                        self.backend.app_state.clone(),
                        id,
                        self.download_format.clone(),
                    ),
                    Message::DownloadDone,
                )
            }

            // ── Podcasts ─────────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}
