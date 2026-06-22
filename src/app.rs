//! Top-level iced application: `App` state, `Message`, `update`, `view`, the
//! event-bus subscription, and (Phase 7) the onboarding flow + Albums view.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use iced::widget::image::Handle as ImageHandle;
use iced::widget::{button, column, container, row, scrollable, slider, stack, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length, Subscription, Task, Theme};

use crate::commands::equalizer::{BandSpec, EqState};
use crate::commands::lyrics::LyricsResult;
use crate::commands::mappers::{Album, Artist, SimilarMatch, Song};
use crate::commands::local_library::LocalAlbumTracks;
use crate::commands::subsonic::{AlbumTracks, ArtistDetails, ArtistInfo, Genre, PlaylistTracks, RemotePlayQueue, SearchResult};
use crate::commands::themes::ThemeEntry;
use crate::config::{Config, SavedAccount};
use crate::events::{BackendEvent, EventBus};
use crate::init::Backend;
use crate::theme::Tokens;
use crate::viz::{Visualizer, VizMode};
use crate::{icons, PlaybackState};

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Home,
    Albums,
    AlbumDetail(String),
    Artists,
    ArtistDetail(String),
    Playlists,
    PlaylistDetail(String),
    Search,
    Mix,
    GenreDetail(String),
    Local,
    LocalAlbumDetail(String),
    Recap,
    Settings,
}

impl View {
    fn title(&self) -> &'static str {
        match self {
            View::Home => "Home",
            View::Albums => "Albums",
            View::AlbumDetail(_) => "Album",
            View::Artists => "Artists",
            View::ArtistDetail(_) => "Artist",
            View::Playlists => "Playlists",
            View::PlaylistDetail(_) => "Playlist",
            View::Search => "Search",
            View::Mix => "Mix",
            View::GenreDetail(_) => "Genre",
            View::Local => "Offline",
            View::LocalAlbumDetail(_) => "Album",
            View::Recap => "Recap",
            View::Settings => "Settings",
        }
    }
}

/// Recap aggregation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecapRange {
    Week,
    Month,
    ThreeMonths,
    Year,
    All,
}

impl RecapRange {
    /// Inclusive lower bound (unix seconds) for this window, given `now`.
    fn from_ts(self, now: i64) -> i64 {
        let day = 86_400;
        match self {
            RecapRange::Week => now - 7 * day,
            RecapRange::Month => now - 30 * day,
            RecapRange::ThreeMonths => now - 90 * day,
            RecapRange::Year => now - 365 * day,
            RecapRange::All => 0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            RecapRange::Week => "7 days",
            RecapRange::Month => "30 days",
            RecapRange::ThreeMonths => "3 months",
            RecapRange::Year => "1 year",
            RecapRange::All => "All time",
        }
    }
}

/// Number of swipeable Recap cards.
const RECAP_CARDS: usize = 9;

/// Which collapsible right-side panel is open (mutually exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Visualizer,
    Queue,
    Lyrics,
    Equalizer,
    AudioStats,
    Similar,
}

/// Which Settings category is selected in the two-column settings layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    Appearance,
    Playback,
    Equalizer,
    Downloads,
    Services,
    Account,
    Debug,
}

/// Home page album sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeSection {
    Recent,
    Newest,
    Random,
}

/// Mood Mix energy band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Energy {
    Chill,
    Mid,
    High,
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(View),
    NavigateBack,
    Backend(BackendEvent),
    VisualizerTick,

    // ── Onboarding ────────────────────────────────────────────────────────────
    ServerInput(String),
    UsernameInput(String),
    PasswordInput(String),
    Connect,
    Connected(Result<(), String>),

    // ── Data ──────────────────────────────────────────────────────────────────
    AlbumsLoaded(Result<Vec<Album>, String>),
    HomeAlbumsLoaded(HomeSection, Result<Vec<Album>, String>),
    AlbumTracksLoaded(Result<AlbumTracks, String>),
    ArtistsLoaded(Result<Vec<Artist>, String>),
    ArtistDetailLoaded(Result<ArtistDetails, String>),
    ArtistInfoLoaded(Result<Option<ArtistInfo>, String>),
    SimilarArtistsLoaded(Result<Vec<String>, String>),
    PlaylistsLoaded(Result<Vec<serde_json::Value>, String>),
    PlaylistTracksLoaded(Result<PlaylistTracks, String>),
    CoverLoaded(String, Result<String, String>),
    AlbumsScrolled(f32),
    PlayAlbumAt(usize),
    PlayPlaylistAt(usize),
    ShuffleAlbum,
    PlaySong(Song),
    SetRating(String, u32),
    DownloadTrack(Song),
    DownloadDone(Result<(), String>),

    // ── Add-to-playlist overlay ───────────────────────────────────────────────
    OpenAddToPlaylist(Song),
    CloseAddToPlaylist,
    NewPlaylistNameInput(String),
    AddToPlaylist(String),
    CreatePlaylistAndAdd,
    PlaylistCreatedThenAdd(String, Result<serde_json::Value, String>),
    AddToPlaylistDone(Result<(), String>),

    // ── Search ────────────────────────────────────────────────────────────────
    SearchInput(String),
    SubmitSearch,
    SearchLoaded(Result<SearchResult, String>),

    // ── Settings ──────────────────────────────────────────────────────────────
    SelectTheme(String),
    SetCrossfadeEnabled(bool),
    SetCrossfadeDuration(f32),
    SetGapless(bool),
    SetReplayGain(bool),
    SetAutoContinue(bool),
    SetBitPerfect(String),
    SetSettingsCategory(SettingsCategory),
    SetDownloadFormat(String),
    SetLastfmEnabled(bool),
    SetLastfmKey(String),
    SetLastfmSecret(String),
    SetListenbrainzEnabled(bool),
    SetListenbrainzToken(String),
    SetLrclibEnabled(bool),
    SetLyricsWordFill(bool),
    SetDecorations(bool),
    WipeCoverCache,
    DeleteSettings,
    Logout,

    // ── Equalizer ─────────────────────────────────────────────────────────────
    SetEqEnabled(bool),
    SetEqProfile(String),
    EqBandChanged(usize, f32),
    EqNewProfileInput(String),
    SaveEqProfile,
    DeleteEqProfile(String),

    // ── Mix ───────────────────────────────────────────────────────────────────
    GenerateMix(Energy),
    MixFetched(Energy, Result<Vec<Song>, String>),

    // ── Transport ─────────────────────────────────────────────────────────────
    TogglePlay,
    Next,
    Prev,
    ToggleShuffle,
    CycleRepeat,
    SetVolume(f32),
    SeekTo(f32),
    TogglePanel(Panel),
    SetVizMode(VizMode),
    LyricsLoaded(String, Result<Option<LyricsResult>, String>),
    SimilarLoaded(String, Result<Vec<SimilarMatch>, String>),
    PlayQueueIndex(usize),
    PlaybackDone(Result<(), String>),

    // ── Resume-queue prompt ───────────────────────────────────────────────────
    PlayQueueFetched(Result<Option<RemotePlayQueue>, String>),
    ResumeQueue,
    DismissResume,

    // ── Account switcher ──────────────────────────────────────────────────────
    ToggleAccountSwitcher,
    SwitchAccount(SavedAccount),
    AddAccount,

    // ── Recap ─────────────────────────────────────────────────────────────────
    SetRecapRange(RecapRange),
    RecapNext,
    RecapPrev,

    // ── Listening stats ───────────────────────────────────────────────────────
    ExportStats(String),
    ExportDone(Result<bool, String>),

    // ── Genre browsing ────────────────────────────────────────────────────────
    GenresLoaded(Result<Vec<Genre>, String>),
    GenreSongsLoaded(Result<Vec<Song>, String>),
    PlayGenreAt(usize),

    // ── Album download ────────────────────────────────────────────────────────
    DownloadAlbum,

    // ── Offline / local library ───────────────────────────────────────────────
    PlayLocalAlbumAt(usize),
}

pub struct App {
    backend: Backend,
    #[allow(dead_code)]
    themes: Vec<ThemeEntry>,
    #[allow(dead_code)]
    theme_id: String,
    tokens: Tokens,

    // ── Auth / onboarding ─────────────────────────────────────────────────────
    authed: bool,
    connecting: bool,
    server_input: String,
    username_input: String,
    password_input: String,
    connect_error: Option<String>,

    view: View,
    nav_stack: Vec<View>,

    // ── Library data ──────────────────────────────────────────────────────────
    albums: Vec<Album>,
    albums_scroll: f32,
    home_recent: Vec<Album>,
    home_newest: Vec<Album>,
    home_random: Vec<Album>,
    album_detail: Option<AlbumTracks>,
    album_detail_id: Option<String>,
    artists: Vec<Artist>,
    artist_detail: Option<ArtistDetails>,
    artist_detail_id: Option<String>,
    artist_info: Option<ArtistInfo>,
    similar_artists: Vec<String>,
    playlists: Vec<serde_json::Value>,
    playlist_detail: Option<PlaylistTracks>,
    playlist_detail_id: Option<String>,
    cover_cache: HashMap<String, ImageHandle>,

    // ── Playback mirror ───────────────────────────────────────────────────────
    queue: Vec<Song>,
    queue_idx: i32,
    playback_state: PlaybackState,
    position: f64,
    duration: Option<f64>,
    current_player_id: Option<String>,
    volume: f32,
    repeat_one: bool,
    repeat_all: bool,
    shuffle: bool,
    right_panel: Option<Panel>,
    visualizer_mode: VizMode,
    lyrics: Option<LyricsResult>,
    lyrics_track_id: Option<String>,
    similar_results: Vec<SimilarMatch>,
    similar_track_id: Option<String>,
    eq_state: Option<EqState>,

    // ── Playback settings (mirrored for the Settings UI) ──────────────────────
    crossfade_enabled: bool,
    crossfade_duration: f32,
    gapless_enabled: bool,
    replay_gain_enabled: bool,
    auto_continue: bool,
    bit_perfect_mode: String,

    // ── Settings UI (two-column categories + service prefs) ───────────────────
    settings_category: SettingsCategory,
    download_format: String,
    lastfm_enabled: bool,
    lastfm_key: String,
    lastfm_secret: String,
    listenbrainz_enabled: bool,
    listenbrainz_token: String,
    lrclib_enabled: bool,
    lyrics_word_fill: bool,
    window_decorations: bool,

    // ── Search ────────────────────────────────────────────────────────────────
    search_query: String,
    search_results: Option<SearchResult>,

    // ── Add-to-playlist overlay ───────────────────────────────────────────────
    add_to_playlist_song: Option<Song>,
    new_playlist_name: String,

    // ── Resume-queue prompt ───────────────────────────────────────────────────
    resume_queue: Option<RemotePlayQueue>,

    // ── Account switcher ──────────────────────────────────────────────────────
    accounts: Vec<SavedAccount>,
    show_account_switcher: bool,

    // ── Recap ─────────────────────────────────────────────────────────────────
    recap: Option<crate::db::RecapStats>,
    recap_range: RecapRange,
    recap_card: usize,

    // ── Listening stats ───────────────────────────────────────────────────────
    history_summary: Option<crate::db::PlayHistorySummary>,

    // ── Genre browsing ────────────────────────────────────────────────────────
    genres: Vec<Genre>,
    genre_songs: Vec<Song>,
    genre_detail_name: Option<String>,

    // ── Equalizer profile editing ─────────────────────────────────────────────
    eq_new_profile_name: String,

    // ── Offline / local library ───────────────────────────────────────────────
    local_albums: Vec<Album>,
    local_album: Option<LocalAlbumTracks>,
    local_album_id: Option<String>,
}

