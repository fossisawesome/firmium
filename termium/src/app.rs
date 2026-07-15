use std::sync::Arc;
use firmium_backend::commands::mappers::{Album, Artist, Song};
use firmium_backend::events::BackendEvent;
use firmium_backend::init::Backend;
use crate::keymap::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Login,
    Home,
    Albums,
    Artists,
    Search,
    Playlists,
}

// BackendEvent (the largest variant) already carries this same allow at its
// own definition (backend/events.rs) — events are low-frequency (key presses,
// tick, backend notifications), not a hot path worth boxing for.
#[allow(clippy::large_enum_variant)]
pub enum AppEvent {
    Key(Action),
    CharInput(char),
    Backspace,
    LoginFieldNext,
    LoginSubmit,
    SearchSubmit,
    SearchCancel,
    LoginResult(Result<Arc<Backend>, String>),
    AlbumsLoaded(Result<Vec<Album>, String>),
    ArtistsLoaded(Result<Vec<Artist>, String>),
    SearchResultsLoaded(Result<Vec<Song>, String>),
    PlaylistsLoaded(Result<Vec<(String, String)>, String>),
    Backend(BackendEvent),
    Tick,
}

/// A backend call `main.rs`'s loop should spawn/perform after `App::update`
/// returns. `App::update` is synchronous and cannot `.await`, so side effects
/// requiring a backend call are handed back to the caller instead of being
/// performed inline.
pub enum SideEffect {
    Login { server: String, username: String, password: String },
    TogglePlay,
    NextTrack,
    PrevTrack,
    LoadHomeAlbums,
    LoadArtists,
    LoadPlaylists,
    RunSearch(String),
    PlayAlbum(String),
    PlaySongs(Vec<Song>),
    PlayPlaylist(String),
}

pub struct App {
    pub backend: Option<Arc<Backend>>, // None until login succeeds
    pub view: View,
    pub should_quit: bool,
    pub status_message: Option<String>,

    // Login screen fields
    pub login_server: String,
    pub login_username: String,
    pub login_password: String,
    pub login_field_focus: u8, // 0 = server, 1 = username, 2 = password

    // Browse state
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub selected_index: usize,

    // Search state
    pub search_query: String,
    pub search_submitted: bool,
    pub search_results_songs: Vec<Song>,

    // Playlists (id, name) pairs — get_playlists returns raw JSON, extracted on load
    pub playlists: Vec<(String, String)>,

    // Playback state (mirrored from BackendEvent::PlaybackStateChanged / PlaybackPosition)
    pub now_playing: Option<Song>,
    pub playback_position: f64,
    pub playback_duration: Option<f64>,
    pub is_playing: bool,
    pub volume: f32,

    // Queue (mirrored from BackendEvent::QueueStateChanged)
    pub queue: Vec<Song>,
    pub queue_idx: i32,

    // Visualizer
    pub viz_snapshot: Vec<f32>,
}

