use iced::Task;

use firmium_backend::errors::UserError;

use super::super::message::Message;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn update_nav(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(view) => {
                if view != self.view {
                    self.nav_stack.push(self.view.clone());
                    self.view = view;
                }
                let state = self.backend.app_state.clone();
                match self.view.clone() {
                    View::AlbumDetail(id) if self.album_detail_id.as_deref() != Some(id.as_str()) => {
                        self.album_detail = None;
                        self.album_detail_id = Some(id.clone());
                        self.album_tracks_scroll = 0.0;
                        if id.starts_with("local:") {
                            let result = firmium_backend::commands::local_library::get_local_album_tracks(&self.backend.app_state, id)
                                .map(|r| firmium_backend::commands::subsonic::AlbumTracks {
                                    tracks: r.tracks,
                                    album_name: r.album_name,
                                    album_artist: r.album_artist,
                                    cover_art_id: r.cover_art_id,
                                })
                                .map_err(|_| UserError::NotFound);
                            Task::done(Message::AlbumTracksLoaded(result))
                        } else {
                            Task::perform(firmium_backend::commands::subsonic::get_album_tracks(state, id), Message::AlbumTracksLoaded)
                        }
                    }
                    View::ArtistDetail(id) if self.artist_detail_id.as_deref() != Some(id.as_str()) => {
                        self.artist_detail = None;
                        self.artist_info = None;
                        self.similar_artists.clear();
                        self.artist_detail_id = Some(id.clone());
                        if id.starts_with("local:") {
                            let result = firmium_backend::commands::local_library::get_local_artist_details(&self.backend.app_state, id)
                                .map(|r| firmium_backend::commands::subsonic::ArtistDetails { name: r.name, albums: r.albums })
                                .map_err(|_| UserError::NotFound);
                            Task::done(Message::ArtistDetailLoaded(result))
                        } else {
                            let artist_name = self
                                .artists
                                .iter()
                                .find(|a| a.id == *id)
                                .map(|a| a.name.clone())
                                .unwrap_or_default();
                            Task::batch([
                                Task::perform(firmium_backend::commands::subsonic::get_artist_details(state.clone(), id.clone()), Message::ArtistDetailLoaded),
                                Task::perform(firmium_backend::commands::subsonic::get_artist_info(state.clone(), id.clone(), self.lastfm_key.clone(), artist_name), Message::ArtistInfoLoaded),
                                Task::perform(firmium_backend::commands::subsonic::get_similar_artists(state, id, None), Message::SimilarArtistsLoaded),
                            ])
                        }
                    }
                    View::PlaylistDetail(id) if self.playlist_detail_id.as_deref() != Some(id.as_str()) => {
                        self.playlist_detail = None;
                        self.playlist_detail_id = Some(id.clone());
                        self.playlist_tracks_scroll = 0.0;
                        self.renaming_playlist = None;
                        if let Some(server_id) = id.strip_prefix("server-") {
                            Task::perform(
                                firmium_backend::commands::subsonic::get_playlist_tracks(state, server_id.to_string()),
                                Message::PlaylistTracksLoaded,
                            )
                        } else {
                            // Local playlist: build detail from memory, no fetch.
                            self.refresh_local_detail(&id);
                            Task::none()
                        }
                    }
                    View::Artists if self.artists.is_empty() => {
                        Task::perform(firmium_backend::commands::subsonic::get_artists(state), Message::ArtistsLoaded)
                    }
                    View::Playlists => {
                        Task::perform(firmium_backend::commands::subsonic::get_playlists(state), Message::PlaylistsLoaded)
                    }
                    View::Recap => self.compute_recap(),
                    View::GenreDetail(name) if self.genre_detail_name.as_deref() != Some(name.as_str()) => {
                        self.genre_songs.clear();
                        self.genre_detail_name = Some(name.clone());
                        Task::perform(firmium_backend::commands::subsonic::get_songs_by_genre(state, name, None), Message::GenreSongsLoaded)
                    }
                    View::Settings => {
                        self.load_history_summary();
                        Task::none()
                    }
                    View::Podcasts => {
                        if let Some(store) = self.backend.podcasts.clone() {
                            Task::perform(
                                async move { firmium_backend::podcasts::list_channels(store) },
                                Message::PodcastChannelsLoaded,
                            )
                        } else {
                            Task::none()
                        }
                    }
                    View::PodcastDetail(id) => {
                        if let Some(store) = self.backend.podcasts.clone() {
                            Task::perform(
                                async move { firmium_backend::podcasts::list_episodes(store, id) },
                                Message::PodcastEpisodesLoaded,
                            )
                        } else {
                            Task::none()
                        }
                    }
                    View::Home => {
                        if let Some(history) = &self.backend.history {
                            self.home_recent_plays = history.recent_plays(15).unwrap_or_default();
                            self.recompute_home_recent_artists();
                        }
                        let play_cover_ids: Vec<String> = self.home_recent_plays.iter()
                            .filter_map(|p| p.cover_art_id.clone())
                            .collect();
                        let cover_task = self.load_cover_ids(play_cover_ids);
                        if self.home_newest.is_empty() {
                            Task::batch([
                                Task::perform(firmium_backend::commands::subsonic::get_recent_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Recent, r)),
                                Task::perform(firmium_backend::commands::subsonic::get_newest_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Newest, r)),
                                Task::perform(firmium_backend::commands::subsonic::get_random_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Random, r)),
                                Task::perform(firmium_backend::commands::subsonic::get_genres_list(state), Message::GenresLoaded),
                                cover_task,
                            ])
                        } else {
                            cover_task
                        }
                    }
                    _ => Task::none(),
                }
            }
            Message::NavigateBack => {
                if let Some(view) = self.nav_stack.pop() {
                    self.view = view;
                }
                Task::none()
            }
            Message::Backend(event) => {
                self.handle_backend(event);
                let queue_cover_ids = self.queue.iter().filter_map(|s| s.cover_art_id.clone()).collect();
                Task::batch([
                    self.load_cover_ids(queue_cover_ids),
                    self.maybe_fetch_lyrics(),
                    self.maybe_fetch_similar(),
                    self.maybe_fetch_viz_colors(),
                ])
            }
            Message::VisualizerTick => Task::none(),
            Message::ShowToast(err) => { self.show_toast(err); Task::none() }
            Message::DismissToast(id) => { self.toasts.retain(|t| t.id != id); Task::none() }
            Message::ToastTick => {
                self.toasts.retain(|t| t.spawned.elapsed().as_secs() < 5);
                Task::none()
            }
            // Priority mirrors the overlay stacking order in `shell()`: close
            // whichever modal is on top first.
            Message::EscapePressed => {
                if self.show_account_switcher {
                    self.update(Message::ToggleAccountSwitcher)
                } else if self.add_to_playlist_song.is_some() {
                    self.update(Message::CloseAddToPlaylist)
                } else if self.show_create_playlist {
                    self.update(Message::CloseCreatePlaylist)
                } else if self.podcast_add_modal_open {
                    self.update(Message::CloseAddPodcastModal)
                } else {
                    Task::none()
                }
            }

            // ── Onboarding ────────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}