impl App {
    pub fn new(backend: Backend) -> Self {
        let cfg = Config::load();
        let themes = crate::commands::themes::list_themes();
        let theme_id = cfg.theme_id.clone().unwrap_or_else(|| "firmium".to_string());
        let tokens = themes
            .iter()
            .find(|t| t.id == theme_id)
            .map(Tokens::from_entry)
            .unwrap_or_default();
        let volume = cfg.volume.unwrap_or(0.8).clamp(0.0, 1.0);

        // Push persisted playback settings into the backend queue state.
        crate::commands::queue::init_playback_settings(
            &backend.queue_state, volume, false, 5.0, "linear".to_string(), true, true, false,
        );

        // Attempt auto-login from saved server + keyring password.
        let mut connecting = false;
        let (server_input, username_input) = match (&cfg.server, &cfg.username) {
            (Some(server), Some(user)) => {
                if let Ok(pass) = crate::commands::credentials::get_password(Some(server), user) {
                    crate::commands::subsonic::set_connection(
                        &backend.app_state,
                        Some(server.clone()),
                        Some(user.clone()),
                        Some(pass),
                    );
                    connecting = true;
                }
                (server.clone(), user.clone())
            }
            _ => (String::new(), String::new()),
        };

        // Service credentials live in the OS keyring (service "firmium-desktop"),
        // never in config.toml. Presence of a value is what enables the feature.
        let keyring_read = |key: &str| {
            crate::commands::credentials::get_password(Some("firmium-desktop"), key)
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        let lastfm_key = keyring_read("lastfm_key");
        let lastfm_secret = keyring_read("lastfm_secret");
        let listenbrainz_token = keyring_read("listenbrainz_token");

        Self {
            backend,
            themes,
            theme_id,
            tokens,
            authed: false,
            connecting,
            server_input,
            username_input,
            password_input: String::new(),
            connect_error: None,
            view: View::Home,
            nav_stack: Vec::new(),
            albums: Vec::new(),
            albums_scroll: 0.0,
            home_recent: Vec::new(),
            home_newest: Vec::new(),
            home_random: Vec::new(),
            album_detail: None,
            album_detail_id: None,
            artists: Vec::new(),
            artist_detail: None,
            artist_detail_id: None,
            artist_info: None,
            similar_artists: Vec::new(),
            playlists: Vec::new(),
            playlist_detail: None,
            playlist_detail_id: None,
            cover_cache: HashMap::new(),
            queue: Vec::new(),
            queue_idx: -1,
            playback_state: PlaybackState::Stopped,
            position: 0.0,
            duration: None,
            current_player_id: None,
            volume,
            repeat_one: false,
            repeat_all: false,
            shuffle: false,
            right_panel: None,
            visualizer_mode: VizMode::Bars,
            lyrics: None,
            lyrics_track_id: None,
            similar_results: Vec::new(),
            similar_track_id: None,
            eq_state: None,
            crossfade_enabled: false,
            crossfade_duration: 5.0,
            gapless_enabled: true,
            replay_gain_enabled: true,
            auto_continue: false,
            bit_perfect_mode: "relaxed".to_string(),
            settings_category: SettingsCategory::Appearance,
            download_format: cfg.download_format.clone().unwrap_or_else(|| "raw".to_string()),
            lastfm_enabled: !lastfm_key.is_empty(),
            lastfm_key,
            lastfm_secret,
            listenbrainz_enabled: !listenbrainz_token.is_empty(),
            listenbrainz_token,
            lrclib_enabled: cfg.lrclib_enabled.unwrap_or(true),
            lyrics_word_fill: cfg.lyrics_word_fill.unwrap_or(false),
            window_decorations: cfg.window_decorations.unwrap_or(true),
            search_query: String::new(),
            search_results: None,
            add_to_playlist_song: None,
            new_playlist_name: String::new(),
            resume_queue: None,
            accounts: cfg.accounts,
            show_account_switcher: false,
            recap: None,
            recap_range: RecapRange::Month,
            recap_card: 0,
            history_summary: None,
            genres: Vec::new(),
            genre_songs: Vec::new(),
            genre_detail_name: None,
            eq_new_profile_name: String::new(),
            local_albums: Vec::new(),
            local_album: None,
            local_album_id: None,
        }
    }

    pub fn theme(&self) -> Theme {
        self.tokens.iced_theme()
    }

    /// Auto-login validation task if saved credentials were loaded at startup.
    pub fn initial_task(&self) -> Task<Message> {
        if self.backend.app_state.connection.read().server.is_some() {
            Task::perform(
                crate::commands::subsonic::validate_connection(self.backend.app_state.clone()),
                Message::Connected,
            )
        } else {
            Task::none()
        }
    }

    fn save_config(&self) {
        let (server, username) = {
            let conn = self.backend.app_state.connection.read();
            (conn.server.clone(), conn.username.clone())
        };
        Config {
            server,
            username,
            theme_id: Some(self.theme_id.clone()),
            volume: Some(self.volume),
            accounts: self.accounts.clone(),
            download_format: Some(self.download_format.clone()),
            lrclib_enabled: Some(self.lrclib_enabled),
            lyrics_word_fill: Some(self.lyrics_word_fill),
            window_decorations: Some(self.window_decorations),
        }
        .save();
    }

    /// Upsert the active connection into the saved-accounts list.
    fn remember_current_account(&mut self) {
        let (server, username) = {
            let conn = self.backend.app_state.connection.read();
            (conn.server.clone(), conn.username.clone())
        };
        if let (Some(server), Some(username)) = (server, username) {
            let acct = SavedAccount { server, username };
            if !self.accounts.contains(&acct) {
                self.accounts.push(acct);
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
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
                        Task::perform(crate::commands::subsonic::get_album_tracks(state, id), Message::AlbumTracksLoaded)
                    }
                    View::ArtistDetail(id) if self.artist_detail_id.as_deref() != Some(id.as_str()) => {
                        self.artist_detail = None;
                        self.artist_info = None;
                        self.similar_artists.clear();
                        self.artist_detail_id = Some(id.clone());
                        let artist_name = self
                            .artists
                            .iter()
                            .find(|a| a.id == *id)
                            .map(|a| a.name.clone())
                            .unwrap_or_default();
                        Task::batch([
                            Task::perform(crate::commands::subsonic::get_artist_details(state.clone(), id.clone()), Message::ArtistDetailLoaded),
                            Task::perform(crate::commands::subsonic::get_artist_info(state.clone(), id.clone(), self.lastfm_key.clone(), artist_name), Message::ArtistInfoLoaded),
                            Task::perform(crate::commands::subsonic::get_similar_artists(state, id, None), Message::SimilarArtistsLoaded),
                        ])
                    }
                    View::PlaylistDetail(id) if self.playlist_detail_id.as_deref() != Some(id.as_str()) => {
                        self.playlist_detail = None;
                        self.playlist_detail_id = Some(id.clone());
                        Task::perform(crate::commands::subsonic::get_playlist_tracks(state, id), Message::PlaylistTracksLoaded)
                    }
                    View::Artists if self.artists.is_empty() => {
                        Task::perform(crate::commands::subsonic::get_artists(state), Message::ArtistsLoaded)
                    }
                    View::Playlists if self.playlists.is_empty() => {
                        Task::perform(crate::commands::subsonic::get_playlists(state), Message::PlaylistsLoaded)
                    }
                    View::Recap => self.compute_recap(),
                    View::GenreDetail(name) if self.genre_detail_name.as_deref() != Some(name.as_str()) => {
                        self.genre_songs.clear();
                        self.genre_detail_name = Some(name.clone());
                        Task::perform(crate::commands::subsonic::get_songs_by_genre(state, name, None), Message::GenreSongsLoaded)
                    }
                    View::Local => {
                        // Local library scan is synchronous + cached after first run.
                        self.local_albums = crate::commands::local_library::get_local_albums(&self.backend.app_state)
                            .unwrap_or_default();
                        Task::none()
                    }
                    View::LocalAlbumDetail(id) if self.local_album_id.as_deref() != Some(id.as_str()) => {
                        self.local_album_id = Some(id.clone());
                        self.local_album = crate::commands::local_library::get_local_album_tracks(&self.backend.app_state, id).ok();
                        Task::none()
                    }
                    View::Settings => {
                        self.load_history_summary();
                        Task::none()
                    }
                    View::Home if self.home_newest.is_empty() => Task::batch([
                        Task::perform(crate::commands::subsonic::get_recent_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Recent, r)),
                        Task::perform(crate::commands::subsonic::get_newest_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Newest, r)),
                        Task::perform(crate::commands::subsonic::get_random_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Random, r)),
                        Task::perform(crate::commands::subsonic::get_genres_list(state), Message::GenresLoaded),
                    ]),
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
                Task::batch([self.maybe_fetch_lyrics(), self.maybe_fetch_similar()])
            }
            Message::VisualizerTick => Task::none(),

            // ── Onboarding ────────────────────────────────────────────────────
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
            Message::Connect => {
                let server = self.server_input.trim().trim_end_matches('/').to_string();
                let user = self.username_input.trim().to_string();
                let pass = self.password_input.clone();
                if server.is_empty() || user.is_empty() {
                    self.connect_error = Some("Server URL and username are required".to_string());
                    return Task::none();
                }
                self.connecting = true;
                self.connect_error = None;
                crate::commands::subsonic::set_connection(
                    &self.backend.app_state,
                    Some(server.clone()),
                    Some(user.clone()),
                    Some(pass.clone()),
                );
                let _ = crate::commands::credentials::save_password(Some(&server), &user, &pass);
                Task::perform(
                    crate::commands::subsonic::validate_connection(self.backend.app_state.clone()),
                    Message::Connected,
                )
            }
            Message::Connected(Ok(())) => {
                self.authed = true;
                self.connecting = false;
                self.password_input.clear();
                self.remember_current_account();
                self.save_config();
                let s = self.backend.app_state.clone();
                Task::batch([
                    Task::perform(crate::commands::subsonic::get_albums(s.clone()), Message::AlbumsLoaded),
                    Task::perform(crate::commands::subsonic::get_recent_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Recent, r)),
                    Task::perform(crate::commands::subsonic::get_newest_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Newest, r)),
                    Task::perform(crate::commands::subsonic::get_random_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Random, r)),
                    Task::perform(crate::commands::subsonic::get_genres_list(s.clone()), Message::GenresLoaded),
                    Task::perform(crate::commands::subsonic::get_play_queue(s), Message::PlayQueueFetched),
                ])
            }
            Message::Connected(Err(e)) => {
                self.connecting = false;
                self.connect_error = Some(e);
                Task::none()
            }

            // ── Data ──────────────────────────────────────────────────────────
            Message::AlbumsLoaded(Ok(albums)) => {
                self.albums = albums;
                self.load_covers()
            }
            Message::AlbumsLoaded(Err(e)) => {
                eprintln!("get_albums failed: {e}");
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
                eprintln!("home albums failed: {e}");
                Task::none()
            }
            Message::CoverLoaded(id, Ok(path)) => {
                self.cover_cache.insert(id, ImageHandle::from_path(path));
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
                eprintln!("get_album_tracks failed: {e}");
                Task::none()
            }
            Message::ArtistsLoaded(Ok(a)) => {
                self.artists = a;
                Task::none()
            }
            Message::ArtistsLoaded(Err(e)) => {
                eprintln!("get_artists failed: {e}");
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
                eprintln!("get_artist_info failed: {e}");
                Task::none()
            }
            Message::SimilarArtistsLoaded(Ok(names)) => {
                self.similar_artists = names;
                Task::none()
            }
            Message::SimilarArtistsLoaded(Err(e)) => {
                eprintln!("get_similar_artists failed: {e}");
                Task::none()
            }
            Message::ArtistDetailLoaded(Err(e)) => {
                eprintln!("get_artist_details failed: {e}");
                Task::none()
            }
            Message::PlaylistsLoaded(Ok(p)) => {
                self.playlists = p;
                Task::none()
            }
            Message::PlaylistsLoaded(Err(e)) => {
                eprintln!("get_playlists failed: {e}");
                Task::none()
            }
            Message::PlaylistTracksLoaded(Ok(pt)) => {
                let ids: Vec<String> = pt.tracks.iter().filter_map(|s| s.cover_art_id.clone()).collect();
                self.playlist_detail = Some(pt);
                self.load_cover_ids(ids)
            }
            Message::PlaylistTracksLoaded(Err(e)) => {
                eprintln!("get_playlist_tracks failed: {e}");
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
            Message::PlayPlaylistAt(idx) => {
                if let Some(pt) = &self.playlist_detail {
                    let songs = pt.tracks.clone();
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
            Message::OpenAddToPlaylist(song) => {
                self.add_to_playlist_song = Some(song);
                self.new_playlist_name.clear();
                // Lazily load the playlist list the first time the overlay opens.
                if self.playlists.is_empty() {
                    Task::perform(
                        crate::commands::subsonic::get_playlists(self.backend.app_state.clone()),
                        Message::PlaylistsLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::CloseAddToPlaylist => {
                self.add_to_playlist_song = None;
                Task::none()
            }
            Message::NewPlaylistNameInput(s) => {
                self.new_playlist_name = s;
                Task::none()
            }
            Message::AddToPlaylist(playlist_id) => {
                if let Some(song) = self.add_to_playlist_song.take() {
                    Task::perform(
                        crate::commands::subsonic::update_playlist(
                            self.backend.app_state.clone(),
                            playlist_id,
                            None,
                            None,
                            vec![song.id],
                            Vec::new(),
                        ),
                        Message::AddToPlaylistDone,
                    )
                } else {
                    Task::none()
                }
            }
            Message::CreatePlaylistAndAdd => {
                let name = self.new_playlist_name.trim().to_string();
                match self.add_to_playlist_song.take() {
                    Some(song) if !name.is_empty() => {
                        let sid = song.id;
                        Task::perform(
                            crate::commands::subsonic::create_playlist(self.backend.app_state.clone(), name),
                            move |res| Message::PlaylistCreatedThenAdd(sid.clone(), res),
                        )
                    }
                    // Nothing to do if the name is blank; keep the overlay open.
                    other => {
                        self.add_to_playlist_song = other;
                        Task::none()
                    }
                }
            }
            Message::PlaylistCreatedThenAdd(song_id, Ok(playlist)) => {
                let id = playlist.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                if let Some(id) = id {
                    Task::perform(
                        crate::commands::subsonic::update_playlist(
                            self.backend.app_state.clone(),
                            id,
                            None,
                            None,
                            vec![song_id],
                            Vec::new(),
                        ),
                        Message::AddToPlaylistDone,
                    )
                } else {
                    eprintln!("create_playlist returned no id");
                    Task::none()
                }
            }
            Message::PlaylistCreatedThenAdd(_, Err(e)) => {
                eprintln!("create_playlist failed: {e}");
                Task::none()
            }
            Message::AddToPlaylistDone(Ok(())) => {
                // Refresh playlists so new entries / counts reflect the change.
                Task::perform(
                    crate::commands::subsonic::get_playlists(self.backend.app_state.clone()),
                    Message::PlaylistsLoaded,
                )
            }
            Message::AddToPlaylistDone(Err(e)) => {
                eprintln!("add to playlist failed: {e}");
                Task::none()
            }

            // ── Search ──────────────────────────────────────────────────────────
            Message::SearchInput(q) => {
                self.search_query = q;
                Task::none()
            }
            Message::SubmitSearch => {
                let q = self.search_query.trim().to_string();
                if q.is_empty() {
                    return Task::none();
                }
                Task::perform(
                    crate::commands::subsonic::search(self.backend.app_state.clone(), q),
                    Message::SearchLoaded,
                )
            }
            Message::SearchLoaded(Ok(res)) => {
                let ids: Vec<String> = res
                    .albums
                    .iter()
                    .filter_map(|a| a.cover_art_id.clone())
                    .chain(res.songs.iter().filter_map(|s| s.cover_art_id.clone()))
                    .collect();
                self.search_results = Some(res);
                self.load_cover_ids(ids)
            }
            Message::SearchLoaded(Err(e)) => {
                eprintln!("search failed: {e}");
                Task::none()
            }

            // ── Settings ────────────────────────────────────────────────────────
            Message::SelectTheme(id) => {
                if let Some(entry) = self.themes.iter().find(|t| t.id == id) {
                    self.tokens = Tokens::from_entry(entry);
                    self.theme_id = id;
                    self.save_config();
                }
                Task::none()
            }
            Message::SetCrossfadeEnabled(on) => {
                self.crossfade_enabled = on;
                crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, on, self.crossfade_duration);
                Task::none()
            }
            Message::SetCrossfadeDuration(secs) => {
                self.crossfade_duration = secs;
                crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, self.crossfade_enabled, secs);
                Task::none()
            }
            Message::SetGapless(on) => {
                self.gapless_enabled = on;
                crate::commands::queue::set_gapless_enabled(&self.backend.bus, &self.backend.queue_state, on);
                Task::none()
            }
            Message::SetReplayGain(on) => {
                self.replay_gain_enabled = on;
                crate::commands::queue::set_replay_gain_enabled(&self.backend.queue_state, &self.backend.audio_player, on);
                Task::none()
            }
            Message::SetAutoContinue(on) => {
                self.auto_continue = on;
                crate::commands::queue::set_auto_continue(&self.backend.queue_state, on);
                Task::none()
            }
            Message::SetBitPerfect(mode) => {
                self.bit_perfect_mode = mode.clone();
                self.backend.audio_player.set_bit_perfect_mode(mode);
                Task::none()
            }

            // ── Settings UI ─────────────────────────────────────────────────────
            Message::SetSettingsCategory(cat) => {
                self.settings_category = cat;
                Task::none()
            }
            Message::SetDownloadFormat(fmt) => {
                self.download_format = fmt;
                self.save_config();
                Task::none()
            }
            Message::SetLastfmEnabled(on) => {
                self.lastfm_enabled = on;
                if !on {
                    let _ = crate::commands::credentials::delete_password(Some("firmium-desktop"), "lastfm_key");
                    let _ = crate::commands::credentials::delete_password(Some("firmium-desktop"), "lastfm_secret");
                    self.lastfm_key.clear();
                    self.lastfm_secret.clear();
                }
                Task::none()
            }
            Message::SetLastfmKey(key) => {
                self.lastfm_key = key.clone();
                let _ = crate::commands::credentials::save_password(Some("firmium-desktop"), "lastfm_key", &key);
                Task::none()
            }
            Message::SetLastfmSecret(secret) => {
                self.lastfm_secret = secret.clone();
                let _ = crate::commands::credentials::save_password(Some("firmium-desktop"), "lastfm_secret", &secret);
                Task::none()
            }
            Message::SetListenbrainzEnabled(on) => {
                self.listenbrainz_enabled = on;
                if !on {
                    let _ = crate::commands::credentials::delete_password(Some("firmium-desktop"), "listenbrainz_token");
                    self.listenbrainz_token.clear();
                }
                Task::none()
            }
            Message::SetListenbrainzToken(token) => {
                self.listenbrainz_token = token.clone();
                let _ = crate::commands::credentials::save_password(Some("firmium-desktop"), "listenbrainz_token", &token);
                Task::none()
            }
            Message::SetLrclibEnabled(on) => {
                self.lrclib_enabled = on;
                self.save_config();
                Task::none()
            }
            Message::SetLyricsWordFill(on) => {
                self.lyrics_word_fill = on;
                self.save_config();
                Task::none()
            }
            Message::SetDecorations(on) => {
                self.window_decorations = on;
                self.save_config();
                // winit only offers a toggle (no absolute set); boot applies the
                // persisted value, so a single toggle here keeps UI and window in sync.
                iced::window::latest().then(|maybe_id| match maybe_id {
                    Some(id) => iced::window::toggle_decorations(id),
                    None => Task::none(),
                })
            }
            Message::WipeCoverCache => {
                let _ = crate::commands::cover_cache::clear_cover_cache();
                self.cover_cache.clear();
                Task::none()
            }
            Message::DeleteSettings => {
                // Reset preference fields to defaults (connection/account untouched).
                self.download_format = "raw".to_string();
                self.lrclib_enabled = true;
                self.lyrics_word_fill = false;
                self.window_decorations = true;
                self.bit_perfect_mode = "relaxed".to_string();
                self.crossfade_enabled = false;
                self.crossfade_duration = 5.0;
                self.gapless_enabled = true;
                self.replay_gain_enabled = true;
                self.auto_continue = false;
                self.backend.audio_player.set_bit_perfect_mode(self.bit_perfect_mode.clone());
                crate::commands::queue::set_gapless_enabled(&self.backend.bus, &self.backend.queue_state, self.gapless_enabled);
                crate::commands::queue::set_replay_gain_enabled(&self.backend.queue_state, &self.backend.audio_player, self.replay_gain_enabled);
                crate::commands::queue::set_auto_continue(&self.backend.queue_state, self.auto_continue);
                crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, self.crossfade_enabled, self.crossfade_duration);
                self.save_config();
                Task::none()
            }
            Message::Logout => {
                crate::commands::subsonic::set_connection(&self.backend.app_state, None, None, None);
                self.authed = false;
                self.albums.clear();
                self.search_results = None;
                self.save_config();
                Task::none()
            }

            // ── Equalizer ───────────────────────────────────────────────────────
            Message::SetEqEnabled(on) => {
                let _ = crate::commands::equalizer::set_eq_enabled(&self.backend.audio_player, on);
                self.eq_state = Some(crate::commands::equalizer::get_eq_state());
                Task::none()
            }
            Message::SetEqProfile(name) => {
                let device = self
                    .eq_state
                    .as_ref()
                    .and_then(|e| e.default_device.clone())
                    .unwrap_or_default();
                let _ = crate::commands::equalizer::set_eq_active_profile(&self.backend.audio_player, device, name);
                self.eq_state = Some(crate::commands::equalizer::get_eq_state());
                Task::none()
            }
            Message::EqBandChanged(idx, gain) => {
                if let Some(eq) = &mut self.eq_state {
                    if let Some(active) = eq.active_profile.clone() {
                        if let Some(p) = eq.profiles.iter_mut().find(|p| p.name == active) {
                            if let Some(b) = p.bands.get_mut(idx) {
                                b.gain = gain;
                            }
                            let bands = p.bands.clone();
                            let _ = crate::commands::equalizer::set_eq_bands(&self.backend.audio_player, active, bands);
                        }
                    }
                }
                Task::none()
            }
            Message::EqNewProfileInput(s) => {
                self.eq_new_profile_name = s;
                Task::none()
            }
            Message::SaveEqProfile => {
                let name = self.eq_new_profile_name.trim().to_string();
                if !name.is_empty() {
                    // Save the active profile's current bands under the new name.
                    if let Some(eq) = &self.eq_state {
                        if let Some(p) = eq
                            .active_profile
                            .as_ref()
                            .and_then(|a| eq.profiles.iter().find(|p| &p.name == a))
                        {
                            let _ = crate::commands::equalizer::save_eq_profile(
                                &self.backend.audio_player,
                                name,
                                p.kind.clone(),
                                p.bands.clone(),
                            );
                        }
                    }
                    self.eq_new_profile_name.clear();
                    self.eq_state = Some(crate::commands::equalizer::get_eq_state());
                }
                Task::none()
            }
            Message::DeleteEqProfile(name) => {
                let _ = crate::commands::equalizer::delete_eq_profile(&self.backend.audio_player, name);
                self.eq_state = Some(crate::commands::equalizer::get_eq_state());
                Task::none()
            }

            // ── Mix ─────────────────────────────────────────────────────────────
            Message::GenerateMix(energy) => Task::perform(
                crate::commands::subsonic::get_random_songs(self.backend.app_state.clone(), Some(200), None),
                move |res| Message::MixFetched(energy, res),
            ),
            Message::MixFetched(energy, Ok(songs)) => {
                let mix = filter_energy(songs, energy);
                if mix.is_empty() {
                    return Task::none();
                }
                Task::perform(
                    crate::commands::queue::set_queue(
                        self.backend.queue_state.clone(),
                        self.backend.app_state.clone(),
                        self.backend.audio_player.clone(),
                        mix,
                        0,
                    ),
                    Message::PlaybackDone,
                )
            }
            Message::MixFetched(_, Err(e)) => {
                eprintln!("mix fetch failed: {e}");
                Task::none()
            }

            // ── Transport ─────────────────────────────────────────────────────
            Message::TogglePlay => Task::perform(
                crate::commands::queue::toggle_play(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                ),
                Message::PlaybackDone,
            ),
            Message::Next => Task::perform(
                crate::commands::queue::queue_next(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                ),
                Message::PlaybackDone,
            ),
            Message::Prev => Task::perform(
                crate::commands::queue::queue_prev(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                ),
                Message::PlaybackDone,
            ),
            Message::ToggleShuffle => {
                crate::commands::queue::toggle_shuffle(&self.backend.bus, &self.backend.queue_state);
                Task::none()
            }
            Message::CycleRepeat => {
                let (one, all) = if !self.repeat_one && !self.repeat_all {
                    (false, true)
                } else if self.repeat_all {
                    (true, false)
                } else {
                    (false, false)
                };
                crate::commands::queue::set_repeat_mode(&self.backend.bus, &self.backend.queue_state, one, all);
                Task::none()
            }
            Message::SetVolume(v) => {
                self.volume = v;
                crate::commands::queue::set_queue_volume(
                    &self.backend.bus,
                    &self.backend.queue_state,
                    &self.backend.audio_player,
                    v,
                );
                Task::none()
            }
            Message::SeekTo(secs) => {
                self.position = secs as f64;
                let _ = crate::commands::queue::seek_queue(&self.backend.queue_state, &self.backend.audio_player, secs as f64);
                Task::none()
            }
            Message::TogglePanel(p) => {
                self.right_panel = if self.right_panel == Some(p) { None } else { Some(p) };
                self.backend
                    .audio_player
                    .set_visualizer_enabled(self.right_panel == Some(Panel::Visualizer));
                if self.right_panel == Some(Panel::Equalizer) {
                    self.eq_state = Some(crate::commands::equalizer::get_eq_state());
                }
                Task::batch([self.maybe_fetch_lyrics(), self.maybe_fetch_similar()])
            }
            Message::SetVizMode(m) => {
                self.visualizer_mode = m;
                Task::none()
            }
            Message::LyricsLoaded(track_id, res) => {
                if self.lyrics_track_id.as_deref() == Some(track_id.as_str()) {
                    self.lyrics = res.ok().flatten();
                }
                Task::none()
            }
            Message::SimilarLoaded(track_id, res) => {
                if self.similar_track_id.as_deref() == Some(track_id.as_str()) {
                    self.similar_results = res.unwrap_or_default();
                    let ids: Vec<String> = self
                        .similar_results
                        .iter()
                        .filter_map(|m| m.song.cover_art_id.clone())
                        .collect();
                    return self.load_cover_ids(ids);
                }
                Task::none()
            }
            Message::PlayQueueIndex(idx) => Task::perform(
                crate::commands::queue::play_queue_index(
                    self.backend.queue_state.clone(),
                    self.backend.app_state.clone(),
                    self.backend.audio_player.clone(),
                    idx,
                ),
                Message::PlaybackDone,
            ),
            Message::PlaybackDone(Err(e)) => {
                eprintln!("transport command failed: {e}");
                Task::none()
            }
            Message::PlaybackDone(Ok(())) => Task::none(),

            // ── Resume-queue prompt ─────────────────────────────────────────────
            Message::PlayQueueFetched(Ok(Some(q))) => {
                // Only prompt for a queue that isn't already playing locally.
                if !q.entries.is_empty() && self.queue.is_empty() {
                    self.resume_queue = Some(q);
                }
                Task::none()
            }
            Message::PlayQueueFetched(Ok(None)) => Task::none(),
            Message::PlayQueueFetched(Err(e)) => {
                eprintln!("get_play_queue failed: {e}");
                Task::none()
            }
            Message::ResumeQueue => {
                let Some(q) = self.resume_queue.take() else { return Task::none() };
                let start_idx = q
                    .current
                    .as_deref()
                    .and_then(|cur| q.entries.iter().position(|s| s.id == cur))
                    .unwrap_or(0);
                let pos = q.position_ms.unwrap_or(0).max(0) as f64 / 1000.0;
                Task::perform(
                    crate::commands::queue::set_queue(
                        self.backend.queue_state.clone(),
                        self.backend.app_state.clone(),
                        self.backend.audio_player.clone(),
                        q.entries,
                        start_idx,
                    ),
                    move |res| match res {
                        // Seek to the saved offset once the track is loaded.
                        Ok(()) => Message::SeekTo(pos as f32),
                        Err(e) => Message::PlaybackDone(Err(e)),
                    },
                )
            }
            Message::DismissResume => {
                self.resume_queue = None;
                Task::none()
            }

            // ── Account switcher ────────────────────────────────────────────────
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
                match crate::commands::credentials::get_password(Some(&acct.server), &acct.username) {
                    Ok(pass) => {
                        crate::commands::subsonic::set_connection(
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
                            crate::commands::subsonic::validate_connection(self.backend.app_state.clone()),
                            Message::Connected,
                        )
                    }
                    Err(_) => {
                        // Password no longer in keyring — bounce to setup, prefilled.
                        self.server_input = acct.server.clone();
                        self.username_input = acct.username.clone();
                        self.authed = false;
                        Task::none()
                    }
                }
            }
            Message::AddAccount => {
                self.show_account_switcher = false;
                self.authed = false;
                self.server_input.clear();
                self.username_input.clear();
                self.password_input.clear();
                Task::none()
            }

            // ── Recap ───────────────────────────────────────────────────────────
            Message::SetRecapRange(r) => {
                self.recap_range = r;
                self.compute_recap()
            }
            Message::RecapNext => {
                self.recap_card = (self.recap_card + 1).min(RECAP_CARDS - 1);
                Task::none()
            }
            Message::RecapPrev => {
                self.recap_card = self.recap_card.saturating_sub(1);
                Task::none()
            }

            // ── Listening stats ─────────────────────────────────────────────────
            Message::ExportStats(format) => {
                let Some(history) = &self.backend.history else { return Task::none() };
                // Serialize synchronously (DB handle isn't Send); the async task
                // only does the file dialog + write on the owned string.
                let contents = match crate::commands::stats::export_play_history(history, format.clone()) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("export_play_history failed: {e}");
                        return Task::none();
                    }
                };
                let ext = if format == "json" { "json" } else { "csv" };
                Task::perform(
                    save_export(format!("firmium-history.{ext}"), ext.to_string(), contents),
                    Message::ExportDone,
                )
            }
            Message::ExportDone(Ok(_)) => Task::none(),
            Message::ExportDone(Err(e)) => {
                eprintln!("export save failed: {e}");
                Task::none()
            }

            // ── Genre browsing ──────────────────────────────────────────────────
            Message::GenresLoaded(Ok(g)) => {
                self.genres = g;
                Task::none()
            }
            Message::GenresLoaded(Err(e)) => {
                eprintln!("get_genres_list failed: {e}");
                Task::none()
            }
            Message::GenreSongsLoaded(Ok(songs)) => {
                let ids: Vec<String> = songs.iter().filter_map(|s| s.cover_art_id.clone()).collect();
                self.genre_songs = songs;
                self.load_cover_ids(ids)
            }
            Message::GenreSongsLoaded(Err(e)) => {
                eprintln!("get_songs_by_genre failed: {e}");
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
            Message::PlayLocalAlbumAt(idx) => match &self.local_album {
                Some(la) if !la.tracks.is_empty() => Task::perform(
                    crate::commands::queue::set_queue(
                        self.backend.queue_state.clone(),
                        self.backend.app_state.clone(),
                        self.backend.audio_player.clone(),
                        la.tracks.clone(),
                        idx,
                    ),
                    Message::PlaybackDone,
                ),
                _ => Task::none(),
            },
        }
    }

    /// Fetch the orb palette for the current track's cover when it changes.
    /// Refresh the play-history summary shown on the Settings page.
    fn load_history_summary(&mut self) {
        if let Some(history) = &self.backend.history {
            self.history_summary = crate::commands::stats::get_play_history_summary(history).ok();
        }
    }

    /// Run the recap aggregation for the current range and queue cover loads.
    fn compute_recap(&mut self) -> Task<Message> {
        self.recap_card = 0;
        let Some(history) = &self.backend.history else {
            self.recap = None;
            return Task::none();
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let from = self.recap_range.from_ts(now);
        match crate::commands::stats::get_recap_stats(history, from, now) {
            Ok(stats) => {
                let mut ids: Vec<String> = Vec::new();
                ids.extend(stats.top_tracks.iter().filter_map(|s| s.cover_art_id.clone()));
                ids.extend(stats.top_albums.iter().filter_map(|s| s.cover_art_id.clone()));
                if let Some(d) = &stats.biggest_discovery {
                    if let Some(c) = &d.cover_art_id {
                        ids.push(c.clone());
                    }
                }
                self.recap = Some(stats);
                self.load_cover_ids(ids)
            }
            Err(e) => {
                eprintln!("recap failed: {e}");
                self.recap = None;
                Task::none()
            }
        }
    }

    /// Reset all server-specific view state when switching accounts.
    fn reset_library(&mut self) {
        self.albums.clear();
        self.albums_scroll = 0.0;
        self.home_recent.clear();
        self.home_newest.clear();
        self.home_random.clear();
        self.album_detail = None;
        self.album_detail_id = None;
        self.artists.clear();
        self.artist_detail = None;
        self.artist_detail_id = None;
        self.artist_info = None;
        self.similar_artists.clear();
        self.playlists.clear();
        self.playlist_detail = None;
        self.playlist_detail_id = None;
        self.cover_cache.clear();
        self.search_results = None;
        self.resume_queue = None;
        self.genres.clear();
        self.genre_songs.clear();
        self.genre_detail_name = None;
        self.local_albums.clear();
        self.local_album = None;
        self.local_album_id = None;
        self.view = View::Home;
        self.nav_stack.clear();
    }

    /// Spawn lazy cover-art loads for the first chunk of albums (virtual list +
    /// scroll-driven loading arrive in Phase 9).
    fn load_covers(&self) -> Task<Message> {
        // Only the first screenful up front; the windowed list loads the rest
        // on scroll (avoids saturating the HTTP client with 100+ requests).
        let ids = self
            .albums
            .iter()
            .take(24)
            .filter_map(|a| a.cover_art_id.clone())
            .collect();
        self.load_cover_ids(ids)
    }

    fn load_cover_ids(&self, ids: Vec<String>) -> Task<Message> {
        let mut tasks = Vec::new();
        for cid in ids {
            if self.cover_cache.contains_key(&cid) {
                continue;
            }
            if let Ok(url) = crate::commands::subsonic::build_cover_url(&self.backend.app_state, &cid, 300) {
                let arg_id = cid.clone();
                tasks.push(Task::perform(
                    crate::commands::cover_cache::get_cover_art(arg_id, url),
                    move |res| Message::CoverLoaded(cid.clone(), res),
                ));
            }
        }
        Task::batch(tasks)
    }

    /// Fetch lyrics for the current track when the Lyrics panel is open and the
    /// track changed since the last fetch.
    fn maybe_fetch_lyrics(&mut self) -> Task<Message> {
        if self.right_panel != Some(Panel::Lyrics) {
            return Task::none();
        }
        let song = if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).cloned()
        } else {
            None
        };
        let Some(song) = song else {
            self.lyrics = None;
            self.lyrics_track_id = None;
            return Task::none();
        };
        if self.lyrics_track_id.as_deref() == Some(song.id.as_str()) {
            return Task::none();
        }
        self.lyrics = None;
        self.lyrics_track_id = Some(song.id.clone());
        let id = song.id.clone();
        Task::perform(
            crate::commands::subsonic::get_song_lyrics(
                self.backend.app_state.clone(),
                song.id,
                song.artist,
                song.title,
                song.duration,
                self.lrclib_enabled,
            ),
            move |res| Message::LyricsLoaded(id.clone(), res),
        )
    }

    /// Fetch similar tracks for the current song when the Similar panel is open
    /// and the track changed. Uses the universal fallback (genre + similar artists).
    fn maybe_fetch_similar(&mut self) -> Task<Message> {
        if self.right_panel != Some(Panel::Similar) {
            return Task::none();
        }
        let song = if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).cloned()
        } else {
            None
        };
        let Some(song) = song else {
            self.similar_results.clear();
            self.similar_track_id = None;
            return Task::none();
        };
        if self.similar_track_id.as_deref() == Some(song.id.as_str()) {
            return Task::none();
        }
        self.similar_results.clear();
        self.similar_track_id = Some(song.id.clone());
        let genre = song
            .genres
            .as_ref()
            .and_then(|g| g.as_array())
            .and_then(|a| a.first())
            .and_then(|g| g.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let id = song.id.clone();
        Task::perform(
            crate::commands::subsonic::get_similar_tracks_fallback(
                self.backend.app_state.clone(),
                song.id,
                song.artist_id,
                genre,
                Some(20),
            ),
            move |res| Message::SimilarLoaded(id.clone(), res),
        )
    }

    fn handle_backend(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::PlaybackStateChanged { player_id, state, .. } => {
                if matches!(state, PlaybackState::Loading | PlaybackState::Playing) {
                    self.current_player_id = Some(player_id.clone());
                }
                if self.current_player_id.as_deref() == Some(player_id.as_str()) {
                    self.playback_state = state;
                }
            }
            BackendEvent::PlaybackPosition { player_id, position, duration } => {
                if self.current_player_id.as_deref() == Some(player_id.as_str()) {
                    self.position = position;
                    self.duration = duration;
                }
            }
            BackendEvent::PlaybackFinished { .. } => {
                self.position = 0.0;
            }
            BackendEvent::QueueStateChanged(snapshot) => {
                self.queue = snapshot.queue;
                self.queue_idx = snapshot.queue_idx;
                self.repeat_one = snapshot.repeat_one;
                self.repeat_all = snapshot.repeat_all;
                self.shuffle = snapshot.shuffle_enabled;
                self.volume = snapshot.volume;
                self.crossfade_enabled = snapshot.crossfade_enabled;
                self.crossfade_duration = snapshot.crossfade_duration;
                self.gapless_enabled = snapshot.gapless_enabled;
                self.replay_gain_enabled = snapshot.replay_gain_enabled;
                self.current_player_id = snapshot.player_id;
            }
            BackendEvent::QueueExhausted(_song) => {}
            BackendEvent::SessionExpired => {
                self.authed = false;
            }
        }
    }

    // ── View ──────────────────────────────────────────────────────────────────

    pub fn view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let content = if self.authed { self.shell() } else { self.setup_view() };
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(fill_bg(t.bg))
            .into()
    }

    fn setup_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let mut form = column![
            icons::logo(48.0),
            text("FIRMIUM").size(20).style(tstyle(t.accent)),
            text("Connect to your OpenSubsonic server").size(13).style(tstyle(t.muted)),
            text("SERVER URL").size(11).style(tstyle(t.muted)),
            text_input("https://music.example.com", &self.server_input)
                .on_input(Message::ServerInput)
                .padding(10)
                .width(Length::Fixed(320.0)),
            text("USERNAME").size(11).style(tstyle(t.muted)),
            text_input("username", &self.username_input)
                .on_input(Message::UsernameInput)
                .padding(10)
                .width(Length::Fixed(320.0)),
            text("PASSWORD").size(11).style(tstyle(t.muted)),
            text_input("password", &self.password_input)
                .on_input(Message::PasswordInput)
                .secure(true)
                .padding(10)
                .width(Length::Fixed(320.0)),
            button(text(if self.connecting { "Connecting…" } else { "Connect" }).size(13))
                .on_press(Message::Connect)
                .padding(12)
                .width(Length::Fixed(320.0))
                .style(primary_button(t)),
        ]
        .spacing(14)
        .align_x(Alignment::Center);

        if let Some(err) = &self.connect_error {
            form = form.push(text(err.clone()).size(12).style(tstyle(t.error)));
        }

        let card = container(form)
            .width(Length::Fixed(400.0))
            .padding(40)
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface)),
                border: Border {
                    color: t.border,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..container::Style::default()
            });

        container(card)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn shell(&self) -> Element<'_, Message> {
        let t = self.tokens;

        let host = {
            let conn = self.backend.app_state.connection.read();
            conn.server.clone().unwrap_or_default()
        };
        let host = host
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();

        let brand = container(
            row![
                icons::logo(20.0),
                text(host).size(11).style(tstyle(t.muted)).width(Length::Fill),
                icon_button(icons::USER, 16.0, t.muted, t, Message::ToggleAccountSwitcher),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(20);

        let nav = column![
            self.nav_button(icons::HOME, "Home", View::Home),
            self.nav_button(icons::DISC, "Albums", View::Albums),
            self.nav_button(icons::USER, "Artists", View::Artists),
            self.nav_button(icons::LIST, "Playlists", View::Playlists),
            self.nav_button(icons::SEARCH, "Search", View::Search),
            self.nav_button(icons::MUSIC, "Mix", View::Mix),
            self.nav_button(icons::CLOUD, "Offline", View::Local),
            self.nav_button(icons::BAR_CHART, "Recap", View::Recap),
            self.nav_button(icons::SETTINGS, "Settings", View::Settings),
        ]
        .spacing(4)
        .padding(8);

        let sidebar = container(column![brand, nav])
            .width(Length::Fixed(220.0))
            .height(Length::Fill)
            .style(fill_bg(t.surface));

        let sep = container(text(""))
            .width(Length::Fixed(1.0))
            .height(Length::Fill)
            .style(fill_bg(t.border));

        let main = container(self.content_view())
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .style(fill_bg(t.bg));

        let body = match self.right_panel {
            Some(Panel::Visualizer) => row![sidebar, sep, main, self.viz_panel()],
            Some(Panel::Queue) => row![sidebar, sep, main, self.queue_panel()],
            Some(Panel::Lyrics) => row![sidebar, sep, main, self.lyrics_panel()],
            Some(Panel::Equalizer) => row![sidebar, sep, main, self.eq_panel()],
            Some(Panel::AudioStats) => row![sidebar, sep, main, self.audio_stats_panel()],
            Some(Panel::Similar) => row![sidebar, sep, main, self.similar_panel()],
            None => row![sidebar, sep, main],
        }
        .height(Length::Fill);
        let base = match &self.resume_queue {
            Some(q) => column![self.resume_banner(q), body, self.player_bar()],
            None => column![body, self.player_bar()],
        };
        if self.show_account_switcher {
            stack![base, self.account_switcher_overlay()].into()
        } else if self.add_to_playlist_song.is_some() {
            stack![base, self.add_to_playlist_overlay()].into()
        } else {
            base.into()
        }
    }

    /// Modal listing saved accounts; tap to switch servers, or add a new one.
    fn account_switcher_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let (cur_server, cur_user) = {
            let conn = self.backend.app_state.connection.read();
            (conn.server.clone(), conn.username.clone())
        };

        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::ToggleAccountSwitcher)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let header = row![
            text("Accounts").size(16).style(tstyle(t.text)).width(Length::Fill),
            icon_button(icons::CLOSE, 16.0, t.muted, t, Message::ToggleAccountSwitcher),
        ]
        .align_y(Alignment::Center);

        let mut list = column![].spacing(2);
        if self.accounts.is_empty() {
            list = list.push(text("No saved accounts").size(12).style(tstyle(t.muted)));
        }
        for acct in &self.accounts {
            let is_current = cur_server.as_deref() == Some(acct.server.as_str())
                && cur_user.as_deref() == Some(acct.username.as_str());
            let name_color = if is_current { t.accent } else { t.text };
            let trailing: Element<'_, Message> = if is_current {
                text("Active").size(11).style(tstyle(t.accent)).into()
            } else {
                icons::icon(icons::CHEVRON_RIGHT, 14.0, t.muted)
            };
            list = list.push(
                button(
                    row![
                        icons::icon(icons::USER, 16.0, name_color),
                        column![
                            text(acct.username.clone()).size(13).style(tstyle(name_color)),
                            text(acct.server.clone()).size(11).style(tstyle(t.muted)),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        trailing,
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(8)
                .on_press(Message::SwitchAccount(acct.clone()))
                .style(list_row_style(t)),
            );
        }

        let add = button(
            row![
                icons::icon(icons::PLUS, 16.0, t.text),
                text("Add account").size(13).style(tstyle(t.text)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::AddAccount)
        .style(list_row_style(t));

        let card = container(
            column![
                header,
                scrollable(list).height(Length::Fixed(240.0)),
                add,
            ]
            .spacing(14),
        )
        .width(Length::Fixed(400.0))
        .padding(20)
        .style(move |_th| container::Style {
            background: Some(Background::Color(t.surface)),
            border: Border { radius: 10.0.into(), width: 1.0, color: t.border },
            ..container::Style::default()
        });

        stack![
            backdrop,
            container(card).center_x(Length::Fill).center_y(Length::Fill),
        ]
        .into()
    }

    /// Top banner offering to resume the last cross-device play queue.
    fn resume_banner(&self, q: &RemotePlayQueue) -> Element<'_, Message> {
        let t = self.tokens;
        let track = q
            .current
            .as_deref()
            .and_then(|cur| q.entries.iter().find(|s| s.id == cur))
            .or_else(|| q.entries.first());
        let label = match track {
            Some(s) => format!("Resume “{}” — {}", s.title, s.artist),
            None => "Resume your last queue".to_string(),
        };

        let resume = button(text("Resume").size(12).style(tstyle(t.bg)))
            .padding([6, 14])
            .on_press(Message::ResumeQueue)
            .style(primary_button(t));
        let dismiss = button(text("Dismiss").size(12).style(tstyle(t.muted)))
            .padding([6, 12])
            .on_press(Message::DismissResume)
            .style(move |_th, status| {
                let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if h { Some(Background::Color(t.surface2)) } else { None },
                    text_color: t.muted,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            });

        container(
            row![
                icons::icon(icons::QUEUE, 16.0, t.accent),
                text(label).size(13).style(tstyle(t.text)).width(Length::Fill),
                resume,
                dismiss,
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([10, 16])
        .style(move |_th| container::Style {
            background: Some(Background::Color(t.surface)),
            border: Border { width: 1.0, color: t.accent, radius: 0.0.into() },
            ..container::Style::default()
        })
        .into()
    }

    /// Modal overlay for adding the active track to an existing or new playlist.
    fn add_to_playlist_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let Some(song) = &self.add_to_playlist_song else {
            return container(text("")).into();
        };

        // Click-catching dim backdrop; taps outside the card dismiss the modal.
        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::CloseAddToPlaylist)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let close = icon_button(icons::CLOSE, 16.0, t.muted, t, Message::CloseAddToPlaylist);
        let header = row![
            text("Add to Playlist").size(16).style(tstyle(t.text)).width(Length::Fill),
            close,
        ]
        .align_y(Alignment::Center);

        let subtitle = text(song.title.clone()).size(12).style(tstyle(t.muted));

        let create_row = row![
            text_input("New playlist name…", &self.new_playlist_name)
                .on_input(Message::NewPlaylistNameInput)
                .on_submit(Message::CreatePlaylistAndAdd)
                .padding(8)
                .size(13)
                .width(Length::Fill),
            button(icons::icon(icons::PLUS, 16.0, t.bg))
                .padding(8)
                .on_press(Message::CreatePlaylistAndAdd)
                .style(primary_button(t)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut list = column![].spacing(2);
        if self.playlists.is_empty() {
            list = list.push(text("No playlists yet").size(12).style(tstyle(t.muted)));
        } else {
            for v in &self.playlists {
                let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("Untitled").to_string();
                let count = v.get("songCount").and_then(|x| x.as_u64()).unwrap_or(0);
                list = list.push(
                    button(
                        row![
                            icons::icon(icons::LIST, 16.0, t.muted),
                            text(name).size(13).style(tstyle(t.text)).width(Length::Fill),
                            text(format!("{count}")).size(11).style(tstyle(t.muted)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Message::AddToPlaylist(id))
                    .style(list_row_style(t)),
                );
            }
        }

        let card = container(
            column![
                header,
                subtitle,
                create_row,
                text("Your playlists").size(11).style(tstyle(t.muted)),
                scrollable(list).height(Length::Fixed(260.0)),
            ]
            .spacing(14),
        )
        .width(Length::Fixed(420.0))
        .padding(20)
        .style(move |_th| container::Style {
            background: Some(Background::Color(t.surface)),
            border: Border { radius: 10.0.into(), width: 1.0, color: t.border },
            ..container::Style::default()
        });

        stack![
            backdrop,
            container(card).center_x(Length::Fill).center_y(Length::Fill),
        ]
        .into()
    }

    fn viz_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let mut modes = row![].spacing(6);
        for m in [VizMode::Bars, VizMode::Lines, VizMode::Scope] {
            let active = self.visualizer_mode == m;
            modes = modes.push(
                button(text(m.label()).size(11).style(tstyle(if active { t.bg } else { t.text })))
                    .padding(6)
                    .on_press(Message::SetVizMode(m))
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if active {
                                t.accent
                            } else if h {
                                t.surface
                            } else {
                                t.surface2
                            })),
                            text_color: if active { t.bg } else { t.text },
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        let close = button(icons::icon(icons::CLOSE, 14.0, t.muted))
            .padding(6)
            .on_press(Message::TogglePanel(Panel::Visualizer))
            .style(move |_th, status| {
                let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if h { Some(Background::Color(t.surface2)) } else { None },
                    text_color: t.muted,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            });
        let header = row![
            text("VISUALIZER").size(11).style(tstyle(t.muted)).width(Length::Fill),
            modes,
            close,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let canvas = iced::widget::shader(Visualizer::new(
            self.backend.audio_player.visualizer(),
            self.visualizer_mode,
            crate::viz::VizConfig::default(),
        ))
        .width(Length::Fill)
        .height(Length::Fill);

        container(column![header, canvas].spacing(12))
            .width(Length::Fixed(360.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    fn content_view(&self) -> Element<'_, Message> {
        match &self.view {
            View::Home => self.home_view(),
            View::Albums => self.album_list_view(),
            View::AlbumDetail(_) => self.album_detail_view(),
            View::Artists => self.artists_view(),
            View::ArtistDetail(_) => self.artist_detail_view(),
            View::Playlists => self.playlists_view(),
            View::PlaylistDetail(_) => self.playlist_detail_view(),
            View::Search => self.search_view(),
            View::Mix => self.mix_view(),
            View::GenreDetail(_) => self.genre_detail_view(),
            View::Local => self.local_view(),
            View::LocalAlbumDetail(_) => self.local_album_detail_view(),
            View::Recap => self.recap_view(),
            View::Settings => self.settings_view(),
        }
    }

    fn search_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let bar = row![
            text_input("Search your library…", &self.search_query)
                .on_input(Message::SearchInput)
                .on_submit(Message::SubmitSearch)
                .padding(10)
                .width(Length::Fill),
            button(text("Search").size(13))
                .on_press(Message::SubmitSearch)
                .padding(10)
                .style(primary_button(t)),
        ]
        .spacing(10);

        let results: Element<'_, Message> = if let Some(res) = &self.search_results {
            let mut col = column![].spacing(4);
            if !res.albums.is_empty() {
                col = col.push(text("Albums").size(13).style(tstyle(t.muted)));
                for a in res.albums.iter().take(40) {
                    col = col.push(self.album_row(a));
                }
            }
            if !res.songs.is_empty() {
                col = col.push(text("Songs").size(13).style(tstyle(t.muted)));
                for s in res.songs.iter().take(100) {
                    col = col.push(self.song_row(s));
                }
            }
            scrollable(col).height(Length::Fill).into()
        } else {
            text("Type a query and press Enter").size(12).style(tstyle(t.muted)).into()
        };

        column![text("Search").size(22).style(tstyle(t.text)), bar, results]
            .spacing(16)
            .into()
    }

    fn song_row(&self, song: &Song) -> Element<'_, Message> {
        let t = self.tokens;
        let is_current = self.current_song_id() == Some(song.id.as_str());
        let title_color = if is_current { t.accent } else { t.text };
        let play_area = button(
            row![
                self.cover_image(song.cover_art_id.as_deref(), 36.0),
                column![
                    text(song.title.clone()).size(13).style(tstyle(title_color)),
                    text(format!("{} · {}", song.artist, song.album)).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2)
                .width(Length::Fill),
                text(fmt_time(song.duration)).size(11).style(tstyle(t.muted)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::PlaySong(song.clone()))
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if h { Some(Background::Color(t.surface)) } else { None },
                text_color: t.text,
                border: Border { radius: 2.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });
        row![
            play_area,
            icon_button(icons::PLUS, 14.0, t.muted, t, Message::OpenAddToPlaylist(song.clone())),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn recap_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let mut ranges = row![].spacing(6);
        for r in [
            RecapRange::Week,
            RecapRange::Month,
            RecapRange::ThreeMonths,
            RecapRange::Year,
            RecapRange::All,
        ] {
            let active = self.recap_range == r;
            ranges = ranges.push(
                button(text(r.label()).size(11).style(tstyle(if active { t.bg } else { t.text })))
                    .padding([6, 10])
                    .on_press(Message::SetRecapRange(r))
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if active {
                                t.accent
                            } else if h {
                                t.surface
                            } else {
                                t.surface2
                            })),
                            text_color: if active { t.bg } else { t.text },
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        let header = row![
            text("Firmium Recap").size(22).style(tstyle(t.text)).width(Length::Fill),
            ranges,
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match &self.recap {
            Some(stats) if stats.total_plays > 0 => {
                let idx = self.recap_card.min(RECAP_CARDS - 1);
                let card = container(self.recap_card_content(stats, idx))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(24)
                    .style(move |_th| container::Style {
                        background: Some(Background::Color(t.surface)),
                        border: Border { radius: 12.0.into(), width: 1.0, color: t.border },
                        ..container::Style::default()
                    });

                let mut dots = row![].spacing(6);
                for i in 0..RECAP_CARDS {
                    let c = if i == idx { t.accent } else { t.surface2 };
                    dots = dots.push(
                        container(text(""))
                            .width(Length::Fixed(8.0))
                            .height(Length::Fixed(8.0))
                            .style(move |_th| container::Style {
                                background: Some(Background::Color(c)),
                                border: Border { radius: 4.0.into(), ..Border::default() },
                                ..container::Style::default()
                            }),
                    );
                }
                let nav = row![
                    icon_button(icons::BACK, 16.0, t.text, t, Message::RecapPrev),
                    container(dots).center_x(Length::Fill),
                    icon_button(icons::CHEVRON_RIGHT, 16.0, t.text, t, Message::RecapNext),
                ]
                .align_y(Alignment::Center);

                column![card, nav].spacing(16).height(Length::Fill).into()
            }
            _ => container(
                text("No listening history yet — play some music and check back.")
                    .size(13)
                    .style(tstyle(t.muted)),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        };

        column![header, body].spacing(16).height(Length::Fill).into()
    }

    /// Body of a single Recap card, selected by `idx` (0..RECAP_CARDS).
    fn recap_card_content(&self, stats: &crate::db::RecapStats, idx: usize) -> Element<'_, Message> {
        let t = self.tokens;
        let label = |s: &'static str| text(s).size(12).style(tstyle(t.muted));
        match idx {
            0 => column![
                label("OVERVIEW"),
                text(format!("{} plays", stats.total_plays)).size(40).style(tstyle(t.accent)),
                text(format!("{} of listening", fmt_hours(stats.total_seconds))).size(16).style(tstyle(t.text)),
                text(format!("over the last {}", self.recap_range.label().to_lowercase()))
                    .size(12).style(tstyle(t.muted)),
            ]
            .spacing(10)
            .into(),
            1 => {
                let mut col = column![label("TOP TRACKS")].spacing(8);
                for (i, s) in stats.top_tracks.iter().take(5).enumerate() {
                    col = col.push(
                        row![
                            text(format!("{}", i + 1)).size(13).style(tstyle(t.muted)).width(Length::Fixed(20.0)),
                            self.cover_image(s.cover_art_id.as_deref(), 40.0),
                            column![
                                text(s.title.clone()).size(13).style(tstyle(t.text)),
                                text(s.artist.clone().unwrap_or_default()).size(11).style(tstyle(t.muted)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                            text(format!("{}×", s.count)).size(12).style(tstyle(t.accent)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
                col.into()
            }
            2 => {
                let mut col = column![label("TOP ARTISTS")].spacing(8);
                for (i, s) in stats.top_artists.iter().take(5).enumerate() {
                    col = col.push(
                        row![
                            text(format!("{}", i + 1)).size(13).style(tstyle(t.muted)).width(Length::Fixed(20.0)),
                            text(s.name.clone()).size(14).style(tstyle(t.text)).width(Length::Fill),
                            text(format!("{}×", s.count)).size(12).style(tstyle(t.accent)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
                col.into()
            }
            3 => {
                let mut col = column![label("TOP ALBUMS")].spacing(8);
                for (i, s) in stats.top_albums.iter().take(5).enumerate() {
                    col = col.push(
                        row![
                            text(format!("{}", i + 1)).size(13).style(tstyle(t.muted)).width(Length::Fixed(20.0)),
                            self.cover_image(s.cover_art_id.as_deref(), 40.0),
                            column![
                                text(s.name.clone()).size(13).style(tstyle(t.text)),
                                text(s.artist.clone().unwrap_or_default()).size(11).style(tstyle(t.muted)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                            text(format!("{}×", s.count)).size(12).style(tstyle(t.accent)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
                col.into()
            }
            4 => {
                let body: Element<'_, Message> = match &stats.top_genre {
                    Some(g) => column![
                        text(g.genre.clone()).size(32).style(tstyle(t.accent)),
                        text(format!("{} plays", g.count)).size(14).style(tstyle(t.muted)),
                    ]
                    .spacing(8)
                    .into(),
                    None => text("No genre data").size(13).style(tstyle(t.muted)).into(),
                };
                column![label("TOP GENRE"), body].spacing(12).into()
            }
            5 => {
                let tod = &stats.by_time_of_day;
                column![
                    label("TIME OF DAY"),
                    stat_row("Morning (5–11)", format!("{}", tod.morning), t),
                    stat_row("Afternoon (12–16)", format!("{}", tod.afternoon), t),
                    stat_row("Evening (17–20)", format!("{}", tod.evening), t),
                    stat_row("Night (21–4)", format!("{}", tod.night), t),
                ]
                .spacing(10)
                .into()
            }
            6 => {
                const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
                let mut col = column![label("DAY OF WEEK")].spacing(10);
                for (i, d) in DAYS.iter().enumerate() {
                    col = col.push(stat_row(d, format!("{}", stats.by_day_of_week[i]), t));
                }
                col.into()
            }
            7 => {
                let body: Element<'_, Message> = match &stats.biggest_discovery {
                    Some(d) => row![
                        self.cover_image(d.cover_art_id.as_deref(), 56.0),
                        column![
                            text(d.title.clone()).size(16).style(tstyle(t.text)),
                            text(d.artist.clone().unwrap_or_default()).size(12).style(tstyle(t.muted)),
                            text(format!("{} plays since you found it", d.count)).size(11).style(tstyle(t.accent)),
                        ]
                        .spacing(4),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .into(),
                    None => text("No standout discovery this period").size(13).style(tstyle(t.muted)).into(),
                };
                column![label("BIGGEST DISCOVERY"), body].spacing(12).into()
            }
            _ => column![
                label("STREAK"),
                text(format!("{} days active", stats.streak.days_active)).size(24).style(tstyle(t.text)),
                text(format!("Longest streak: {} days", stats.streak.longest_streak)).size(14).style(tstyle(t.accent)),
            ]
            .spacing(10)
            .into(),
        }
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let t = self.tokens;

        // Left rail: category nav.
        let cats = [
            (SettingsCategory::Appearance, icons::PALETTE, "Appearance"),
            (SettingsCategory::Playback, icons::PLAY, "Playback"),
            (SettingsCategory::Equalizer, icons::EQUALIZER, "Equalizer"),
            (SettingsCategory::Downloads, icons::DOWNLOAD, "Downloads"),
            (SettingsCategory::Services, icons::GLOBE, "Services"),
            (SettingsCategory::Account, icons::USER, "Account"),
            (SettingsCategory::Debug, icons::INFO, "Debug"),
        ];
        let mut nav = column![text("SETTINGS").size(11).style(tstyle(t.muted))]
            .spacing(2)
            .padding([4, 8]);
        for (cat, icon_src, label_str) in cats {
            let active = self.settings_category == cat;
            nav = nav.push(
                button(
                    row![
                        icons::icon(icon_src, 16.0, if active { t.accent } else { t.muted }),
                        text(label_str).size(13).style(tstyle(if active { t.accent } else { t.text })),
                    ]
                    .spacing(9)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([7, 10])
                .on_press(Message::SetSettingsCategory(cat))
                .style(move |_theme, status| {
                    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if active {
                            t.accent_dim
                        } else if hovered {
                            t.surface
                        } else {
                            Color::TRANSPARENT
                        })),
                        text_color: if active { t.accent } else { t.text },
                        border: Border { radius: 6.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                }),
            );
        }
        let sidebar = container(nav)
            .width(Length::Fixed(180.0))
            .height(Length::Fill)
            .style(fill_bg(t.surface));

        let sep = container(text(""))
            .width(Length::Fixed(1.0))
            .height(Length::Fill)
            .style(fill_bg(t.border));

        let panel = scrollable(match self.settings_category {
            SettingsCategory::Appearance => self.settings_appearance(t),
            SettingsCategory::Playback => self.settings_playback(t),
            SettingsCategory::Equalizer => self.settings_equalizer(t),
            SettingsCategory::Downloads => self.settings_downloads(t),
            SettingsCategory::Services => self.settings_services(t),
            SettingsCategory::Account => self.settings_account(t),
            SettingsCategory::Debug => self.settings_debug(t),
        })
        .height(Length::Fill);

        row![sidebar, sep, container(panel).padding([0, 4]).width(Length::Fill)]
            .height(Length::Fill)
            .into()
    }

    fn settings_appearance(&self, t: Tokens) -> Element<'_, Message> {
        let mut theme_grid = column![].spacing(8);
        for chunk in self.themes.chunks(4) {
            let mut r = row![].spacing(8);
            for entry in chunk {
                r = r.push(self.theme_swatch(entry));
            }
            theme_grid = theme_grid.push(r);
        }
        column![
            sett_panel_title("Appearance", t),
            sett_row("Theme", "Color scheme for the interface", t, theme_grid.into()),
            sett_row(
                "Window Decorations",
                "Show the native title bar and window borders",
                t,
                toggler(self.window_decorations).on_toggle(Message::SetDecorations).into(),
            ),
        ]
        .spacing(0)
        .into()
    }

    fn settings_playback(&self, t: Tokens) -> Element<'_, Message> {
        let bp = |label: &'static str, mode: &'static str| -> Element<'_, Message> {
            let active = self.bit_perfect_mode == mode;
            button(text(label).size(12).style(tstyle(if active { t.bg } else { t.text })))
                .padding(8)
                .on_press(Message::SetBitPerfect(mode.to_string()))
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if active {
                            t.accent
                        } else if h {
                            t.surface
                        } else {
                            t.surface2
                        })),
                        text_color: if active { t.bg } else { t.text },
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        let crossfade_dur: Element<'_, Message> = if self.crossfade_enabled {
            sett_row(
                "Crossfade Duration",
                "Length of the blend in seconds",
                t,
                row![
                    slider(1.0..=12.0, self.crossfade_duration, Message::SetCrossfadeDuration)
                        .step(1.0)
                        .width(Length::Fixed(100.0)),
                    text(format!("{:.0}s", self.crossfade_duration)).size(12).style(tstyle(t.muted)),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            )
        } else {
            column![].into()
        };
        column![
            sett_panel_title("Playback", t),
            sett_row("Crossfade", "Smoothly blend between tracks", t,
                toggler(self.crossfade_enabled).on_toggle(Message::SetCrossfadeEnabled).into()),
            crossfade_dur,
            sett_row("Gapless Playback", "Pre-buffer the next track for seamless transitions", t,
                toggler(self.gapless_enabled).on_toggle(Message::SetGapless).into()),
            sett_row("ReplayGain", "Normalize track loudness using server-provided gain values", t,
                toggler(self.replay_gain_enabled).on_toggle(Message::SetReplayGain).into()),
            sett_row("Auto-continue (Smart Radio)", "Adds similar tracks when the queue runs out", t,
                toggler(self.auto_continue).on_toggle(Message::SetAutoContinue).into()),
            sett_row("Bit-Perfect Audio", "Match each track's native sample rate", t,
                row![bp("Off", "off"), bp("Relaxed", "relaxed"), bp("Strict", "strict")].spacing(4).into()),
        ]
        .spacing(0)
        .into()
    }

    fn settings_equalizer(&self, t: Tokens) -> Element<'_, Message> {
        column![
            sett_panel_title("Equalizer", t),
            sett_row(
                "Graphic Equalizer",
                "Open the multi-band EQ in the side panel",
                t,
                button(text("Open Equalizer").size(13).style(tstyle(t.text)))
                    .padding(10)
                    .on_press(Message::TogglePanel(Panel::Equalizer))
                    .style(move |_t, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                            text_color: t.text,
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    })
                    .into(),
            ),
        ]
        .spacing(0)
        .into()
    }

    fn settings_downloads(&self, t: Tokens) -> Element<'_, Message> {
        let formats = ["raw", "mp3", "flac", "wav", "opus"];
        let format_labels = ["Original", "MP3", "FLAC", "WAV", "Opus"];
        let mut fmt_btns = row![].spacing(4);
        for (id, label) in formats.iter().zip(format_labels.iter()) {
            let active = self.download_format == *id;
            let id_owned = id.to_string();
            fmt_btns = fmt_btns.push(
                button(text(*label).size(12).style(tstyle(if active { t.bg } else { t.text })))
                    .padding(8)
                    .on_press(Message::SetDownloadFormat(id_owned))
                    .style(move |_t, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if active {
                                t.accent
                            } else if h {
                                t.surface
                            } else {
                                t.surface2
                            })),
                            text_color: if active { t.bg } else { t.text },
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        column![
            sett_panel_title("Downloads", t),
            sett_row(
                "Download Format",
                "Format used when downloading. \"Original\" saves exactly as stored on the server.",
                t,
                fmt_btns.into(),
            ),
        ]
        .spacing(0)
        .into()
    }

    fn settings_services(&self, t: Tokens) -> Element<'_, Message> {
        let mut col = column![sett_panel_title("Services", t)].spacing(0);
        col = col.push(sett_row(
            "Last.fm Integration",
            "Fetch richer artist bio and photo using your own Last.fm API key",
            t,
            toggler(self.lastfm_enabled).on_toggle(Message::SetLastfmEnabled).into(),
        ));
        if self.lastfm_enabled {
            col = col.push(sett_row(
                "Last.fm API Key",
                "From your Last.fm API account",
                t,
                text_input("API key…", &self.lastfm_key)
                    .on_input(Message::SetLastfmKey)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .into(),
            ));
            col = col.push(sett_row(
                "Last.fm Secret",
                "Shared secret for your API account",
                t,
                text_input("Secret…", &self.lastfm_secret)
                    .on_input(Message::SetLastfmSecret)
                    .secure(true)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "ListenBrainz Scrobbling",
            "Submit each completed track to ListenBrainz using your user token",
            t,
            toggler(self.listenbrainz_enabled).on_toggle(Message::SetListenbrainzEnabled).into(),
        ));
        if self.listenbrainz_enabled {
            col = col.push(sett_row(
                "ListenBrainz Token",
                "From your ListenBrainz profile settings",
                t,
                text_input("User token…", &self.listenbrainz_token)
                    .on_input(Message::SetListenbrainzToken)
                    .secure(true)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "External Lyrics (LRCLIB)",
            "Fetch synced lyrics from lrclib.net when your server has none",
            t,
            toggler(self.lrclib_enabled).on_toggle(Message::SetLrclibEnabled).into(),
        ));
        col = col.push(sett_row(
            "Word-by-Word Lyrics Animation",
            "Karaoke-style fill across the active synced lyric line",
            t,
            toggler(self.lyrics_word_fill).on_toggle(Message::SetLyricsWordFill).into(),
        ));
        col.into()
    }

    fn settings_account(&self, t: Tokens) -> Element<'_, Message> {
        let (server, username) = {
            let conn = self.backend.app_state.connection.read();
            (conn.server.clone().unwrap_or_default(), conn.username.clone().unwrap_or_default())
        };
        let conn_desc = if self.authed {
            format!("{username} @ {server}")
        } else {
            "Not connected".to_string()
        };
        let conn_btn: Element<'_, Message> = if self.authed {
            button(text("Log out").size(13).style(tstyle(t.error)))
                .padding(10)
                .on_press(Message::Logout)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: if h { Some(Background::Color(t.surface)) } else { None },
                        text_color: t.error,
                        border: Border { color: t.error, width: 1.0, radius: 4.0.into() },
                        ..button::Style::default()
                    }
                })
                .into()
        } else {
            button(text("Connect").size(13).style(tstyle(t.text)))
                .padding(10)
                .on_press(Message::ToggleAccountSwitcher)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };

        let sec_btn = |label: &'static str, msg: Message| -> Element<'_, Message> {
            button(text(label).size(13).style(tstyle(t.text)))
                .padding(10)
                .on_press(msg)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        let stats_section: Element<'_, Message> = match &self.history_summary {
            Some(s) if s.total_plays > 0 => column![
                stat_row("Total plays", s.total_plays.to_string(), t),
                stat_row("Listening time", fmt_hours(s.total_seconds), t),
                stat_row("Unique tracks", s.unique_tracks.to_string(), t),
                stat_row("Unique artists", s.unique_artists.to_string(), t),
                stat_row("Unique albums", s.unique_albums.to_string(), t),
                row![
                    sec_btn("Export CSV", Message::ExportStats("csv".to_string())),
                    sec_btn("Export JSON", Message::ExportStats("json".to_string())),
                    sec_btn("View Recap", Message::Navigate(View::Recap)),
                ]
                .spacing(8),
            ]
            .spacing(10)
            .into(),
            _ => text("No play history yet — start listening to build your stats.")
                .size(12)
                .style(tstyle(t.muted))
                .into(),
        };

        column![
            sett_panel_title("Account", t),
            sett_row("Connection", conn_desc, t, conn_btn),
            sett_panel_title("Listening Stats", t),
            container(stats_section).padding([15, 10]),
        ]
        .spacing(0)
        .into()
    }

    fn settings_debug(&self, t: Tokens) -> Element<'_, Message> {
        let version = crate::commands::app_info::get_app_version();
        let debug_btn = |label: &'static str, msg: Message, danger: bool| -> Element<'_, Message> {
            button(text(label).size(13).style(tstyle(if danger { t.error } else { t.text })))
                .padding(10)
                .on_press(msg)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: if danger { t.error } else { t.text },
                        border: Border {
                            color: if danger { t.error } else { Color::TRANSPARENT },
                            width: if danger { 1.0 } else { 0.0 },
                            radius: 4.0.into(),
                        },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        column![
            sett_panel_title("Debug", t),
            sett_row("App Version", version, t, text("").into()),
            sett_row("Wipe Cache", "Clear cached cover art", t,
                debug_btn("Wipe", Message::WipeCoverCache, false)),
            sett_row("Delete Settings", "Reset all preferences to defaults", t,
                debug_btn("Delete", Message::DeleteSettings, true)),
        ]
        .spacing(0)
        .into()
    }

    fn theme_swatch(&self, entry: &ThemeEntry) -> Element<'_, Message> {
        let t = self.tokens;
        let active = entry.id == self.theme_id;
        let acc = Tokens::from_entry(entry).accent;
        let swatch = container(text(""))
            .width(Length::Fixed(90.0))
            .height(Length::Fixed(26.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(acc)),
                border: Border {
                    color: if active { t.text } else { t.border },
                    width: if active { 2.0 } else { 1.0 },
                    radius: 4.0.into(),
                },
                ..container::Style::default()
            });
        button(
            column![swatch, text(entry.name.clone()).size(10).style(tstyle(if active { t.accent } else { t.muted }))]
                .spacing(4)
                .align_x(Alignment::Center),
        )
        .padding(4)
        .on_press(Message::SelectTheme(entry.id.clone()))
        .style(|_t, _status| button::Style {
            background: None,
            ..button::Style::default()
        })
        .into()
    }

    fn current_song_id(&self) -> Option<&str> {
        if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).map(|s| s.id.as_str())
        } else {
            None
        }
    }

    fn album_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let Some(at) = &self.album_detail else {
            return text("Loading…").size(13).style(tstyle(t.muted)).into();
        };

        let back = button(
            row![
                icons::icon(icons::BACK, 14.0, t.muted),
                text("Back").size(12).style(tstyle(t.muted)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(6)
        .on_press(Message::NavigateBack)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if h { Some(Background::Color(t.surface)) } else { None },
                text_color: t.muted,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        let play_btn = button(
            row![
                icons::icon(icons::PLAY, 14.0, t.bg),
                text("Play").size(12).style(tstyle(t.bg)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::PlayAlbumAt(0))
        .style(primary_button(t));

        let shuffle_btn = button(
            row![
                icons::icon(icons::SHUFFLE, 14.0, t.text),
                text("Shuffle").size(12).style(tstyle(t.text)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::ShuffleAlbum)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        let album_download_btn = button(
            row![
                icons::icon(icons::DOWNLOAD, 14.0, t.text),
                text("Download").size(12).style(tstyle(t.text)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::DownloadAlbum)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        let header = row![
            self.cover_image(at.cover_art_id.as_deref(), 80.0),
            column![
                text(at.album_name.clone()).size(18).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::MONOSPACE
                }),
                text(at.album_artist.clone()).size(13).style(tstyle(t.muted)),
                text(format!("{} tracks", at.tracks.len())).size(11).style(tstyle(t.muted)),
                row![play_btn, shuffle_btn, album_download_btn].spacing(8),
            ]
            .spacing(10),
        ]
        .spacing(20);

        let mut list = column![].spacing(2);
        for (i, track) in at.tracks.iter().enumerate() {
            list = list.push(self.track_row(i, track, Message::PlayAlbumAt(i)));
        }

        column![back, header, scrollable(list).height(Length::Fill)]
            .spacing(16)
            .into()
    }

    fn track_row(&self, idx: usize, song: &Song, on_press: Message) -> Element<'_, Message> {
        let t = self.tokens;
        let is_current = self.current_song_id() == Some(song.id.as_str());
        let title_color = if is_current { t.accent } else { t.text };
        let num = song
            .track_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| (idx + 1).to_string());

        let play_area = button(
            row![
                text(num).size(11).style(tstyle(t.muted)).width(Length::Fixed(24.0)),
                self.cover_image(song.cover_art_id.as_deref(), 36.0),
                column![
                    text(song.title.clone()).size(13).style(tstyle(title_color)),
                    text(song.artist.clone()).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(on_press)
        .style(move |_t, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if is_current {
                    Some(Background::Color(t.accent_dim))
                } else if h {
                    Some(Background::Color(t.surface))
                } else {
                    None
                },
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        });

        row![
            play_area,
            self.star_rating(song),
            icon_button(icons::PLUS, 14.0, t.muted, t, Message::OpenAddToPlaylist(song.clone())),
            icon_button(icons::DOWNLOAD, 14.0, t.muted, t, Message::DownloadTrack(song.clone())),
            text(fmt_time(song.duration)).size(11).style(tstyle(t.muted)).width(Length::Fixed(44.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn star_rating(&self, song: &Song) -> Element<'_, Message> {
        let t = self.tokens;
        let rating = song.user_rating.unwrap_or(0);
        let id = song.id.clone();
        let mut stars = row![].spacing(1);
        for i in 1..=5u32 {
            let filled = i <= rating;
            let src = if filled { icons::STAR_FILLED } else { icons::STAR_EMPTY };
            let color = if filled { t.accent } else { t.muted };
            let sid = id.clone();
            stars = stars.push(
                button(icons::icon(src, 12.0, color))
                    .padding(1)
                    .on_press(Message::SetRating(sid, i))
                    .style(|_th, _status| button::Style {
                        background: None,
                        ..button::Style::default()
                    }),
            );
        }
        stars.into()
    }

    fn album_list_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = text(format!("Albums ({})", self.albums.len())).size(22).style(tstyle(t.text));

        // Windowed (virtual) rendering: only the visible rows are built; the
        // scrolled-past and remaining heights are filled with spacers so the
        // scrollbar stays correct for libraries with thousands of albums.
        let total = self.albums.len();
        let first = ((self.albums_scroll / ALBUM_ROW_H).floor().max(0.0) as usize).min(total);
        let count = (VIEWPORT_H / ALBUM_ROW_H).ceil() as usize + 4;
        let end = (first + count).min(total);

        let mut list = column![];
        if first > 0 {
            list = list.push(container(text("")).height(Length::Fixed(first as f32 * ALBUM_ROW_H)));
        }
        for album in &self.albums[first..end] {
            list = list.push(self.album_row(album));
        }
        if end < total {
            list = list.push(container(text("")).height(Length::Fixed((total - end) as f32 * ALBUM_ROW_H)));
        }

        let scroller = scrollable(list)
            .height(Length::Fill)
            .on_scroll(|v| Message::AlbumsScrolled(v.absolute_offset().y));

        column![header, scroller].spacing(16).into()
    }

    fn album_row(&self, album: &Album) -> Element<'_, Message> {
        let t = self.tokens;
        let cover = self.cover_image(album.cover_art_id.as_deref(), 44.0);
        let info = column![
            text(album.name.clone()).size(13).style(tstyle(t.text)),
            text(album.album_artist.clone()).size(11).style(tstyle(t.muted)),
        ]
        .spacing(2);

        button(row![cover, info].spacing(12).align_y(Alignment::Center))
            .width(Length::Fill)
            .padding(8)
            .on_press(Message::Navigate(View::AlbumDetail(album.id.clone())))
            .style(move |_theme, status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if hovered { Some(Background::Color(t.surface)) } else { None },
                    text_color: t.text,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            })
            .into()
    }

    fn cover_image(&self, cover_id: Option<&str>, size: f32) -> Element<'_, Message> {
        let t = self.tokens;
        let radius = if size >= 80.0 { 12.0_f32 } else if size >= 40.0 { 8.0 } else { 6.0 };
        if let Some(id) = cover_id {
            if let Some(handle) = self.cover_cache.get(id) {
                return container(
                    iced::widget::image(handle.clone())
                        .width(Length::Fixed(size))
                        .height(Length::Fixed(size))
                        .content_fit(ContentFit::Cover),
                )
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .clip(true)
                .style(move |_| container::Style {
                    border: Border { radius: radius.into(), ..Border::default() },
                    ..container::Style::default()
                })
                .into();
            }
        }
        container(icons::icon(icons::DISC, size * 0.5, t.muted))
            .center_x(Length::Fixed(size))
            .center_y(Length::Fixed(size))
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface2)),
                border: Border { radius: radius.into(), ..Border::default() },
                ..container::Style::default()
            })
            .into()
    }

    fn artists_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = text(format!("Artists ({})", self.artists.len())).size(22).style(tstyle(t.text));
        let mut list = column![].spacing(2);
        for artist in self.artists.iter().take(300) {
            list = list.push(self.artist_row(artist));
        }
        column![header, scrollable(list).height(Length::Fill)].spacing(16).into()
    }

    fn artist_row(&self, artist: &Artist) -> Element<'_, Message> {
        let t = self.tokens;
        let avatar = container(icons::icon(icons::USER, 22.0, t.muted))
            .center_x(Length::Fixed(44.0))
            .center_y(Length::Fixed(44.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface2)),
                border: Border { radius: 22.0.into(), ..Border::default() },
                ..container::Style::default()
            });
        button(
            row![
                avatar,
                column![
                    text(artist.name.clone()).size(13).style(tstyle(t.text)),
                    text(format!("{} albums", artist.album_count)).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::Navigate(View::ArtistDetail(artist.id.clone())))
        .style(list_row_style(t))
        .into()
    }

    fn artist_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let Some(d) = &self.artist_detail else {
            return text("Loading…").size(13).style(tstyle(t.muted)).into();
        };
        let mut list = column![].spacing(2);
        for album in &d.albums {
            list = list.push(self.album_row(album));
        }

        let mut head = column![
            back_button(t),
            text(d.name.clone()).size(26).style(tstyle(t.accent)).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            }),
            text(format!("{} albums", d.albums.len())).size(12).style(tstyle(t.muted)),
        ]
        .spacing(12);

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

        column![head, scrollable(list).height(Length::Fill)]
            .spacing(12)
            .into()
    }

    fn genre_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let name = self.genre_detail_name.clone().unwrap_or_default();
        if self.genre_songs.is_empty() {
            return column![back_button(t), text("Loading…").size(13).style(tstyle(t.muted))]
                .spacing(12)
                .into();
        }
        let play = button(
            row![
                icons::icon(icons::PLAY, 14.0, t.bg),
                text("Play all").size(12).style(tstyle(t.bg)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::PlayGenreAt(0))
        .style(primary_button(t));

        let mut list = column![].spacing(2);
        for (i, song) in self.genre_songs.iter().enumerate() {
            list = list.push(self.track_row(i, song, Message::PlayGenreAt(i)));
        }

        column![
            back_button(t),
            text(name).size(24).style(tstyle(t.text)),
            text(format!("{} songs", self.genre_songs.len())).size(11).style(tstyle(t.muted)),
            play,
            scrollable(list).height(Length::Fill),
        ]
        .spacing(12)
        .into()
    }

    fn local_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = text(format!("Offline Library ({})", self.local_albums.len()))
            .size(22)
            .style(tstyle(t.text));
        if self.local_albums.is_empty() {
            return column![
                header,
                text("No downloaded music yet. Use the download button on a track or album to save it here for offline playback.")
                    .size(12)
                    .style(tstyle(t.muted)),
            ]
            .spacing(16)
            .into();
        }
        let mut list = column![].spacing(2);
        for album in &self.local_albums {
            let id = album.id.clone();
            let cover = self.cover_image(album.cover_art_id.as_deref(), 44.0);
            let info = column![
                text(album.name.clone()).size(13).style(tstyle(t.text)),
                text(album.album_artist.clone()).size(11).style(tstyle(t.muted)),
            ]
            .spacing(2);
            list = list.push(
                button(row![cover, info].spacing(12).align_y(Alignment::Center))
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Message::Navigate(View::LocalAlbumDetail(id)))
                    .style(list_row_style(t)),
            );
        }
        column![header, scrollable(list).height(Length::Fill)].spacing(16).into()
    }

    fn local_album_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let Some(la) = &self.local_album else {
            return column![back_button(t), text("Loading…").size(13).style(tstyle(t.muted))]
                .spacing(12)
                .into();
        };
        let play = button(
            row![
                icons::icon(icons::PLAY, 14.0, t.bg),
                text("Play").size(12).style(tstyle(t.bg)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::PlayLocalAlbumAt(0))
        .style(primary_button(t));

        let header = row![
            self.cover_image(la.cover_art_id.as_deref(), 120.0),
            column![
                text(la.album_name.clone()).size(24).style(tstyle(t.text)),
                text(la.album_artist.clone()).size(14).style(tstyle(t.muted)),
                text(format!("{} tracks · offline", la.tracks.len())).size(11).style(tstyle(t.muted)),
                play,
            ]
            .spacing(10),
        ]
        .spacing(20);

        let mut list = column![].spacing(2);
        for (i, track) in la.tracks.iter().enumerate() {
            list = list.push(self.track_row(i, track, Message::PlayLocalAlbumAt(i)));
        }

        column![back_button(t), header, scrollable(list).height(Length::Fill)]
            .spacing(16)
            .into()
    }

    fn playlists_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = text(format!("Playlists ({})", self.playlists.len())).size(22).style(tstyle(t.text));
        let mut list = column![].spacing(2);
        for v in &self.playlists {
            list = list.push(self.playlist_row(v));
        }
        column![header, scrollable(list).height(Length::Fill)].spacing(16).into()
    }

    fn playlist_row(&self, v: &serde_json::Value) -> Element<'_, Message> {
        let t = self.tokens;
        let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("Untitled").to_string();
        let count = v.get("songCount").and_then(|x| x.as_u64()).unwrap_or(0);
        let icon_box = container(icons::icon(icons::LIST, 22.0, t.muted))
            .center_x(Length::Fixed(44.0))
            .center_y(Length::Fixed(44.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface2)),
                border: Border { radius: 6.0.into(), ..Border::default() },
                ..container::Style::default()
            });
        button(
            row![
                icon_box,
                column![
                    text(name).size(13).style(tstyle(t.text)),
                    text(format!("{count} tracks")).size(11).style(tstyle(t.muted)),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(Message::Navigate(View::PlaylistDetail(id)))
        .style(list_row_style(t))
        .into()
    }

    fn playlist_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let Some(pt) = &self.playlist_detail else {
            return text("Loading…").size(13).style(tstyle(t.muted)).into();
        };
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

        let mut list = column![].spacing(2);
        for (i, track) in pt.tracks.iter().enumerate() {
            list = list.push(self.track_row(i, track, Message::PlayPlaylistAt(i)));
        }

        column![
            back_button(t),
            column![
                text(pt.name.clone()).size(24).style(tstyle(t.text)),
                text(format!("{} tracks", pt.tracks.len())).size(11).style(tstyle(t.muted)),
                play,
            ]
            .spacing(8),
            scrollable(list).height(Length::Fill),
        ]
        .spacing(16)
        .into()
    }

    fn nav_button(&self, _icon_src: &'static str, label: &'static str, target: View) -> Element<'_, Message> {
        let active = self.view == target;
        let t = self.tokens;
        let color = if active { t.accent } else { t.muted };
        let mut content = text(label).size(13).style(tstyle(color));
        if active {
            content = content.font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            });
        }

        button(content)
            .width(Length::Fill)
            .padding([10, 12])
            .on_press(Message::Navigate(target))
            .style(move |_theme, status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if active || hovered { Some(Background::Color(t.surface2)) } else { None },
                    text_color: color,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            })
            .into()
    }

    fn queue_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = row![
            text("QUEUE").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, Message::TogglePanel(Panel::Queue)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = if self.queue.is_empty() {
            text("Queue is empty").size(12).style(tstyle(t.muted)).into()
        } else {
            let mut list = column![].spacing(2);
            for (i, song) in self.queue.iter().enumerate() {
                let is_current = i as i32 == self.queue_idx;
                let tc = if is_current { t.accent } else { t.text };
                list = list.push(
                    button(
                        row![
                            self.cover_image(song.cover_art_id.as_deref(), 32.0),
                            column![
                                text(song.title.clone()).size(12).style(tstyle(tc)),
                                text(song.artist.clone()).size(10).style(tstyle(t.muted)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding(6)
                    .on_press(Message::PlayQueueIndex(i))
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if is_current {
                                Some(Background::Color(t.accent_dim))
                            } else if h {
                                Some(Background::Color(t.surface2))
                            } else {
                                None
                            },
                            text_color: t.text,
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
                );
            }
            scrollable(list).height(Length::Fill).into()
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(320.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    fn lyrics_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = row![
            text("LYRICS").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, Message::TogglePanel(Panel::Lyrics)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match &self.lyrics {
            None => text("Loading lyrics…").size(12).style(tstyle(t.muted)).into(),
            Some(lr) if lr.lines.is_empty() => {
                text("No lyrics available").size(12).style(tstyle(t.muted)).into()
            }
            Some(lr) => {
                let pos_ms = (self.position * 1000.0) as i64;
                let cur = if lr.synced {
                    lr.lines.iter().rposition(|l| l.start <= pos_ms)
                } else {
                    None
                };
                let mut col = column![].spacing(8);
                for (i, line) in lr.lines.iter().enumerate() {
                    let active = Some(i) == cur;
                    let value = if line.value.trim().is_empty() { "♪".to_string() } else { line.value.clone() };
                    // Karaoke word-fill: LRC only carries line-level timing, so
                    // approximate per-word progress by distributing the line's
                    // window evenly across its words.
                    if active && self.lyrics_word_fill && lr.synced && value.split_whitespace().next().is_some() {
                        let end = lr.lines.get(i + 1).map(|n| n.start).unwrap_or(line.start + 4000);
                        let span = (end - line.start).max(1) as f64;
                        let frac = ((pos_ms - line.start) as f64 / span).clamp(0.0, 1.0);
                        let words: Vec<&str> = value.split_whitespace().collect();
                        let filled = (frac * words.len() as f64).ceil() as usize;
                        let mut wr = row![].spacing(6);
                        for (wi, w) in words.iter().enumerate() {
                            let wc = if wi < filled { t.accent } else { t.muted };
                            wr = wr.push(text(w.to_string()).size(16).style(tstyle(wc)));
                        }
                        col = col.push(wr);
                    } else {
                        let (sz, c) = if active { (16.0_f32, t.accent) } else { (13.0_f32, t.muted) };
                        col = col.push(text(value).size(sz).style(tstyle(c)));
                    }
                }
                scrollable(col).height(Length::Fill).into()
            }
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(320.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    fn home_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let username = {
            let conn = self.backend.app_state.connection.read();
            conn.username.clone().unwrap_or_default()
        };
        scrollable(
            column![
                row![
                    text(format!("Good {},", time_of_day())).size(22).style(tstyle(t.muted)),
                    text(username).size(22).style(tstyle(t.text)).font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::MONOSPACE
                    }),
                ]
                .spacing(8),
                self.home_section("RECENTLY ADDED", &self.home_recent),
                self.home_section("NEWEST", &self.home_newest),
                self.home_section("RANDOM PICKS", &self.home_random),
                self.home_genres(),
            ]
            .spacing(28),
        )
        .height(Length::Fill)
        .into()
    }

    /// Genre pills on the home page; tap one to browse its songs.
    fn home_genres(&self) -> Element<'_, Message> {
        let t = self.tokens;
        if self.genres.is_empty() {
            return column![].into();
        }
        let mut chips = row![].spacing(8);
        for g in self.genres.iter().take(30) {
            let name = g.name.clone();
            chips = chips.push(
                button(text(g.name.clone()).size(12).style(tstyle(t.text)))
                    .padding([6, 12])
                    .on_press(Message::Navigate(View::GenreDetail(name)))
                    .style(move |_th, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                            text_color: if h { t.text } else { t.muted },
                            border: Border { radius: 2.0.into(), color: t.border, width: 1.0 },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        column![
            text("GENRES").size(11).style(tstyle(t.muted)),
            scrollable(chips)
                .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())),
        ]
        .spacing(10)
        .into()
    }

    fn home_section(&self, title: &'static str, albums: &[Album]) -> Element<'_, Message> {
        let t = self.tokens;
        if albums.is_empty() {
            return column![].into();
        }
        let mut cards = row![].spacing(12);
        for a in albums.iter().take(12) {
            cards = cards.push(self.album_card(a));
        }
        column![
            text(title).size(11).style(tstyle(t.muted)),
            scrollable(cards)
                .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())),
        ]
        .spacing(12)
        .into()
    }

    fn album_card(&self, album: &Album) -> Element<'_, Message> {
        let t = self.tokens;
        button(
            column![
                self.cover_image(album.cover_art_id.as_deref(), 130.0),
                text(album.name.clone()).size(12).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::MONOSPACE
                }),
                text(album.album_artist.clone()).size(11).style(tstyle(t.muted)),
            ]
            .spacing(6)
            .width(Length::Fixed(130.0)),
        )
        .padding(4)
        .on_press(Message::Navigate(View::AlbumDetail(album.id.clone())))
        .style(move |_th, status| {
            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if h { Some(Background::Color(t.surface)) } else { None },
                text_color: t.text,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
    }

    fn similar_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = row![
            text("SIMILAR TRACKS").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, Message::TogglePanel(Panel::Similar)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = if self.similar_results.is_empty() {
            text("No similar tracks found").size(12).style(tstyle(t.muted)).into()
        } else {
            let mut col = column![].spacing(2);
            for m in &self.similar_results {
                col = col.push(self.song_row(&m.song));
            }
            scrollable(col).height(Length::Fill).into()
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(320.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    fn audio_stats_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = row![
            text("AUDIO STATS").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, Message::TogglePanel(Panel::AudioStats)),
        ]
        .align_y(Alignment::Center);

        let song = if self.queue_idx >= 0 { self.queue.get(self.queue_idx as usize) } else { None };
        let body: Element<'_, Message> = match song {
            None => text("Nothing playing").size(12).style(tstyle(t.muted)).into(),
            Some(s) => {
                let mut col = column![].spacing(8);
                col = col.push(stat_row("Format", s.track_info.clone().unwrap_or_else(|| "—".to_string()), t));
                col = col.push(stat_row("BPM", s.bpm.map(|b| format!("{b:.0}")).unwrap_or_else(|| "—".to_string()), t));
                if let Some(sr) = s.sampling_rate {
                    col = col.push(stat_row("Sample rate", format!("{:.1} kHz", sr as f32 / 1000.0), t));
                }
                if let Some(bd) = s.bit_depth {
                    col = col.push(stat_row("Bit depth", format!("{bd}-bit"), t));
                }
                if let Some(br) = s.bit_rate {
                    col = col.push(stat_row("Bitrate", format!("{br} kbps"), t));
                }
                if let Some(rg) = &s.replay_gain {
                    if let Some(v) = rg.get("trackGain").and_then(|v| v.as_f64()) {
                        col = col.push(stat_row("Track gain", format!("{v:+.1} dB"), t));
                    }
                    if let Some(v) = rg.get("albumGain").and_then(|v| v.as_f64()) {
                        col = col.push(stat_row("Album gain", format!("{v:+.1} dB"), t));
                    }
                    if let Some(v) = rg.get("trackPeak").and_then(|v| v.as_f64()) {
                        col = col.push(stat_row("Track peak", format!("{v:.3}"), t));
                    }
                }
                scrollable(col).height(Length::Fill).into()
            }
        };

        container(column![header, body].spacing(12))
            .width(Length::Fixed(300.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    fn mix_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        column![
            text("Mix").size(22).style(tstyle(t.text)),
            text("Generate a shuffled queue by energy level").size(12).style(tstyle(t.muted)),
            row![
                mix_button("Chill", Energy::Chill, t),
                mix_button("Mid", Energy::Mid, t),
                mix_button("High", Energy::High, t),
            ]
            .spacing(12),
        ]
        .spacing(16)
        .into()
    }

    fn eq_panel(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = row![
            text("EQUALIZER").size(11).style(tstyle(t.muted)).width(Length::Fill),
            icon_button(icons::CLOSE, 14.0, t.muted, t, Message::TogglePanel(Panel::Equalizer)),
        ]
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = match &self.eq_state {
            None => text("Loading…").size(12).style(tstyle(t.muted)).into(),
            Some(eq) => {
                let enabled = setting_toggle("Enabled", eq.enabled, Message::SetEqEnabled, t);

                let mut profs = column![].spacing(4);
                for p in &eq.profiles {
                    let active = eq.active_profile.as_deref() == Some(p.name.as_str());
                    let label = if p.imported {
                        format!("{} (imported)", p.name)
                    } else {
                        p.name.clone()
                    };
                    let name = p.name.clone();
                    profs = profs.push(
                        button(text(label).size(12).style(tstyle(if active { t.bg } else { t.text })))
                            .width(Length::Fill)
                            .padding(8)
                            .on_press(Message::SetEqProfile(name))
                            .style(move |_th, status| {
                                let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                                button::Style {
                                    background: Some(Background::Color(if active {
                                        t.accent
                                    } else if h {
                                        t.surface
                                    } else {
                                        t.surface2
                                    })),
                                    text_color: if active { t.bg } else { t.text },
                                    border: Border { radius: 4.0.into(), ..Border::default() },
                                    ..button::Style::default()
                                }
                            }),
                    );
                }

                let bands_ui: Element<'_, Message> = match eq
                    .active_profile
                    .as_ref()
                    .and_then(|a| eq.profiles.iter().find(|p| &p.name == a))
                {
                    Some(p) => {
                        let mut col = column![].spacing(8);
                        for (i, b) in p.bands.iter().enumerate() {
                            col = col.push(
                                row![
                                    text(fmt_freq(b.freq)).size(10).style(tstyle(t.muted)).width(Length::Fixed(46.0)),
                                    slider(-12.0..=12.0, b.gain, move |g| Message::EqBandChanged(i, g)).step(0.5).width(Length::Fill),
                                    text(format!("{:+.1}", b.gain)).size(10).style(tstyle(t.muted)).width(Length::Fixed(40.0)),
                                ]
                                .spacing(8)
                                .align_y(Alignment::Center),
                            );
                        }
                        col.into()
                    }
                    None => text("Select a profile").size(12).style(tstyle(t.muted)).into(),
                };

                // Save the current bands as a new named profile.
                let save_row = row![
                    text_input("New profile name…", &self.eq_new_profile_name)
                        .on_input(Message::EqNewProfileInput)
                        .on_submit(Message::SaveEqProfile)
                        .padding(6)
                        .size(12)
                        .width(Length::Fill),
                    button(text("Save").size(12).style(tstyle(t.bg)))
                        .padding(6)
                        .on_press(Message::SaveEqProfile)
                        .style(primary_button(t)),
                ]
                .spacing(6)
                .align_y(Alignment::Center);

                // Delete control for the active profile, unless it's read-only.
                let active_custom = eq
                    .active_profile
                    .as_ref()
                    .and_then(|a| eq.profiles.iter().find(|p| &p.name == a))
                    .filter(|p| !p.imported)
                    .map(|p| p.name.clone());
                let delete_row: Element<'_, Message> = match active_custom {
                    Some(name) => button(text(format!("Delete \"{name}\"")).size(12).style(tstyle(t.error)))
                        .padding(6)
                        .on_press(Message::DeleteEqProfile(name))
                        .style(move |_th, status| {
                            let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                            button::Style {
                                background: if h { Some(Background::Color(t.surface)) } else { None },
                                text_color: t.error,
                                border: Border { color: t.error, width: 1.0, radius: 4.0.into() },
                                ..button::Style::default()
                            }
                        })
                        .into(),
                    None => container(text("")).into(),
                };

                column![
                    enabled,
                    section_label("PROFILES", t),
                    profs,
                    save_row,
                    delete_row,
                    section_label("BANDS", t),
                    bands_ui,
                ]
                .spacing(10)
                .into()
            }
        };

        container(column![header, scrollable(body).height(Length::Fill)].spacing(12))
            .width(Length::Fixed(340.0))
            .height(Length::Fill)
            .padding(16)
            .style(fill_bg(t.surface))
            .into()
    }

    fn player_bar(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let song = if self.queue_idx >= 0 { self.queue.get(self.queue_idx as usize) } else { None };

        let cover = self.cover_image(song.and_then(|s| s.cover_art_id.as_deref()), 44.0);
        let info = column![
            text(song.map(|s| s.title.clone()).unwrap_or_else(|| "—".to_string())).size(13).style(tstyle(t.text)),
            text(song.map(|s| s.artist.clone()).unwrap_or_default()).size(11).style(tstyle(t.muted)),
            text(song.and_then(|s| s.track_info.clone()).unwrap_or_default()).size(10).style(tstyle(t.muted)),
        ]
        .spacing(2)
        .width(Length::Fixed(180.0));
        let volume = row![
            icons::icon(icons::VOLUME, 16.0, t.muted),
            slider(0.0..=1.0, self.volume, Message::SetVolume).step(0.01).width(Length::Fixed(55.0)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        let left = container(row![cover, info, volume].spacing(14).align_y(Alignment::Center))
            .width(Length::Fixed(240.0));

        let dur = self.duration.unwrap_or(0.0).max(0.1) as f32;
        let pos = (self.position as f32).clamp(0.0, dur);
        let center = container(
            row![
                text(fmt_time(self.position)).size(11).style(tstyle(t.muted)),
                slider(0.0..=dur, pos, Message::SeekTo).step(0.5).width(Length::Fill),
                text(fmt_time(self.duration.unwrap_or(0.0))).size(11).style(tstyle(t.muted)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill);

        let playing = matches!(self.playback_state, PlaybackState::Playing);
        let pp_icon = if playing { icons::PAUSE } else { icons::PLAY };
        let shuffle_color = if self.shuffle { t.accent } else { t.muted };
        let repeat_color = if self.repeat_one || self.repeat_all { t.accent } else { t.muted };
        let viz_color = if self.right_panel == Some(Panel::Visualizer) { t.accent } else { t.muted };
        let lyr_color = if self.right_panel == Some(Panel::Lyrics) { t.accent } else { t.muted };
        let q_color = if self.right_panel == Some(Panel::Queue) { t.accent } else { t.muted };
        let as_color = if self.right_panel == Some(Panel::AudioStats) { t.accent } else { t.muted };
        let sim_color = if self.right_panel == Some(Panel::Similar) { t.accent } else { t.muted };

        let controls = row![
            ctrl_button(icons::PREV, 15.0, t.text, t, Message::Prev),
            main_ctrl_button(pp_icon, 20.0, t, Message::TogglePlay),
            ctrl_button(icons::NEXT, 15.0, t.text, t, Message::Next),
            ctrl_button(icons::SHUFFLE, 16.0, shuffle_color, t, Message::ToggleShuffle),
            ctrl_button(icons::REPEAT, 16.0, repeat_color, t, Message::CycleRepeat),
            ctrl_button(icons::LYRICS, 16.0, lyr_color, t, Message::TogglePanel(Panel::Lyrics)),
            ctrl_button(icons::QUEUE, 16.0, q_color, t, Message::TogglePanel(Panel::Queue)),
            ctrl_button(icons::INFO, 16.0, as_color, t, Message::TogglePanel(Panel::AudioStats)),
            ctrl_button(icons::PLAY_CIRCLE, 16.0, sim_color, t, Message::TogglePanel(Panel::Similar)),
            ctrl_button(icons::WAVEFORM, 16.0, viz_color, t, Message::TogglePanel(Panel::Visualizer)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let right = container(controls).width(Length::Fixed(280.0));

        container(row![left, center, right].spacing(12).align_y(Alignment::Center))
            .width(Length::Fill)
            .height(Length::Fixed(75.0))
            .padding(iced::Padding { top: 0.0, right: 30.0, bottom: 0.0, left: 30.0 })
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface)),
                border: Border { color: t.border, width: 1.0, ..Border::default() },
                ..container::Style::default()
            })
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let bus = Subscription::run_with(BusSub(self.backend.bus.clone()), bus_stream);
        if self.right_panel == Some(Panel::Visualizer) {
            Subscription::batch([
                bus,
                iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::VisualizerTick),
            ])
        } else {
            bus
        }
    }
}

// ── Style helpers ───────────────────────────────────────────────────────────

fn time_of_day() -> &'static str {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() % 86400)
        .unwrap_or(0);
    match (secs / 3600) as u32 {
        5..=11 => "Morning",
        12..=16 => "Afternoon",
        17..=20 => "Evening",
        _ => "Night",
    }
}

fn tstyle(c: Color) -> impl Fn(&Theme) -> text::Style {
    move |_| text::Style { color: Some(c) }
}

fn fill_bg(c: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(c)),
        ..container::Style::default()
    }
}

fn primary_button(t: Tokens) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if hovered {
                Color { a: 0.85, ..t.accent }
            } else {
                t.accent
            })),
            text_color: t.bg,
            border: Border { radius: 2.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    }
}