impl App {
    pub fn new() -> Self {
        Self {
            backend: None,
            view: View::Login,
            should_quit: false,
            status_message: None,
            login_server: String::new(),
            login_username: String::new(),
            login_password: String::new(),
            login_field_focus: 0,
            albums: Vec::new(),
            artists: Vec::new(),
            selected_index: 0,
            search_query: String::new(),
            search_submitted: false,
            search_results_songs: Vec::new(),
            playlists: Vec::new(),
            now_playing: None,
            playback_position: 0.0,
            playback_duration: None,
            is_playing: false,
            volume: 1.0,
            queue: Vec::new(),
            queue_idx: -1,
            viz_snapshot: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self::new()
    }

    pub fn update(&mut self, event: AppEvent) -> Option<SideEffect> {
        match event {
            AppEvent::Key(action) => return self.handle_action(action),
            AppEvent::CharInput(c) => self.handle_char_input(c),
            AppEvent::Backspace => self.handle_backspace(),
            AppEvent::LoginFieldNext => {
                self.login_field_focus = (self.login_field_focus + 1) % 3;
            }
            AppEvent::LoginSubmit => {
                return Some(SideEffect::Login {
                    server: self.login_server.clone(),
                    username: self.login_username.clone(),
                    password: self.login_password.clone(),
                });
            }
            AppEvent::SearchSubmit => {
                self.search_submitted = true;
                if !self.search_query.is_empty() {
                    return Some(SideEffect::RunSearch(self.search_query.clone()));
                }
            }
            AppEvent::SearchCancel => {
                self.view = View::Home;
                self.search_query.clear();
                self.search_submitted = false;
                self.search_results_songs.clear();
            }
            AppEvent::LoginResult(Ok(backend)) => {
                backend.audio_player.set_visualizer_enabled(true);
                self.backend = Some(backend);
                self.view = View::Home;
                self.status_message = None;
                return Some(SideEffect::LoadHomeAlbums);
            }
            AppEvent::LoginResult(Err(e)) => {
                self.status_message = Some(e);
            }
            AppEvent::AlbumsLoaded(Ok(albums)) => {
                self.albums = albums;
                self.selected_index = 0;
            }
            AppEvent::AlbumsLoaded(Err(e)) => self.status_message = Some(e),
            AppEvent::ArtistsLoaded(Ok(artists)) => {
                self.artists = artists;
                self.selected_index = 0;
            }
            AppEvent::ArtistsLoaded(Err(e)) => self.status_message = Some(e),
            AppEvent::SearchResultsLoaded(Ok(songs)) => {
                self.search_results_songs = songs;
                self.selected_index = 0;
            }
            AppEvent::SearchResultsLoaded(Err(e)) => self.status_message = Some(e),
            AppEvent::PlaylistsLoaded(Ok(playlists)) => {
                self.playlists = playlists;
                self.selected_index = 0;
            }
            AppEvent::PlaylistsLoaded(Err(e)) => self.status_message = Some(e),
            AppEvent::Backend(evt) => self.handle_backend_event(evt),
            AppEvent::Tick => {
                if let Some(backend) = &self.backend {
                    self.viz_snapshot = backend.audio_player.visualizer().bars();
                }
            }
        }
        None
    }

    fn current_list_len(&self) -> usize {
        match self.view {
            View::Albums => self.albums.len(),
            View::Artists => self.artists.len(),
            View::Search => self.search_results_songs.len(),
            View::Playlists => self.playlists.len(),
            _ => 0,
        }
    }

    fn handle_char_input(&mut self, c: char) {
        match self.view {
            View::Login => match self.login_field_focus {
                0 => self.login_server.push(c),
                1 => self.login_username.push(c),
                _ => self.login_password.push(c),
            },
            View::Search if !self.search_submitted => self.search_query.push(c),
            _ => {}
        }
    }

    fn handle_backspace(&mut self) {
        match self.view {
            View::Login => match self.login_field_focus {
                0 => { self.login_server.pop(); }
                1 => { self.login_username.pop(); }
                _ => { self.login_password.pop(); }
            },
            View::Search if !self.search_submitted => { self.search_query.pop(); }
            _ => {}
        }
    }

    fn handle_action(&mut self, action: Action) -> Option<SideEffect> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::NavDown => {
                let len = self.current_list_len();
                if len > 0 && self.selected_index + 1 < len {
                    self.selected_index += 1;
                }
            }
            Action::NavUp => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Action::NavLeft | Action::NavRight => {
                // Tab-cycle between browse views when not inside a text field.
                if !matches!(self.view, View::Login) && (self.view != View::Search || self.search_submitted) {
                    let order = [View::Home, View::Albums, View::Artists, View::Playlists];
                    let idx = order.iter().position(|v| *v == self.view).unwrap_or(0);
                    let next = match action {
                        Action::NavRight => (idx + 1) % order.len(),
                        _ => (idx + order.len() - 1) % order.len(),
                    };
                    self.view = order[next];
                    self.selected_index = 0;
                    return match self.view {
                        View::Artists if self.artists.is_empty() => Some(SideEffect::LoadArtists),
                        View::Playlists if self.playlists.is_empty() => Some(SideEffect::LoadPlaylists),
                        _ => None,
                    };
                }
            }
            Action::PlayPause => return Some(SideEffect::TogglePlay),
            Action::NextTrack => return Some(SideEffect::NextTrack),
            Action::PrevTrack => return Some(SideEffect::PrevTrack),
            Action::Search => {
                self.view = View::Search;
                self.selected_index = 0;
                self.search_query.clear();
                self.search_submitted = false;
            }
            Action::Activate => match self.view {
                View::Albums => {
                    if let Some(album) = self.albums.get(self.selected_index) {
                        return Some(SideEffect::PlayAlbum(album.id.clone()));
                    }
                }
                View::Search if self.search_submitted => {
                    if let Some(song) = self.search_results_songs.get(self.selected_index) {
                        return Some(SideEffect::PlaySongs(vec![song.clone()]));
                    }
                }
                View::Playlists => {
                    if let Some((id, _)) = self.playlists.get(self.selected_index) {
                        return Some(SideEffect::PlayPlaylist(id.clone()));
                    }
                }
                _ => {}
            },
        }
        None
    }

    fn handle_backend_event(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::PlaybackStateChanged { state, .. } => {
                self.is_playing = matches!(state, firmium_backend::types::PlaybackState::Playing);
            }
            BackendEvent::PlaybackPosition { position, duration, .. } => {
                self.playback_position = position;
                self.playback_duration = duration;
            }
            BackendEvent::PlaybackFinished { .. } => {
                self.is_playing = false;
            }
            BackendEvent::QueueStateChanged(snapshot) => {
                self.queue = snapshot.queue;
                self.queue_idx = snapshot.queue_idx;
                self.volume = snapshot.volume;
                self.now_playing = if snapshot.queue_idx >= 0 {
                    self.queue.get(snapshot.queue_idx as usize).cloned()
                } else {
                    None
                };
            }
            BackendEvent::QueueExhausted(_) => {}
            BackendEvent::SessionExpired => {
                self.backend = None;
                self.view = View::Login;
                self.status_message = Some("Session expired. Please log in again.".to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_action_sets_should_quit() {
        let mut app = App::new_for_test();
        app.update(AppEvent::Key(Action::Quit));
        assert!(app.should_quit);
    }

    #[test]
    fn nav_down_in_album_list_advances_selection() {
        let mut app = App::new_for_test();
        app.view = View::Albums;
        app.albums = vec![test_album("1", "First"), test_album("2", "Second")];
        app.selected_index = 0;
        app.update(AppEvent::Key(Action::NavDown));
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn nav_down_at_end_of_list_does_not_overflow() {
        let mut app = App::new_for_test();
        app.view = View::Albums;
        app.albums = vec![test_album("1", "Only")];
        app.selected_index = 0;
        app.update(AppEvent::Key(Action::NavDown));
        assert_eq!(app.selected_index, 0);
    }

    fn test_album(id: &str, name: &str) -> Album {
        Album {
            id: id.to_string(),
            name: name.to_string(),
            album_artist: "Someone".to_string(),
            artist_id: None,
            cover_art_id: None,
            song_count: None,
            release_type: "album".to_string(),
            genres: None,
            year: None,
            is_compilation: false,
            starred: false,
        }
    }
}
