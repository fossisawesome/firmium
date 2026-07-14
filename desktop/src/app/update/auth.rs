use iced::Task;

use firmium_backend::errors::UserError;

use super::super::message::Message;
use super::super::types::*;
use super::super::App;
use super::super::Toast;

impl App {
    pub(crate) fn update_auth(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ServerInput(s) => {
                self.server_input = s;
                Task::none()
            }
            Message::UsernameInput(s) => {
                self.username_input = s;
                Task::none()
            }
            Message::PasswordInput(s) => {
                self.password_input = s;
                Task::none()
            }
            Message::ToggleSavePassword(v) => {
                self.save_password = v;
                Task::none()
            }
            Message::Connect => {
                let server = self.server_input.trim().trim_end_matches('/').to_string();
                let user = self.username_input.trim().to_string();
                let pass = self.password_input.clone();
                if server.is_empty() || user.is_empty() {
                    self.toasts.push(Toast {
                        id: self.next_toast_id,
                        category: UserError::Unknown,
                        text: "Server URL and username are required".to_string(),
                        spawned: std::time::Instant::now(),
                    });
                    self.next_toast_id += 1;
                    return Task::none();
                }
                self.connecting = true;
                firmium_backend::commands::subsonic::set_connection(
                    &self.backend.app_state,
                    Some(server.clone()),
                    Some(user.clone()),
                    Some(pass.clone()),
                );
                if self.save_password {
                    let _ = firmium_backend::commands::credentials::save_password(Some(&server), &user, &pass);
                }
                Task::perform(
                    firmium_backend::commands::subsonic::validate_connection(self.backend.app_state.clone()),
                    Message::Connected,
                )
            }
            Message::Connected(Ok(())) => {
                self.authed = true;
                self.show_account_switcher = false;
                self.connecting = false;
                self.password_input.clear();
                self.remember_current_account();
                self.save_config();
                // Local-fallback artists aren't auto-refetched on connect (only on
                // first Artists nav); clear so that nav re-fetches from the server.
                self.artists.clear();
                if let Some(history) = &self.backend.history {
                    self.home_recent_plays = history.recent_plays(15).unwrap_or_default();
                    self.recompute_home_recent_artists();
                }
                let play_cover_ids: Vec<String> = self.home_recent_plays.iter()
                    .filter_map(|p| p.cover_art_id.clone())
                    .collect();
                let cover_task = self.load_cover_ids(play_cover_ids);
                let s = self.backend.app_state.clone();
                Task::batch([
                    Task::perform(firmium_backend::commands::subsonic::get_albums(s.clone()), Message::AlbumsLoaded),
                    Task::perform(firmium_backend::commands::subsonic::get_recent_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Recent, r)),
                    Task::perform(firmium_backend::commands::subsonic::get_newest_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Newest, r)),
                    Task::perform(firmium_backend::commands::subsonic::get_random_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Random, r)),
                    Task::perform(firmium_backend::commands::subsonic::get_genres_list(s.clone()), Message::GenresLoaded),
                    Task::perform(firmium_backend::commands::subsonic::get_play_queue(s.clone()), Message::PlayQueueFetched),
                    Task::perform(firmium_backend::podcasts::probe_server_podcast_support(s), |_| Message::PlaylistSyncNoop),
                    cover_task,
                ])
            }
            Message::Connected(Err(e)) => {
                self.connecting = false;
                self.show_account_switcher = true;
                eprintln!("connect failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }

            // Deferred startup keyring reads — see initial_task().
            Message::CredentialsLoaded(Some(pass)) => {
                firmium_backend::commands::subsonic::set_connection(
                    &self.backend.app_state,
                    Some(self.server_input.clone()),
                    Some(self.username_input.clone()),
                    Some(pass),
                );
                self.connecting = true;
                Task::perform(
                    firmium_backend::commands::subsonic::validate_connection(self.backend.app_state.clone()),
                    Message::Connected,
                )
            }
            Message::CredentialsLoaded(None) => Task::none(),
            Message::ServiceCredsLoaded(lastfm_key, lastfm_secret, listenbrainz_token) => {
                self.lastfm_enabled = !lastfm_key.is_empty();
                self.lastfm_key = lastfm_key;
                self.lastfm_secret = lastfm_secret;
                self.listenbrainz_enabled = !listenbrainz_token.is_empty();
                self.listenbrainz_token = listenbrainz_token;
                Task::none()
            }

            // ── Data ──────────────────────────────────────────────────────────
            Message::Logout => {
                firmium_backend::commands::subsonic::set_connection(&self.backend.app_state, None, None, None);
                self.authed = false;
                self.show_account_switcher = true;
                self.search_results = None;
                self.populate_offline_library();
                self.save_config();
                Task::batch([self.load_covers(), self.load_cover_ids(self.offline_home_cover_ids())])
            }

            // ── Equalizer ───────────────────────────────────────────────────────
            Message::ToggleAccountSwitcher => {
                self.show_account_switcher = !self.show_account_switcher;
                Task::none()
            }
            Message::SwitchAccount(acct) => {
                self.show_account_switcher = false;
                let already = {
                    let conn = self.backend.app_state.connection.read();
                    conn.server.as_deref() == Some(acct.server.as_str())
                        && conn.username.as_deref() == Some(acct.username.as_str())
                };
                if already {
                    return Task::none();
                }
                match firmium_backend::commands::credentials::get_password(Some(&acct.server), &acct.username) {
                    Ok(pass) => {
                        firmium_backend::commands::subsonic::set_connection(
                            &self.backend.app_state,
                            Some(acct.server.clone()),
                            Some(acct.username.clone()),
                            Some(pass),
                        );
                        self.server_input = acct.server.clone();
                        self.username_input = acct.username.clone();
                        self.reset_library();
                        self.connecting = true;
                        self.authed = false;
                        Task::perform(
                            firmium_backend::commands::subsonic::validate_connection(self.backend.app_state.clone()),
                            Message::Connected,
                        )
                    }
                    Err(_) => {
                        // Password no longer in keyring — bounce to login popup, prefilled.
                        self.server_input = acct.server.clone();
                        self.username_input = acct.username.clone();
                        self.authed = false;
                        self.show_account_switcher = true;
                        Task::none()
                    }
                }
            }
            Message::AddAccount => {
                self.show_account_switcher = true;
                self.authed = false;
                self.server_input.clear();
                self.username_input.clear();
                self.password_input.clear();
                Task::none()
            }

            // ── Recap ───────────────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}

impl App {
    pub(crate) fn reset_library(&mut self) {
        self.albums.clear();
        self.albums_scroll = 0.0;
        self.home_recent.clear();
        self.home_newest.clear();
        self.home_random.clear();
        self.home_recent_plays.clear();
        self.home_recent_artists_cache.clear();
        self.album_detail = None;
        self.album_detail_id = None;
        self.album_tracks_scroll = 0.0;
        self.artists.clear();
        self.artists_scroll = 0.0;
        self.artist_detail = None;
        self.artist_detail_id = None;
        self.artist_info = None;
        self.similar_artists.clear();
        self.server_playlists.clear();
        self.playlist_detail = None;
        self.playlist_detail_id = None;
        self.playlist_tracks_scroll = 0.0;
        self.cover_cache.clear();
        self.cover_cache_order.clear();
        self.search_results = None;
        self.resume_queue = None;
        self.genres.clear();
        self.genre_songs.clear();
        self.genre_detail_name = None;
        self.view = View::Home;
        self.nav_stack.clear();
    }
}