fn section_label(label: &'static str, t: Tokens) -> Element<'static, Message> {
    text(label).size(11).style(tstyle(t.muted)).into()
}

/// One settings row: bold title + muted description on the left, control on the right.
fn sett_row<'a>(
    title: impl Into<String>,
    desc: impl Into<String>,
    t: Tokens,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        column![
            text(title.into()).size(14).style(tstyle(t.text)).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            }),
            text(desc.into()).size(11).style(tstyle(t.muted)),
        ]
        .spacing(4)
        .width(Length::Fill),
        control,
    ]
    .spacing(16)
    .align_y(Alignment::Start)
    .padding([15, 10])
    .into()
}

/// Bordered panel heading for a settings category content area.
fn sett_panel_title<'a>(title: impl Into<String>, t: Tokens) -> Element<'a, Message> {
    container(
        text(title.into()).size(16).style(tstyle(t.text)).font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..iced::Font::MONOSPACE
        }),
    )
    .padding(iced::Padding { top: 16.0, right: 10.0, bottom: 12.0, left: 10.0 })
    .width(Length::Fill)
    .style(move |_| container::Style {
        border: Border { color: t.border, width: 1.0, ..Border::default() },
        ..container::Style::default()
    })
    .into()
}

fn list_row_style(t: Tokens) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered { Some(Background::Color(t.surface)) } else { None },
            text_color: t.text,
            border: Border { radius: 2.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    }
}

