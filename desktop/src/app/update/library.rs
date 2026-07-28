use iced::Task;


use super::super::cover::{ALBUM_CARD_W, ALBUM_ROW_H, TRACK_ROW_H, VIEWPORT_H, load_rounded_cover};
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
            Message::FavoritesSongsScrolled(y) => {
                self.favorites_songs_scroll = y;
                let Some(starred) = &self.favorites else { return Task::none(); };
                let first = ((y / TRACK_ROW_H).floor().max(0.0) as usize).min(starred.songs.len());
                let end = (first + 16).min(starred.songs.len());
                let ids: Vec<String> = starred.songs[first..end].iter().filter_map(|s| s.cover_art_id.clone()).collect();
                self.load_cover_ids(ids)
            }
            Message::FavoritesAlbumsScrolled(x) => {
                self.favorites_albums_scroll = x;
                let Some(starred) = &self.favorites else { return Task::none(); };
                let first = ((x / ALBUM_CARD_W).floor().max(0.0) as usize).min(starred.albums.len());
                let end = (first + 8).min(starred.albums.len());
                let ids: Vec<String> = starred.albums[first..end].iter().filter_map(|a| a.cover_art_id.clone()).collect();
                self.load_cover_ids(ids)
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
                        firmium_backend::commands::queue::set_queue(
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
                        firmium_backend::commands::queue::shuffle_and_play(
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
                firmium_backend::commands::queue::set_queue(
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
                // Remember what it was so a failed write (RatingRefreshed
                // below) can be reverted, same as ToggleStar/StarToggled.
                let previous_rating = self.current_user_rating(&id);
                self.for_each_rated_song_mut(&id, |s| s.user_rating = Some(rating));
                let temp_id = id.clone();
                Task::perform(
                    firmium_backend::commands::subsonic::set_rating_and_refetch(self.backend.app_state.clone(), id, rating),
                    move |result| Message::RatingRefreshed(temp_id.clone(), rating, previous_rating, result),
                )
            }
            Message::RatingRefreshed(id, _rating, _previous, Ok(song)) => {
                // Only revert if nothing newer has superseded this attempt
                if self.current_user_rating(&id) == song.user_rating {
                    self.update_average_rating_locally(&id, song.average_rating);
                }
                Task::none()
            }
            Message::RatingRefreshed(id, rating, previous, Err(e)) => {
                // Only revert if nothing newer has superseded this attempt
                if self.current_user_rating(&id) == Some(rating) {
                    self.for_each_rated_song_mut(&id, |s| s.user_rating = previous);
                }
                self.show_toast(e);
                Task::none()
            }
            Message::ToggleStar(id, kind) => {
                // Optimistic: figure out current starred state from whichever
                // in-memory collection holds this id, flip it locally, then
                // fire the network call. On failure, StarToggled reverts it.
                let currently_starred = self.is_starred(&id, kind);
                self.set_starred_locally(&id, kind, !currently_starred);
                let state = self.backend.app_state.clone();
                let id2 = id.clone();
                Task::perform(
                    async move {
                        let result = if currently_starred {
                            firmium_backend::commands::subsonic::unstar_item(state, id2, kind).await
                        } else {
                            firmium_backend::commands::subsonic::star_item(state, id2, kind).await
                        };
                        result.map(|_| !currently_starred)
                    },
                    move |r| Message::StarToggled(id.clone(), kind, r),
                )
            }
            Message::StarToggled(id, kind, result) => {
                if let Err(_e) = result {
                    // Revert the optimistic flip — re-read what it should have stayed as.
                    let reverted = !self.is_starred(&id, kind);
                    self.set_starred_locally(&id, kind, reverted);
                }
                Task::none()
            }
            Message::FavoritesLoaded(Ok(starred)) => {
                // First screenful only, the rest load incrementally as each shelf scrolls
                let albums_per_viewport = (VIEWPORT_H / ALBUM_CARD_W).ceil() as usize;
                let songs_per_viewport = (VIEWPORT_H / TRACK_ROW_H).ceil() as usize;
                let ids: Vec<String> = starred.albums.iter().take(albums_per_viewport).filter_map(|a| a.cover_art_id.clone())
                    .chain(starred.songs.iter().take(songs_per_viewport).filter_map(|s| s.cover_art_id.clone()))
                    .collect();
                self.favorites = Some(starred);
                self.favorites_songs_scroll = 0.0;
                self.favorites_albums_scroll = 0.0;
                self.load_cover_ids(ids)
            }
            Message::FavoritesLoaded(Err(_e)) => Task::none(),
            Message::DownloadTrack(song) => Task::perform(
                firmium_backend::commands::downloads::download_track(
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
                        firmium_backend::commands::queue::set_queue(
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
                    firmium_backend::commands::downloads::download_album(
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