fn back_button<'a>(t: Tokens) -> Element<'a, Message> {
    button(
        row![
            icons::icon(icons::BACK, 14.0, t.muted),
            text("Back").size(12).style(tstyle(t.muted)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding(6)
    .on_press(Message::NavigateBack)
    .style(move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered { Some(Background::Color(t.surface)) } else { None },
            text_color: t.muted,
            border: Border { radius: 4.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    })
    .into()
}

fn setting_toggle<'a>(label: &'a str, on: bool, on_toggle: fn(bool) -> Message, t: Tokens) -> Element<'a, Message> {
    row![
        text(label).size(13).style(tstyle(t.text)).width(Length::Fill),
        toggler(on).on_toggle(on_toggle),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn icon_button<'a>(src: &'static str, size: f32, color: Color, t: Tokens, msg: Message) -> Element<'a, Message> {
    button(icons::icon(src, size, color))
        .padding(6)
        .on_press(msg)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                text_color: color,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
}

/// Circular player-control button (matches the old `.ctrl-btn` style).
fn ctrl_button<'a>(src: &'static str, size: f32, color: Color, t: Tokens, msg: Message) -> Element<'a, Message> {
    button(icons::icon(src, size, color))
        .padding(8)
        .on_press(msg)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                text_color: color,
                border: Border { radius: 100.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
}

/// Main play/pause button — always shows a circle background, accent on hover.
fn main_ctrl_button<'a>(src: &'static str, size: f32, t: Tokens, msg: Message) -> Element<'a, Message> {
    button(icons::icon(src, size, t.text))
        .padding(10)
        .on_press(msg)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if hovered { t.accent } else { t.surface2 })),
                text_color: if hovered { t.bg } else { t.text },
                border: Border { radius: 100.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
}

/// Approximate album-row height (cover 44 + padding) and assumed viewport
/// height, used by the windowed album list.
const ALBUM_ROW_H: f32 = 60.0;
const VIEWPORT_H: f32 = 640.0;

fn stat_row(label: &'static str, val: String, t: Tokens) -> Element<'static, Message> {
    row![
        text(label).size(12).style(tstyle(t.muted)).width(Length::Fill),
        text(val).size(12).style(tstyle(t.text)),
    ]
    .into()
}

fn mix_button<'a>(label: &'static str, e: Energy, t: Tokens) -> Element<'a, Message> {
    button(text(label).size(14).style(tstyle(t.bg)))
        .padding(14)
        .on_press(Message::GenerateMix(e))
        .style(primary_button(t))
        .into()
}

fn fmt_freq(f: f32) -> String {
    if f >= 1000.0 {
        let k = f / 1000.0;
        if k.fract() == 0.0 {
            format!("{k:.0}k")
        } else {
            format!("{k:.1}k")
        }
    } else {
        format!("{f:.0}")
    }
}

/// Keep songs whose BPM falls in the energy band; fall back to the whole pool
/// if too few have BPM tags. Caps at 60 tracks.
fn filter_energy(songs: Vec<Song>, e: Energy) -> Vec<Song> {
    let in_band = |s: &Song| match s.bpm {
        Some(b) => match e {
            Energy::Chill => b < 95.0,
            Energy::Mid => (95.0..130.0).contains(&b),
            Energy::High => b >= 130.0,
        },
        None => false,
    };
    let filtered: Vec<Song> = songs.iter().filter(|s| in_band(s)).cloned().collect();
    let mut out = if filtered.len() >= 10 { filtered } else { songs };
    out.truncate(60);
    out
}

fn fmt_time(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "0:00".to_string();
    }
    let s = secs as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Open a save dialog and write `contents` to the chosen path. Returns
/// `Ok(false)` if the user cancelled the dialog.
async fn save_export(default_name: String, ext: String, contents: String) -> Result<bool, String> {
    let handle = rfd::AsyncFileDialog::new()
        .set_file_name(&default_name)
        .add_filter("export", &[ext.as_str()])
        .save_file()
        .await;
    let Some(handle) = handle else { return Ok(false) };
    crate::commands::stats::save_text_file(handle.path().to_string_lossy().to_string(), contents)?;
    Ok(true)
}

/// "3h 24m" / "47m" style duration for Recap totals.
fn fmt_hours(secs: i64) -> String {
    let secs = secs.max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

struct BusSub(EventBus);

impl Hash for BusSub {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "firmium-event-bus".hash(state);
    }
}

fn bus_stream(data: &BusSub) -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::SinkExt;
    use tokio::sync::broadcast::error::RecvError;

    let bus = data.0.clone();
    iced::stream::channel(64, move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
        let mut rx = bus.subscribe();
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let _ = output.send(Message::Backend(event)).await;
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    })
}
