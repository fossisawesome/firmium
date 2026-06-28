//! Top-level iced application: `App` state, `Message`, `update`, `view`, the
//! event-bus subscription, and (Phase 7) the onboarding flow + Albums view.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

use iced::widget::image::Handle as ImageHandle;
use iced::widget::{button, checkbox, column, container, pick_list, row, scrollable, slider, stack, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length, Shadow, Subscription, Task, Theme};

use crate::commands::equalizer::EqState;
use crate::commands::lyrics::LyricsResult;
use crate::commands::mappers::{Album, Artist, SimilarMatch, Song};
use crate::podcasts::{PodcastChannel, PodcastEpisode};
use crate::playlists::Playlist;
use crate::commands::subsonic::{AlbumTracks, ArtistDetails, ArtistInfo, Genre, PlaylistTracks, RemotePlayQueue, SearchResult};
use crate::commands::themes::ThemeEntry;
use crate::config::{Config, SavedAccount};
use crate::errors::UserError;
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
    Recap,
    Settings,
    Podcasts,
    PodcastDetail(String),
}

/// A row in the merged playlists list: either a local playlist (index into
/// `App.playlists`) or a server-only playlist (index into `App.server_playlists`)
/// not claimed by any local `server_id`. Mirrors Android `PlaylistListItem`.
#[derive(Debug, Clone)]
enum PlaylistListItem {
    Local(usize),
    ServerOnly(usize),
}

impl View {
    #[allow(dead_code)]
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
            View::Recap => "Recap",
            View::Settings => "Settings",
            View::Podcasts => "Podcasts",
            View::PodcastDetail(_) => "Podcast",
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
    #[allow(clippy::wrong_self_convention)]
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
pub struct Toast {
    pub id: u64,
    pub category: UserError,
    pub text: String,
    pub spawned: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(View),
    NavigateBack,
    Backend(BackendEvent),
    VisualizerTick,
    #[expect(dead_code)]
    ShowToast(UserError),
    DismissToast(u64),
    ToastTick,

    // ── Onboarding ────────────────────────────────────────────────────────────
    ServerInput(String),
    UsernameInput(String),
    PasswordInput(String),
    Connect,
    Connected(Result<(), UserError>),
    CredentialsLoaded(Option<String>),
    ServiceCredsLoaded(String, String, String),
    ToggleSavePassword(bool),

    // ── Data ──────────────────────────────────────────────────────────────────
    AlbumsLoaded(Result<Vec<Album>, UserError>),
    HomeAlbumsLoaded(HomeSection, Result<Vec<Album>, UserError>),
    AlbumTracksLoaded(Result<AlbumTracks, UserError>),
    ArtistsLoaded(Result<Vec<Artist>, UserError>),
    ArtistDetailLoaded(Result<ArtistDetails, UserError>),
    ArtistInfoLoaded(Result<Option<ArtistInfo>, UserError>),
    SimilarArtistsLoaded(Result<Vec<String>, UserError>),
    PlaylistsLoaded(Result<Vec<serde_json::Value>, UserError>),
    PlaylistTracksLoaded(Result<PlaylistTracks, UserError>),
    CoverLoaded(String, Result<String, String>),
    AlbumsScrolled(f32),
    ArtistsScrolled(f32),
    AlbumTracksScrolled(f32),
    PlaylistTracksScrolled(f32),
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

    // ── Local-first playlist management ───────────────────────────────────────
    CreatePlaylist(String),
    PlaylistCreateSynced(String, Result<serde_json::Value, UserError>),
    DeleteLocalPlaylist(String),
    RenamePlaylist(String, String),
    SyncPlaylistNow(String),
    MovePlaylistTrack(String, usize, usize),
    RemovePlaylistTrack(String, String),
    MoveServerTrack(String, usize, usize),
    RemoveServerTrack(String, usize),
    OpenCreatePlaylist,
    CloseCreatePlaylist,
    CreatePlaylistNameInput(String),
    StartRenamePlaylist(String),
    CommitRenamePlaylist,
    PlaylistSyncNoop,

    // ── Search ────────────────────────────────────────────────────────────────
    SearchInput(String),
    SubmitSearch,
    SearchLoaded(Result<SearchResult, UserError>),
    SetSearchRatingFilter(u32),

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
    MixFetched(Energy, Result<Vec<Song>, UserError>),

    // ── Transport ─────────────────────────────────────────────────────────────
    TogglePlay,
    Next,
    Prev,
    #[allow(dead_code)]
    ToggleShuffle,
    CycleRepeat,
    SetVolume(f32),
    SeekTo(f32),
    TogglePanel(Panel),
    SetVizMode(VizMode),
    SetVizCoverColors(bool),
    VizColorsLoaded(String, Result<crate::commands::cover_colors::CoverColorsResult, String>),
    LyricsLoaded(String, Result<Option<LyricsResult>, UserError>),
    SimilarLoaded(String, Result<Vec<SimilarMatch>, UserError>),
    PlayQueueIndex(usize),
    PlaybackDone(Result<(), String>),

    // ── Resume-queue prompt ───────────────────────────────────────────────────
    PlayQueueFetched(Result<Option<RemotePlayQueue>, UserError>),
    ResumeQueue,
    DismissResume,

    // ── Account switcher ──────────────────────────────────────────────────────
    ToggleAccountSwitcher,
    #[allow(dead_code)]
    SwitchAccount(SavedAccount),
    #[allow(dead_code)]
    AddAccount,

    // ── Recap ─────────────────────────────────────────────────────────────────
    SetRecapRange(RecapRange),
    RecapNext,
    RecapPrev,

    // ── Listening stats ───────────────────────────────────────────────────────
    ExportStats(String),
    ExportDone(Result<bool, String>),

    // ── Genre browsing ────────────────────────────────────────────────────────
    GenresLoaded(Result<Vec<Genre>, UserError>),
    GenreSongsLoaded(Result<Vec<Song>, UserError>),
    PlayGenreAt(usize),

    // ── Album download ────────────────────────────────────────────────────────
    DownloadAlbum,

    // ── Podcasts ──────────────────────────────────────────────────────────────
    PodcastChannelsLoaded(Result<Vec<PodcastChannel>, String>),
    OpenAddPodcastModal,
    CloseAddPodcastModal,
    PodcastAddUrlChanged(String),
    SubmitAddPodcastChannel,
    PodcastChannelAdded(Result<PodcastChannel, String>),
    PodcastEpisodesLoaded(Result<Vec<PodcastEpisode>, String>),
    RefreshPodcastChannel(String, String),
    PodcastChannelRefreshed(Result<usize, String>),
    UnsubscribePodcastChannel(String),
    PodcastChannelUnsubscribed(Result<(), String>),
    PlayPodcastEpisode(PodcastEpisode),
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
    save_password: bool,
    server_input: String,
    username_input: String,
    password_input: String,

    toasts: Vec<Toast>,
    next_toast_id: u64,

    view: View,
    nav_stack: Vec<View>,

    // ── Library data ──────────────────────────────────────────────────────────
    albums: Vec<Album>,
    albums_scroll: f32,
    home_recent: Vec<Album>,
    home_newest: Vec<Album>,
    home_random: Vec<Album>,
    home_recent_plays: Vec<crate::db::RecentPlay>,
    // Deduplicated (artist_id, name, cover_art_id) derived from home_recent_plays;
    // recomputed only when home_recent_plays changes so view() doesn't rebuild it per frame.
    home_recent_artists_cache: Vec<(String, String, Option<String>)>,
    album_detail: Option<AlbumTracks>,
    album_detail_id: Option<String>,
    album_tracks_scroll: f32,
    artists: Vec<Artist>,
    artists_scroll: f32,
    artist_detail: Option<ArtistDetails>,
    artist_detail_id: Option<String>,
    artist_info: Option<ArtistInfo>,
    similar_artists: Vec<String>,
    server_playlists: Vec<serde_json::Value>,
    playlists: Vec<Playlist>,
    playlist_items: Vec<PlaylistListItem>,
    playlist_detail: Option<PlaylistTracks>,
    playlist_detail_id: Option<String>,
    playlist_tracks_scroll: f32,
    cover_cache: HashMap<String, ImageHandle>,
    // Insertion order for cover_cache, used to evict the oldest decoded handles
    // once MAX_COVER_HANDLES is exceeded (the disk cache in cover_cache.rs is
    // bounded separately; this bounds in-memory decoded images).
    cover_cache_order: VecDeque<String>,

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
    // When true, the visualizer gradient is derived from the current track's
    // cover art; otherwise it follows the active theme's colors.
    viz_cover_colors: bool,
    viz_palette: Option<crate::commands::cover_colors::OrbPalette>,
    // Track id the current viz_palette was extracted for, so it's fetched once per track.
    viz_palette_track: Option<String>,
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
    search_rating_filter: u32,

    // ── Add-to-playlist overlay ───────────────────────────────────────────────
    add_to_playlist_song: Option<Song>,
    new_playlist_name: String,
    show_create_playlist: bool,
    create_playlist_name: String,
    renaming_playlist: Option<String>,

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

    // ── Podcasts ───────────────────────────────────────────────────────────────
    podcast_channels: Vec<PodcastChannel>,
    podcast_episodes: Vec<PodcastEpisode>,
    podcast_add_url_input: String,
    podcast_add_modal_open: bool,
    podcast_add_error: Option<String>,
    // The episode loaded into `current_player_id`, if the active player session
    // is a podcast episode rather than a queued `Song` — used to persist resume position.
    current_podcast_episode: Option<PodcastEpisode>,
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
        // Keyring reads are deferred to async tasks in initial_task() to avoid
        // blocking the iced event loop thread (first libsecret call establishes
        // a D-Bus connection, which can block ~500ms–1s on Linux).
        let connecting = false;
        let (server_input, username_input) = match (&cfg.server, &cfg.username) {
            (Some(server), Some(user)) => (server.clone(), user.clone()),
            _ => (String::new(), String::new()),
        };
        let lastfm_key = String::new();
        let lastfm_secret = String::new();
        let listenbrainz_token = String::new();

        let mut app = Self {
            backend,
            themes,
            theme_id,
            tokens,
            authed: false,
            connecting,
            save_password: true,
            server_input,
            username_input,
            password_input: String::new(),
            toasts: Vec::new(),
            next_toast_id: 0,
            view: View::Home,
            nav_stack: Vec::new(),
            albums: Vec::new(),
            albums_scroll: 0.0,
            home_recent: Vec::new(),
            home_newest: Vec::new(),
            home_random: Vec::new(),
            home_recent_plays: Vec::new(),
            home_recent_artists_cache: Vec::new(),
            album_detail: None,
            album_detail_id: None,
            album_tracks_scroll: 0.0,
            artists: Vec::new(),
            artists_scroll: 0.0,
            artist_detail: None,
            artist_detail_id: None,
            artist_info: None,
            similar_artists: Vec::new(),
            server_playlists: Vec::new(),
            playlists: crate::playlists::load_playlists(),
            playlist_items: Vec::new(),
            playlist_detail: None,
            playlist_detail_id: None,
            playlist_tracks_scroll: 0.0,
            cover_cache: HashMap::new(),
            cover_cache_order: VecDeque::new(),
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
            viz_cover_colors: cfg.viz_cover_colors.unwrap_or(true),
            viz_palette: None,
            viz_palette_track: None,
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
            search_rating_filter: 0,
            add_to_playlist_song: None,
            new_playlist_name: String::new(),
            show_create_playlist: false,
            create_playlist_name: String::new(),
            renaming_playlist: None,
            resume_queue: None,
            accounts: cfg.accounts,
            show_account_switcher: true,
            recap: None,
            recap_range: RecapRange::Month,
            recap_card: 0,
            history_summary: None,
            genres: Vec::new(),
            genre_songs: Vec::new(),
            genre_detail_name: None,
            eq_new_profile_name: String::new(),
            podcast_channels: Vec::new(),
            podcast_episodes: Vec::new(),
            podcast_add_url_input: String::new(),
            podcast_add_modal_open: false,
            podcast_add_error: None,
            current_podcast_episode: None,
        };
        app.rebuild_playlist_items();
        if !app.authed {
            app.populate_offline_library();
        }
        app
    }

    pub fn theme(&self) -> Theme {
        self.tokens.iced_theme()
    }

    /// Spawn startup tasks: async keyring reads so the event loop isn't blocked.
    pub fn initial_task(&self) -> Task<Message> {
        // Cover art for the offline-library fallback populated in `App::new()`.
        let offline_cover_task = Task::batch([self.load_covers(), self.load_cover_ids(self.offline_home_cover_ids())]);

        // Always load service credentials (lastfm, listenbrainz) from keyring.
        let service_task = Task::perform(
            async {
                tokio::task::spawn_blocking(|| {
                    let read = |key: &str| {
                        crate::commands::credentials::get_password(Some("firmium-desktop"), key)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default()
                    };
                    (read("lastfm_key"), read("lastfm_secret"), read("listenbrainz_token"))
                })
                .await
                .unwrap_or_default()
            },
            |(k, s, t)| Message::ServiceCredsLoaded(k, s, t),
        );

        // If server + username are in config, fetch the saved password to auto-login.
        if !self.server_input.is_empty() && !self.username_input.is_empty() {
            let server = self.server_input.clone();
            let user = self.username_input.clone();
            let creds_task = Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        crate::commands::credentials::get_password(Some(&server), &user).ok()
                    })
                    .await
                    .unwrap_or(None)
                },
                Message::CredentialsLoaded,
            );
            Task::batch([creds_task, service_task, offline_cover_task])
        } else {
            Task::batch([service_task, offline_cover_task])
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
            viz_cover_colors: Some(self.viz_cover_colors),
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
                        self.album_tracks_scroll = 0.0;
                        if id.starts_with("local:") {
                            let result = crate::commands::local_library::get_local_album_tracks(&self.backend.app_state, id)
                                .map(|r| crate::commands::subsonic::AlbumTracks {
                                    tracks: r.tracks,
                                    album_name: r.album_name,
                                    album_artist: r.album_artist,
                                    cover_art_id: r.cover_art_id,
                                })
                                .map_err(|_| UserError::NotFound);
                            Task::done(Message::AlbumTracksLoaded(result))
                        } else {
                            Task::perform(crate::commands::subsonic::get_album_tracks(state, id), Message::AlbumTracksLoaded)
                        }
                    }
                    View::ArtistDetail(id) if self.artist_detail_id.as_deref() != Some(id.as_str()) => {
                        self.artist_detail = None;
                        self.artist_info = None;
                        self.similar_artists.clear();
                        self.artist_detail_id = Some(id.clone());
                        if id.starts_with("local:") {
                            let result = crate::commands::local_library::get_local_artist_details(&self.backend.app_state, id)
                                .map(|r| crate::commands::subsonic::ArtistDetails { name: r.name, albums: r.albums })
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
                                Task::perform(crate::commands::subsonic::get_artist_details(state.clone(), id.clone()), Message::ArtistDetailLoaded),
                                Task::perform(crate::commands::subsonic::get_artist_info(state.clone(), id.clone(), self.lastfm_key.clone(), artist_name), Message::ArtistInfoLoaded),
                                Task::perform(crate::commands::subsonic::get_similar_artists(state, id, None), Message::SimilarArtistsLoaded),
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
                                crate::commands::subsonic::get_playlist_tracks(state, server_id.to_string()),
                                Message::PlaylistTracksLoaded,
                            )
                        } else {
                            // Local playlist: build detail from memory, no fetch.
                            self.refresh_local_detail(&id);
                            Task::none()
                        }
                    }
                    View::Artists if self.artists.is_empty() => {
                        Task::perform(crate::commands::subsonic::get_artists(state), Message::ArtistsLoaded)
                    }
                    View::Playlists => {
                        Task::perform(crate::commands::subsonic::get_playlists(state), Message::PlaylistsLoaded)
                    }
                    View::Recap => self.compute_recap(),
                    View::GenreDetail(name) if self.genre_detail_name.as_deref() != Some(name.as_str()) => {
                        self.genre_songs.clear();
                        self.genre_detail_name = Some(name.clone());
                        Task::perform(crate::commands::subsonic::get_songs_by_genre(state, name, None), Message::GenreSongsLoaded)
                    }
                    View::Settings => {
                        self.load_history_summary();
                        Task::none()
                    }
                    View::Podcasts => {
                        if let Some(store) = self.backend.podcasts.clone() {
                            Task::perform(
                                async move { crate::podcasts::list_channels(store) },
                                Message::PodcastChannelsLoaded,
                            )
                        } else {
                            Task::none()
                        }
                    }
                    View::PodcastDetail(id) => {
                        if let Some(store) = self.backend.podcasts.clone() {
                            Task::perform(
                                async move { crate::podcasts::list_episodes(store, id) },
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
                                Task::perform(crate::commands::subsonic::get_recent_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Recent, r)),
                                Task::perform(crate::commands::subsonic::get_newest_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Newest, r)),
                                Task::perform(crate::commands::subsonic::get_random_albums(state.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Random, r)),
                                Task::perform(crate::commands::subsonic::get_genres_list(state), Message::GenresLoaded),
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
                Task::batch([self.maybe_fetch_lyrics(), self.maybe_fetch_similar(), self.maybe_fetch_viz_colors()])
            }
            Message::VisualizerTick => Task::none(),
            Message::ShowToast(err) => { self.show_toast(err); Task::none() }
            Message::DismissToast(id) => { self.toasts.retain(|t| t.id != id); Task::none() }
            Message::ToastTick => {
                self.toasts.retain(|t| t.spawned.elapsed().as_secs() < 5);
                Task::none()
            }

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
                crate::commands::subsonic::set_connection(
                    &self.backend.app_state,
                    Some(server.clone()),
                    Some(user.clone()),
                    Some(pass.clone()),
                );
                if self.save_password {
                    let _ = crate::commands::credentials::save_password(Some(&server), &user, &pass);
                }
                Task::perform(
                    crate::commands::subsonic::validate_connection(self.backend.app_state.clone()),
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
                    Task::perform(crate::commands::subsonic::get_albums(s.clone()), Message::AlbumsLoaded),
                    Task::perform(crate::commands::subsonic::get_recent_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Recent, r)),
                    Task::perform(crate::commands::subsonic::get_newest_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Newest, r)),
                    Task::perform(crate::commands::subsonic::get_random_albums(s.clone(), 12), |r| Message::HomeAlbumsLoaded(HomeSection::Random, r)),
                    Task::perform(crate::commands::subsonic::get_genres_list(s.clone()), Message::GenresLoaded),
                    Task::perform(crate::commands::subsonic::get_play_queue(s.clone()), Message::PlayQueueFetched),
                    Task::perform(crate::podcasts::probe_server_podcast_support(s), |_| Message::PlaylistSyncNoop),
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
                crate::commands::subsonic::set_connection(
                    &self.backend.app_state,
                    Some(self.server_input.clone()),
                    Some(self.username_input.clone()),
                    Some(pass),
                );
                self.connecting = true;
                Task::perform(
                    crate::commands::subsonic::validate_connection(self.backend.app_state.clone()),
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
            Message::PlaylistTracksScrolled(y) => {
                self.playlist_tracks_scroll = y;
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
            Message::PlaylistsLoaded(Ok(p)) => {
                self.server_playlists = p;
                // Adopt same-named server playlists for unsynced locals (avoid dups);
                // retry creating the rest that are still under the attempt cap.
                let mut to_retry: Vec<(String, String, Vec<String>)> = Vec::new();
                for local in self.playlists.iter_mut() {
                    if local.server_id.is_some() || !local.create_pending {
                        continue;
                    }
                    if local.create_attempts >= crate::playlists::CREATE_ATTEMPT_CAP {
                        continue;
                    }
                    let same = self.server_playlists.iter().find(|sp| {
                        sp.get("name").and_then(|v| v.as_str()) == Some(local.name.as_str())
                    });
                    if let Some(sp) = same {
                        local.server_id = sp.get("id").and_then(|v| v.as_str()).map(String::from);
                        local.create_pending = false;
                    } else {
                        let ids = local.tracks.iter().map(|s| s.id.clone()).collect();
                        to_retry.push((local.id.clone(), local.name.clone(), ids));
                    }
                }
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                let tasks = to_retry.into_iter().map(|(local_id, name, ids)| {
                    Task::perform(
                        crate::commands::playlists::sync_create(
                            self.backend.app_state.clone(),
                            name,
                            ids,
                        ),
                        move |res| Message::PlaylistCreateSynced(local_id.clone(), res),
                    )
                });
                Task::batch(tasks)
            }
            Message::PlaylistsLoaded(Err(e)) => {
                eprintln!("get_playlists failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::PlaylistTracksLoaded(Ok(pt)) => {
                let ids: Vec<String> = pt.tracks.iter().filter_map(|s| s.cover_art_id.clone()).collect();
                self.playlist_detail = Some(pt);
                self.load_cover_ids(ids)
            }
            Message::PlaylistTracksLoaded(Err(e)) => {
                eprintln!("get_playlist_tracks failed: {e:?}");
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
            Message::OpenAddToPlaylist(song) => {
                self.add_to_playlist_song = Some(song);
                self.new_playlist_name.clear();
                // Lazily load the playlist list the first time the overlay opens.
                if self.server_playlists.is_empty() {
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
            Message::AddToPlaylist(local_id) => {
                if let Some(song) = self.add_to_playlist_song.take() {
                    let added = crate::playlists::add_tracks(&mut self.playlists, &local_id, vec![song]);
                    crate::playlists::save_playlists(&self.playlists);
                    self.rebuild_playlist_items();
                    self.refresh_local_detail(&local_id);
                    let server_id = self
                        .playlists
                        .iter()
                        .find(|p| p.id == local_id)
                        .and_then(|p| p.server_id.clone());
                    match server_id {
                        Some(sid) => Task::perform(
                            crate::commands::playlists::push_add(self.backend.app_state.clone(), sid, added),
                            |_| Message::PlaylistSyncNoop,
                        ),
                        None => Task::none(),
                    }
                } else {
                    Task::none()
                }
            }
            Message::CreatePlaylistAndAdd => {
                let name = self.new_playlist_name.trim().to_string();
                match self.add_to_playlist_song.take() {
                    Some(song) if !name.is_empty() => {
                        let mut p = crate::playlists::new_local(name.clone());
                        let local_id = p.id.clone();
                        p.tracks.push(song);
                        let track_ids: Vec<String> = p.tracks.iter().map(|s| s.id.clone()).collect();
                        self.playlists.insert(0, p);
                        crate::playlists::save_playlists(&self.playlists);
                        self.rebuild_playlist_items();
                        self.new_playlist_name.clear();
                        Task::perform(
                            crate::commands::playlists::sync_create(
                                self.backend.app_state.clone(),
                                name,
                                track_ids,
                            ),
                            move |res| Message::PlaylistCreateSynced(local_id.clone(), res),
                        )
                    }
                    // Nothing to do if the name is blank; keep the overlay open.
                    other => {
                        self.add_to_playlist_song = other;
                        Task::none()
                    }
                }
            }
            Message::PlaylistSyncNoop => Task::none(),

            // ── Local-first playlist management ──────────────────────────────────
            Message::OpenCreatePlaylist => {
                self.create_playlist_name.clear();
                self.show_create_playlist = true;
                Task::none()
            }
            Message::CloseCreatePlaylist => {
                self.show_create_playlist = false;
                Task::none()
            }
            Message::CreatePlaylistNameInput(s) => {
                self.create_playlist_name = s;
                Task::none()
            }
            Message::CreatePlaylist(name) => {
                let name = name.trim().to_string();
                self.show_create_playlist = false;
                if name.is_empty() {
                    return Task::none();
                }
                let p = crate::playlists::new_local(name.clone());
                let local_id = p.id.clone();
                self.playlists.insert(0, p);
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                Task::perform(
                    crate::commands::playlists::sync_create(
                        self.backend.app_state.clone(),
                        name,
                        Vec::new(),
                    ),
                    move |res| Message::PlaylistCreateSynced(local_id.clone(), res),
                )
            }
            Message::PlaylistCreateSynced(local_id, Ok(server_pl)) => {
                let server_id = server_pl.get("id").and_then(|v| v.as_str()).map(String::from);
                if let Some(p) = self.playlists.iter_mut().find(|p| p.id == local_id) {
                    p.server_id = server_id;
                    p.create_pending = false;
                }
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                Task::none()
            }
            Message::PlaylistCreateSynced(local_id, Err(e)) => {
                eprintln!("playlist create sync failed: {e:?}");
                if let Some(p) = self.playlists.iter_mut().find(|p| p.id == local_id) {
                    p.create_attempts += 1;
                    p.create_pending = p.create_attempts < crate::playlists::CREATE_ATTEMPT_CAP;
                }
                crate::playlists::save_playlists(&self.playlists);
                Task::none()
            }
            Message::SyncPlaylistNow(local_id) => {
                let Some(p) = self.playlists.iter().find(|p| p.id == local_id) else {
                    return Task::none();
                };
                if p.server_id.is_some() {
                    return Task::none();
                }
                let name = p.name.clone();
                let track_ids: Vec<String> = p.tracks.iter().map(|s| s.id.clone()).collect();
                let lid = local_id.clone();
                Task::perform(
                    crate::commands::playlists::sync_create(
                        self.backend.app_state.clone(),
                        name,
                        track_ids,
                    ),
                    move |res| Message::PlaylistCreateSynced(lid.clone(), res),
                )
            }
            Message::DeleteLocalPlaylist(local_id) => {
                let server_id = self
                    .playlists
                    .iter()
                    .find(|p| p.id == local_id)
                    .and_then(|p| p.server_id.clone());
                self.playlists.retain(|p| p.id != local_id);
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                // If the open detail belonged to this playlist, go back to the list.
                if self.playlist_detail_id.as_deref() == Some(local_id.as_str()) {
                    self.view = View::Playlists;
                    self.playlist_detail = None;
                    self.playlist_detail_id = None;
                }
                match server_id {
                    Some(sid) => Task::perform(
                        crate::commands::playlists::push_delete(self.backend.app_state.clone(), sid),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    None => Task::none(),
                }
            }
            Message::RenamePlaylist(local_id, name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }
                let mut server_id = None;
                if let Some(p) = self.playlists.iter_mut().find(|p| p.id == local_id) {
                    p.name = name.clone();
                    server_id = p.server_id.clone();
                }
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                self.refresh_local_detail(&local_id);
                match server_id {
                    Some(sid) => Task::perform(
                        crate::commands::playlists::push_rename(self.backend.app_state.clone(), sid, name),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    None => Task::none(),
                }
            }
            Message::StartRenamePlaylist(id) => {
                self.create_playlist_name = self
                    .playlists
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                self.renaming_playlist = Some(id);
                Task::none()
            }
            Message::CommitRenamePlaylist => {
                match self.renaming_playlist.take() {
                    Some(id) => {
                        let name = self.create_playlist_name.clone();
                        self.update(Message::RenamePlaylist(id, name))
                    }
                    None => Task::none(),
                }
            }
            Message::MovePlaylistTrack(local_id, from, to) => {
                let ordered = crate::playlists::move_track(&mut self.playlists, &local_id, from, to);
                if ordered.is_none() {
                    return Task::none();
                }
                crate::playlists::save_playlists(&self.playlists);
                self.refresh_local_detail(&local_id);
                let server_id = self
                    .playlists
                    .iter()
                    .find(|p| p.id == local_id)
                    .and_then(|p| p.server_id.clone());
                match (server_id, ordered) {
                    (Some(sid), Some(ids)) => Task::perform(
                        crate::commands::playlists::push_reorder(self.backend.app_state.clone(), sid, ids),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    _ => Task::none(),
                }
            }
            Message::RemovePlaylistTrack(local_id, track_id) => {
                let idx = crate::playlists::remove_track(&mut self.playlists, &local_id, &track_id);
                if idx.is_none() {
                    return Task::none();
                }
                crate::playlists::save_playlists(&self.playlists);
                self.refresh_local_detail(&local_id);
                self.rebuild_playlist_items();
                let server_id = self
                    .playlists
                    .iter()
                    .find(|p| p.id == local_id)
                    .and_then(|p| p.server_id.clone());
                match (server_id, idx) {
                    (Some(sid), Some(i)) => Task::perform(
                        crate::commands::playlists::push_remove(self.backend.app_state.clone(), sid, i as u32),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    _ => Task::none(),
                }
            }
            Message::MoveServerTrack(server_id, from, to) => {
                let Some(pt) = &mut self.playlist_detail else {
                    return Task::none();
                };
                let n = pt.tracks.len();
                if from >= n || to >= n || from == to {
                    return Task::none();
                }
                let moved = pt.tracks.remove(from);
                pt.tracks.insert(to, moved);
                let ids: Vec<String> = pt.tracks.iter().map(|s| s.id.clone()).collect();
                Task::perform(
                    crate::commands::playlists::push_reorder(self.backend.app_state.clone(), server_id, ids),
                    |_| Message::PlaylistSyncNoop,
                )
            }
            Message::RemoveServerTrack(server_id, index) => {
                let Some(pt) = &mut self.playlist_detail else {
                    return Task::none();
                };
                if index >= pt.tracks.len() {
                    return Task::none();
                }
                pt.tracks.remove(index);
                Task::perform(
                    crate::commands::playlists::push_remove(self.backend.app_state.clone(), server_id, index as u32),
                    |_| Message::PlaylistSyncNoop,
                )
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
                eprintln!("search failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::SetSearchRatingFilter(n) => {
                self.search_rating_filter = if self.search_rating_filter == n { 0 } else { n };
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
                if on {
                    if self.gapless_enabled {
                        self.gapless_enabled = false;
                        crate::commands::queue::set_gapless_enabled(&self.backend.bus, &self.backend.queue_state, false);
                    }
                    if self.bit_perfect_mode == "strict" {
                        self.bit_perfect_mode = "relaxed".to_string();
                        self.backend.audio_player.set_bit_perfect_mode("relaxed".to_string());
                    }
                }
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
                if on && self.crossfade_enabled {
                    self.crossfade_enabled = false;
                    crate::commands::queue::set_crossfade_settings(&self.backend.bus, &self.backend.queue_state, false, self.crossfade_duration);
                }
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
                self.cover_cache_order.clear();
                Task::none()
            }
            Message::DeleteSettings => {
                // Reset preference fields to defaults (connection/account untouched).
                self.download_format = "raw".to_string();
                self.lrclib_enabled = true;
                self.lyrics_word_fill = false;
                self.window_decorations = true;
                self.viz_cover_colors = true;
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
                self.show_account_switcher = true;
                self.search_results = None;
                self.populate_offline_library();
                self.save_config();
                Task::batch([self.load_covers(), self.load_cover_ids(self.offline_home_cover_ids())])
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
                eprintln!("mix fetch failed: {e:?}");
                self.show_toast(e);
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
                Task::batch([self.maybe_fetch_lyrics(), self.maybe_fetch_similar(), self.maybe_fetch_viz_colors()])
            }
            Message::SetVizMode(m) => {
                self.visualizer_mode = m;
                Task::none()
            }
            Message::SetVizCoverColors(on) => {
                self.viz_cover_colors = on;
                self.save_config();
                self.maybe_fetch_viz_colors()
            }
            Message::VizColorsLoaded(track_id, res) => {
                if self.viz_palette_track.as_deref() == Some(track_id.as_str()) {
                    if let Ok(colors) = res {
                        self.viz_palette = Some(colors.orb);
                    }
                }
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
                eprintln!("get_play_queue failed: {e:?}");
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
            Message::PodcastChannelsLoaded(Ok(channels)) => {
                self.podcast_channels = channels;
                Task::none()
            }
            Message::PodcastChannelsLoaded(Err(e)) => {
                eprintln!("Failed to load podcast channels: {e}");
                Task::none()
            }
            Message::OpenAddPodcastModal => {
                self.podcast_add_modal_open = true;
                self.podcast_add_error = None;
                Task::none()
            }
            Message::CloseAddPodcastModal => {
                self.podcast_add_modal_open = false;
                self.podcast_add_url_input.clear();
                Task::none()
            }
            Message::PodcastAddUrlChanged(url) => {
                self.podcast_add_url_input = url;
                Task::none()
            }
            Message::SubmitAddPodcastChannel => {
                let url = self.podcast_add_url_input.clone();
                if url.trim().is_empty() {
                    return Task::none();
                }
                if let Some(store) = self.backend.podcasts.clone() {
                    let state = self.backend.app_state.clone();
                    Task::perform(crate::podcasts::add_channel(state, store, url), Message::PodcastChannelAdded)
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelAdded(Ok(channel)) => {
                self.podcast_channels.push(channel);
                self.podcast_add_modal_open = false;
                self.podcast_add_url_input.clear();
                self.podcast_add_error = None;
                Task::none()
            }
            Message::PodcastChannelAdded(Err(e)) => {
                self.podcast_add_error = Some(e);
                Task::none()
            }
            Message::PodcastEpisodesLoaded(Ok(episodes)) => {
                self.podcast_episodes = episodes;
                Task::none()
            }
            Message::PodcastEpisodesLoaded(Err(e)) => {
                eprintln!("Failed to load podcast episodes: {e}");
                Task::none()
            }
            Message::RefreshPodcastChannel(channel_id, feed_url) => {
                if let Some(store) = self.backend.podcasts.clone() {
                    let state = self.backend.app_state.clone();
                    Task::perform(
                        crate::podcasts::refresh_channel(state, store, channel_id, feed_url),
                        Message::PodcastChannelRefreshed,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelRefreshed(Ok(_new_count)) => {
                if let View::PodcastDetail(channel_id) = self.view.clone() {
                    if let Some(store) = self.backend.podcasts.clone() {
                        return Task::perform(
                            async move { crate::podcasts::list_episodes(store, channel_id) },
                            Message::PodcastEpisodesLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::PodcastChannelRefreshed(Err(e)) => {
                eprintln!("Failed to refresh podcast channel: {e}");
                Task::none()
            }
            Message::UnsubscribePodcastChannel(channel_id) => {
                if let Some(store) = self.backend.podcasts.clone() {
                    Task::perform(
                        async move { crate::podcasts::unsubscribe(store, channel_id) },
                        Message::PodcastChannelUnsubscribed,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelUnsubscribed(Ok(())) => {
                if let Some(store) = self.backend.podcasts.clone() {
                    Task::perform(
                        async move { crate::podcasts::list_channels(store) },
                        Message::PodcastChannelsLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PodcastChannelUnsubscribed(Err(e)) => {
                eprintln!("Failed to unsubscribe podcast channel: {e}");
                Task::none()
            }
            Message::PlayPodcastEpisode(episode) => {
                let resume_secs = episode.position_ms as f64 / 1000.0;
                match crate::audio::AudioPlayer::play_stream(
                    &self.backend.audio_player,
                    &episode.audio_url,
                    episode.id.clone(),
                    None,
                ) {
                    Ok(player_id) => {
                        self.current_player_id = Some(player_id.clone());
                        self.current_podcast_episode = Some(episode);
                        if resume_secs > 0.0 {
                            let _ = self.backend.audio_player.seek(&player_id, resume_secs);
                        }
                    }
                    Err(e) => eprintln!("Failed to play podcast episode: {e}"),
                }
                Task::none()
            }
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

    /// Loads Home/Albums/Artists from the on-disk local library (`~/Music/Firmium`)
    /// so browsing works without a server connection, mirroring Android's
    /// `if (auth.isAuthenticated) api... else localLibrary...` fallback. Overwritten
    /// by server data once a connection succeeds (see `Connected(Ok(()))`).
    fn populate_offline_library(&mut self) {
        self.albums = crate::commands::local_library::get_local_albums(&self.backend.app_state).unwrap_or_default();
        self.artists = crate::commands::local_library::get_local_artists(&self.backend.app_state).unwrap_or_default();
        self.home_recent = crate::commands::local_library::get_local_recent_albums(&self.backend.app_state, 12).unwrap_or_default();
        self.home_newest = crate::commands::local_library::get_local_newest_albums(&self.backend.app_state, 12).unwrap_or_default();
        self.home_random = crate::commands::local_library::get_local_random_albums(&self.backend.app_state, 12).unwrap_or_default();
    }

    fn offline_home_cover_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .home_recent
            .iter()
            .chain(self.home_newest.iter())
            .chain(self.home_random.iter())
            .filter_map(|a| a.cover_art_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
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
            if cid.starts_with("local:") {
                let state = self.backend.app_state.clone();
                let arg_id = cid.clone();
                tasks.push(Task::perform(
                    crate::commands::local_library::get_local_cover_art_async(state, arg_id),
                    move |res| Message::CoverLoaded(cid.clone(), res),
                ));
            } else if let Ok(url) = crate::commands::subsonic::build_cover_url(&self.backend.app_state, &cid, 300) {
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

    /// Build the visualizer config, overriding its gradient with either the
    /// current cover-art palette or the active theme's colors.
    fn viz_config(&self) -> crate::viz::VizConfig {
        let gradient_colors = match (self.viz_cover_colors, &self.viz_palette) {
            (true, Some(p)) => ramp8(
                rgb_to_color(p.primary),
                rgb_to_color(p.secondary),
                rgb_to_color(p.tertiary),
            ),
            _ => self.theme_gradient(),
        };
        crate::viz::VizConfig { gradient_colors, ..crate::viz::VizConfig::default() }
    }

    /// An 8-stop gradient derived from the active theme's accent colors, used
    /// when cover coloring is off or no cover palette is available.
    fn theme_gradient(&self) -> Vec<iced::Color> {
        let t = self.tokens;
        let lighten = |c: iced::Color, k: f32| iced::Color {
            r: c.r + (1.0 - c.r) * k,
            g: c.g + (1.0 - c.g) * k,
            b: c.b + (1.0 - c.b) * k,
            a: c.a,
        };
        ramp8(t.accent, lighten(t.accent, 0.35), t.accent_dim)
    }

    /// Extract the cover-art palette for the current track when the Visualizer
    /// panel is open, the cover-color option is on, and the track changed.
    fn maybe_fetch_viz_colors(&mut self) -> Task<Message> {
        if self.right_panel != Some(Panel::Visualizer) || !self.viz_cover_colors {
            return Task::none();
        }
        let song = if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).cloned()
        } else {
            None
        };
        let Some(song) = song else {
            self.viz_palette = None;
            self.viz_palette_track = None;
            return Task::none();
        };
        if self.viz_palette_track.as_deref() == Some(song.id.as_str()) {
            return Task::none();
        }
        self.viz_palette_track = Some(song.id.clone());
        // No cover → fall back to the theme gradient (handled in viz_config).
        let Some(cover_id) = song.cover_art_id.clone() else {
            self.viz_palette = None;
            return Task::none();
        };
        let Ok(url) = crate::commands::subsonic::build_cover_url(&self.backend.app_state, &cover_id, 300) else {
            return Task::none();
        };
        let track_id = song.id.clone();
        Task::perform(
            crate::commands::cover_colors::extract_cover_colors(cover_id, url),
            move |res| Message::VizColorsLoaded(track_id.clone(), res),
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
                    if let Some(episode) = &self.current_podcast_episode {
                        if let Some(store) = &self.backend.podcasts {
                            let position_ms = (position * 1000.0) as i64;
                            if let Err(e) = store.update_position(&episode.id, position_ms) {
                                eprintln!("Failed to save podcast position: {e}");
                            }
                        }
                    }
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
                self.current_podcast_episode = None;
            }
            BackendEvent::QueueExhausted(_song) => {}
            BackendEvent::SessionExpired => {
                self.authed = false;
                self.show_account_switcher = true;
            }
        }
    }

    // ── View ──────────────────────────────────────────────────────────────────

    pub fn view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let base_el: Element<'_, Message> = container(self.shell())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(fill_bg(t.bg))
            .into();
        if self.toasts.is_empty() {
            base_el
        } else {
            stack![base_el, self.toast_host()].into()
        }
    }

    /// Pushes a toast, coalescing by category (no duplicate visible) and capping
    /// the visible stack at 3 (drops oldest). `SessionExpired` is suppressed — the
    /// existing SessionExpired event already drives the UI.
    fn show_toast(&mut self, err: UserError) {
        if matches!(err, UserError::SessionExpired) {
            return;
        }
        if let Some(existing) = self.toasts.iter_mut().find(|t| t.category == err) {
            existing.spawned = std::time::Instant::now();
            return;
        }
        self.toasts.push(Toast {
            id: self.next_toast_id,
            text: err.message(),
            category: err.clone(),
            spawned: std::time::Instant::now(),
        });
        self.next_toast_id += 1;
        while self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    fn toast_host(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let cards: Vec<Element<Message>> = self.toasts.iter().map(|toast| {
            let close = icon_button(icons::CLOSE, 14.0, t.muted, t, Message::DismissToast(toast.id));
            container(
                row![text(toast.text.clone()).size(14), close]
                    .spacing(12)
                    .align_y(Alignment::Center),
            )
            .padding(12)
            .style(fill_bg(t.surface))
            .into()
        }).collect();
        container(column(cards).spacing(8))
            .padding(16)
            .align_x(Alignment::End)
            .align_y(Alignment::End)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn shell(&self) -> Element<'_, Message> {
        let t = self.tokens;

        let brand = container(
            row![
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
            self.nav_button(icons::PODCAST, "Podcasts", View::Podcasts),
            self.nav_button(icons::SEARCH, "Search", View::Search),
            self.nav_button(icons::MUSIC, "Mix", View::Mix),
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
        } else if self.show_create_playlist {
            stack![base, self.create_playlist_overlay()].into()
        } else if self.podcast_add_modal_open {
            stack![base, self.add_podcast_overlay()].into()
        } else {
            base.into()
        }
    }

    fn add_podcast_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let can_add = !self.podcast_add_url_input.trim().is_empty();

        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::CloseAddPodcastModal)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let add_msg = can_add.then(|| Message::SubmitAddPodcastChannel);
        let mut card_col = column![
            text("Add a podcast").size(16).style(tstyle(t.text)),
            text_input("RSS feed URL…", &self.podcast_add_url_input)
                .on_input(Message::PodcastAddUrlChanged)
                .on_submit(Message::SubmitAddPodcastChannel)
                .padding(10)
                .size(13)
                .style(text_input_style(t)),
        ]
        .spacing(16);

        if let Some(err) = &self.podcast_add_error {
            card_col = card_col.push(text(err).size(12).style(tstyle(t.muted)));
        }

        card_col = card_col.push(
            row![
                button(text("Cancel").size(13).style(tstyle(t.muted)))
                    .padding(8)
                    .on_press(Message::CloseAddPodcastModal)
                    .style(list_row_style(t)),
                button(text("Add").size(13).style(tstyle(if can_add { t.bg } else { t.muted })))
                    .padding([8, 16])
                    .on_press_maybe(add_msg)
                    .style(primary_button(t)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );

        let card = container(card_col)
            .width(Length::Fixed(420.0))
            .padding(24)
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

    fn create_playlist_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let can_create = !self.create_playlist_name.trim().is_empty();

        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::CloseCreatePlaylist)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let create_msg = can_create.then(|| Message::CreatePlaylist(self.create_playlist_name.clone()));
        let card = container(
            column![
                text("New Playlist").size(16).style(tstyle(t.text)),
                text_input("Playlist name…", &self.create_playlist_name)
                    .on_input(Message::CreatePlaylistNameInput)
                    .on_submit(Message::CreatePlaylist(self.create_playlist_name.clone()))
                    .padding(10)
                    .size(13)
                    .style(text_input_style(t)),
                row![
                    button(text("Cancel").size(13).style(tstyle(t.muted)))
                        .padding(8)
                        .on_press(Message::CloseCreatePlaylist)
                        .style(list_row_style(t)),
                    button(text("Create").size(13).style(tstyle(if can_create { t.bg } else { t.muted })))
                        .padding([8, 16])
                        .on_press_maybe(create_msg)
                        .style(primary_button(t)),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ]
            .spacing(16),
        )
        .width(Length::Fixed(360.0))
        .padding(24)
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

    /// Modal: login form when not authed, connected info + disconnect when authed.
    fn saved_account_row(&self, acct: &SavedAccount) -> Element<'_, Message> {
        let t = self.tokens;
        let server_display = acct
            .server
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        button(
            column![
                text(acct.username.clone()).size(13).style(tstyle(t.text)),
                text(server_display).size(11).style(tstyle(t.muted)),
            ]
            .spacing(2),
        )
        .width(Length::Fixed(320.0))
        .padding(10)
        .on_press(Message::SwitchAccount(acct.clone()))
        .style(list_row_style(t))
        .into()
    }

    fn account_switcher_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;

        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::ToggleAccountSwitcher)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let card: Element<'_, Message> = if self.authed {
            let (cur_server, cur_username) = {
                let conn = self.backend.app_state.connection.read();
                (conn.server.clone().unwrap_or_default(), conn.username.clone().unwrap_or_default())
            };
            let server_display = cur_server
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/')
                .to_string();
            let other_accounts: Vec<&SavedAccount> = self
                .accounts
                .iter()
                .filter(|a| a.server != cur_server || a.username != cur_username)
                .collect();

            let disconnect_btn = button(text("DISCONNECT").size(13))
                .on_press(Message::Logout)
                .padding(14)
                .width(Length::Fixed(320.0))
                .style(move |_, status| {
                    use iced::widget::button::Status;
                    let bg = match status {
                        Status::Hovered | Status::Pressed => Color {
                            r: t.error.r * 0.85,
                            g: t.error.g * 0.85,
                            b: t.error.b * 0.85,
                            ..t.error
                        },
                        _ => t.error,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::BLACK,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                });

            let mut card_col = column![
                text("Connected").size(26).style(tstyle(t.accent)),
                text(server_display).size(13).style(tstyle(t.muted)),
            ]
            .spacing(20)
            .align_x(Alignment::Start);

            if !other_accounts.is_empty() {
                let mut switch_col = column![text("SWITCH ACCOUNT").size(11).style(tstyle(t.muted))].spacing(8);
                for acct in &other_accounts {
                    switch_col = switch_col.push(self.saved_account_row(acct));
                }
                card_col = card_col.push(switch_col);
            }
            card_col = card_col.push(disconnect_btn);

            container(card_col)
                .width(Length::Fixed(400.0))
                .padding(40)
                .style(move |_th| container::Style {
                    background: Some(Background::Color(t.surface)),
                    border: Border { radius: 10.0.into(), width: 1.0, color: t.border },
                    ..container::Style::default()
                })
                .into()
        } else {
            let mut card_col = column![].spacing(20).align_x(Alignment::Start);
            if !self.accounts.is_empty() {
                let mut switch_col = column![text("SAVED ACCOUNTS").size(11).style(tstyle(t.muted))].spacing(8);
                for acct in &self.accounts {
                    switch_col = switch_col.push(self.saved_account_row(acct));
                }
                card_col = card_col.push(switch_col);
                card_col = card_col.push(text("OR CONNECT TO A NEW SERVER").size(11).style(tstyle(t.muted)));
            }

            let save_pw_row = row![
                checkbox(self.save_password)
                    .on_toggle(Message::ToggleSavePassword)
                    .style(move |_, status| {
                        use iced::widget::checkbox::{Status, Style};
                        let checked = matches!(status, Status::Active { is_checked: true } | Status::Hovered { is_checked: true });
                        Style {
                            background: Background::Color(if checked { t.accent } else { t.surface }),
                            icon_color: t.bg,
                            border: Border { color: if checked { t.accent } else { t.border }, width: 1.0, radius: 3.0.into() },
                            text_color: None,
                        }
                    }),
                text("SAVE PASSWORD").size(11).style(tstyle(t.muted)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fixed(320.0));

            let form = column![
                text_input("https://music.example.com", &self.server_input)
                    .on_input(Message::ServerInput)
                    .padding(10)
                    .width(Length::Fixed(320.0))
                    .style(text_input_style(t)),
                text_input("username", &self.username_input)
                    .on_input(Message::UsernameInput)
                    .padding(10)
                    .width(Length::Fixed(320.0))
                    .style(text_input_style(t)),
                text_input("password", &self.password_input)
                    .on_input(Message::PasswordInput)
                    .secure(true)
                    .padding(10)
                    .width(Length::Fixed(320.0))
                    .style(text_input_style(t)),
                save_pw_row,
                button(text("CONNECT").size(13))
                    .on_press(Message::Connect)
                    .padding(14)
                    .width(Length::Fixed(320.0))
                    .style(primary_button(t)),
            ]
            .spacing(12)
            .align_x(Alignment::Start);
            card_col = card_col.push(form);

            container(card_col)
                .width(Length::Fixed(400.0))
                .padding(40)
                .style(move |_| container::Style {
                    background: Some(Background::Color(t.surface)),
                    border: Border { color: t.border, width: 1.0, radius: 10.0.into() },
                    ..container::Style::default()
                })
                .into()
        };

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
                .width(Length::Fill)
                .style(text_input_style(t)),
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
            for p in &self.playlists {
                let id = p.id.clone();
                let name = p.name.clone();
                let count = p.tracks.len();
                let synced = p.server_id.is_some();
                let mut label = row![
                    icons::icon(icons::LIST, 16.0, t.muted),
                    text(name).size(13).style(tstyle(t.text)).width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center);
                if synced {
                    label = label.push(icons::icon(icons::CLOUD, 12.0, t.muted));
                }
                label = label.push(text(format!("{count}")).size(11).style(tstyle(t.muted)));
                list = list.push(
                    button(label)
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
                scrollable(list).height(Length::Fixed(260.0)).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)),
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
            self.viz_config(),
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
            View::Recap => self.recap_view(),
            View::Settings => self.settings_view(),
            View::Podcasts => self.podcasts_view(),
            View::PodcastDetail(_) => self.podcast_detail_view(),
        }
    }

    fn search_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let bar = row![
            text_input("Search your library…", &self.search_query)
                .on_input(Message::SearchInput)
                .on_submit(Message::SubmitSearch)
                .padding(10)
                .width(Length::Fill)
                .style(text_input_style(t)),
            button(text("Search").size(13))
                .on_press(Message::SubmitSearch)
                .padding(10)
                .style(primary_button(t)),
        ]
        .spacing(10);

        let results: Element<'_, Message> = if let Some(res) = &self.search_results {
            let mut col = column![].spacing(4);
            col = col.push(self.rating_filter_row());
            if !res.albums.is_empty() {
                col = col.push(text("Albums").size(13).style(tstyle(t.muted)));
                for a in res.albums.iter().take(40) {
                    col = col.push(self.album_row(a));
                }
            }
            let filter = self.search_rating_filter;
            let filtered_songs: Vec<&Song> = res
                .songs
                .iter()
                .filter(|s| {
                    filter == 0
                        || s.user_rating.unwrap_or(0) >= filter
                        || s.average_rating.unwrap_or(0.0) >= filter as f32
                })
                .take(100)
                .collect();
            if !filtered_songs.is_empty() {
                col = col.push(text("Songs").size(13).style(tstyle(t.muted)));
                for s in filtered_songs {
                    col = col.push(self.song_row(s));
                }
            }
            scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).into()
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
            self.star_rating(song),
            self.avg_rating_badge(song),
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
            .style(fill_bg(t.bg));

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
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(thin_scrollbar()))
        .style(thin_scroll_style(t));

        row![sidebar, sep, container(panel).padding([0, 4]).width(Length::Fill)]
            .height(Length::Fill)
            .into()
    }

    fn settings_appearance(&self, t: Tokens) -> Element<'_, Message> {
        let selected = self.themes.iter().find(|e| e.id == self.theme_id).cloned();
        let theme_picker = pick_list(self.themes.clone(), selected, |entry: ThemeEntry| {
            Message::SelectTheme(entry.id)
        })
        .width(Length::Fixed(200.0))
        .into();
        column![
            sett_panel_title("Appearance", t),
            sett_row(
                "Window Decorations",
                "Show native title bar and borders",
                t,
                toggler(self.window_decorations).on_toggle(Message::SetDecorations).style(toggler_style(t)).into(),
            ),
            sett_row(
                "Cover-Colored Visualizer",
                "Tint the visualizer with the current album's artwork. When off, it follows your theme colors.",
                t,
                toggler(self.viz_cover_colors).on_toggle(Message::SetVizCoverColors).style(toggler_style(t)).into(),
            ),
            sett_row("Theme", "Color scheme for the interface", t, theme_picker),
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
                        .width(Length::Fixed(100.0))
                        .style(slider_style(t)),
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
                toggler(self.crossfade_enabled).on_toggle(Message::SetCrossfadeEnabled).style(toggler_style(t)).into()),
            crossfade_dur,
            sett_row("Gapless Playback", "Pre-buffer the next track for seamless transitions", t,
                toggler(self.gapless_enabled).on_toggle(Message::SetGapless).style(toggler_style(t)).into()),
            sett_row("ReplayGain", "Normalize track loudness using server-provided gain values", t,
                toggler(self.replay_gain_enabled).on_toggle(Message::SetReplayGain).style(toggler_style(t)).into()),
            sett_row("Continue playing after queue ends", "Smart Radio keeps the music going by adding similar tracks when the queue runs out", t,
                toggler(self.auto_continue).on_toggle(Message::SetAutoContinue).style(toggler_style(t)).into()),
            sett_row("Bit-Perfect Audio", "Matches native sample rate; crossfade is disabled", t,
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
        fn fmt_label(id: &str) -> &'static str {
            match id {
                "mp3" => "MP3",
                "flac" => "FLAC",
                "wav" => "WAV",
                "opus" => "Opus",
                _ => "Original",
            }
        }
        let selected = fmt_label(&self.download_format);
        let fmt_picker = pick_list(
            ["Original", "MP3", "FLAC", "WAV", "Opus"],
            Some(selected),
            |label: &'static str| {
                let id = match label {
                    "MP3" => "mp3",
                    "FLAC" => "flac",
                    "WAV" => "wav",
                    "Opus" => "opus",
                    _ => "raw",
                };
                Message::SetDownloadFormat(id.to_string())
            },
        )
        .width(Length::Fixed(200.0))
        .into();
        column![
            sett_panel_title("Downloads", t),
            sett_row(
                "Download Format",
                "Format used when downloading tracks and albums. \"Original\" saves the file exactly as stored on the server.",
                t,
                fmt_picker,
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
            toggler(self.lastfm_enabled).on_toggle(Message::SetLastfmEnabled).style(toggler_style(t)).into(),
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
                    .style(text_input_style(t))
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
                    .style(text_input_style(t))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "ListenBrainz Scrobbling",
            "Submit each completed track to ListenBrainz using your user token",
            t,
            toggler(self.listenbrainz_enabled).on_toggle(Message::SetListenbrainzEnabled).style(toggler_style(t)).into(),
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
                    .style(text_input_style(t))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "External Lyrics (LRCLIB)",
            "Fetch synced lyrics from lrclib.net when your server has none. Sends song title and artist name.",
            t,
            toggler(self.lrclib_enabled).on_toggle(Message::SetLrclibEnabled).style(toggler_style(t)).into(),
        ));
        col = col.push(sett_row(
            "Word-by-Word Lyrics Animation",
            "Karaoke-style fill on the active lyric line, with per-word timing estimated from the line's timestamps. Disable for plain line-by-line highlighting.",
            t,
            toggler(self.lyrics_word_fill).on_toggle(Message::SetLyricsWordFill).style(toggler_style(t)).into(),
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

        let (first, end, top, bottom) = list_window(at.tracks.len(), self.album_tracks_scroll, TRACK_ROW_H);
        let mut list = column![];
        if top > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(top)));
        }
        for (i, track) in at.tracks[first..end].iter().enumerate() {
            list = list.push(self.track_row(first + i, track, Message::PlayAlbumAt(first + i)));
        }
        if bottom > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(bottom)));
        }

        column![back, header, scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).on_scroll(|v| Message::AlbumTracksScrolled(v.absolute_offset().y))]
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
            self.avg_rating_badge(song),
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
                    .padding(4)
                    .on_press(Message::SetRating(sid, i))
                    .style(move |_th, status| {
                        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        stars.into()
    }

    fn avg_rating_badge(&self, song: &Song) -> Element<'_, Message> {
        let t = self.tokens;
        match song.average_rating {
            Some(r) if r > 0.0 => row![
                icons::icon(icons::STAR_FILLED, 11.0, t.muted),
                text(format!("{r:.1}")).size(11).style(tstyle(t.muted)),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into(),
            _ => row![].into(),
        }
    }

    fn rating_filter_row(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let active = self.search_rating_filter;
        let mut stars = row![].spacing(1);
        for i in 1..=5u32 {
            let filled = i <= active;
            let src = if filled { icons::STAR_FILLED } else { icons::STAR_EMPTY };
            let color = if filled { t.accent } else { t.muted };
            stars = stars.push(
                button(icons::icon(src, 14.0, color))
                    .padding(4)
                    .on_press(Message::SetSearchRatingFilter(i))
                    .style(move |_th, status| {
                        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    }),
            );
        }
        row![
            text("Min rating:").size(12).style(tstyle(t.muted)),
            stars,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn album_list_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = text(format!("Albums ({})", self.albums.len())).size(22).style(tstyle(t.text));

        // Windowed (virtual) rendering: only the visible rows are built; the
        // scrolled-past and remaining heights are filled with spacers so the
        // scrollbar stays correct for libraries with thousands of albums.
        let (first, end, top, bottom) = list_window(self.albums.len(), self.albums_scroll, ALBUM_ROW_H);
        let mut list = column![];
        if top > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(top)));
        }
        for album in &self.albums[first..end] {
            list = list.push(self.album_row(album));
        }
        if bottom > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(bottom)));
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
        let radius = if size >= 80.0 { 14.0_f32 } else if size >= 40.0 { 10.0 } else { 6.0 };
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
        let (first, end, top, bottom) = list_window(self.artists.len(), self.artists_scroll, ARTIST_ROW_H);
        let mut list = column![];
        if top > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(top)));
        }
        for artist in &self.artists[first..end] {
            list = list.push(self.artist_row(artist));
        }
        if bottom > 0.0 {
            list = list.push(container(text("")).height(Length::Fixed(bottom)));
        }
        column![header, scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).on_scroll(|v| Message::ArtistsScrolled(v.absolute_offset().y))].spacing(16).into()
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

        column![head, scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t))]
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
            scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)),
        ]
        .spacing(12)
        .into()
    }

    fn podcasts_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let header = row![
            text(format!("Podcasts ({})", self.podcast_channels.len()))
                .size(22)
                .style(tstyle(t.text))
                .width(Length::Fill),
            button(
                row![icons::icon(icons::PLUS, 12.0, t.accent), text("Add podcast").size(12).style(tstyle(t.accent))]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
            .padding([6, 14])
            .on_press(Message::OpenAddPodcastModal)
            .style(list_row_style(t)),
        ]
        .align_y(Alignment::Center);

        if self.podcast_channels.is_empty() {
            return column![
                header,
                text("No podcasts yet. Add one by RSS feed URL.").size(13).style(tstyle(t.muted))
            ]
            .spacing(16)
            .into();
        }

        let mut list = column![].spacing(2);
        for channel in &self.podcast_channels {
            list = list.push(
                button(
                    column![
                        text(&channel.title).size(14).style(tstyle(t.text)),
                        text(channel.description.clone().unwrap_or_default()).size(12).style(tstyle(t.muted)),
                    ]
                    .spacing(4),
                )
                .width(Length::Fill)
                .padding(10)
                .on_press(Message::Navigate(View::PodcastDetail(channel.id.clone())))
                .style(list_row_style(t)),
            );
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

    fn podcast_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let View::PodcastDetail(channel_id) = self.view.clone() else {
            return text("No channel selected").size(13).style(tstyle(t.muted)).into();
        };

        let channel = self.podcast_channels.iter().find(|c| c.id == channel_id);
        let channel_title = channel.map(|c| c.title.clone()).unwrap_or_default();
        let feed_url = channel.map(|c| c.feed_url.clone()).unwrap_or_default();

        let header = row![
            text(channel_title).size(20).style(tstyle(t.text)).width(Length::Fill),
            button(text("Refresh").size(12).style(tstyle(t.text)))
                .padding([6, 12])
                .on_press(Message::RefreshPodcastChannel(channel_id.clone(), feed_url))
                .style(list_row_style(t)),
            button(text("Unsubscribe").size(12).style(tstyle(t.text)))
                .padding([6, 12])
                .on_press(Message::UnsubscribePodcastChannel(channel_id.clone()))
                .style(list_row_style(t)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if self.podcast_episodes.is_empty() {
            return column![header, text("No episodes found.").size(13).style(tstyle(t.muted))]
                .spacing(16)
                .into();
        }

        let mut list = column![].spacing(2);
        for episode in &self.podcast_episodes {
            let duration_label = episode
                .duration_seconds
                .map(|s| fmt_time(s as f64))
                .unwrap_or_default();
            list = list.push(
                row![
                    column![
                        text(&episode.title).size(13).style(tstyle(t.text)),
                        text(duration_label).size(11).style(tstyle(t.muted)),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(icons::icon(icons::PLAY, 14.0, t.accent))
                        .padding(8)
                        .on_press(Message::PlayPodcastEpisode(episode.clone()))
                        .style(list_row_style(t)),
                ]
                .spacing(12)
                .padding(10)
                .align_y(Alignment::Center),
            );
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

    fn playlists_view(&self) -> Element<'_, Message> {
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

    /// Rebuilds `playlist_items` from the local + server lists: local first, then
    /// server playlists whose id is not already a local playlist's `server_id`.
    fn rebuild_playlist_items(&mut self) {
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

    /// If the playlist detail view currently shows local playlist `local_id`,
    /// rebuild its `playlist_detail` from the in-memory tracks (after a local edit).
    fn refresh_local_detail(&mut self, local_id: &str) {
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

    /// Cover for a playlist row: mosaic of distinct track covers (local) or the
    /// single server cover (server-only), falling back to the list icon.
    fn playlist_cover(&self, item: &PlaylistListItem) -> Element<'_, Message> {
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
            0 => icons::icon(icons::LIST, 22.0, t.muted).into(),
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

    fn playlist_row(&self, item: &PlaylistListItem) -> Element<'_, Message> {
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

    /// A playlist detail track row: the standard track row plus reorder (up/down)
    /// and remove controls, dispatching local vs server-only messages.
    fn playlist_track_row(
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
            .on_press_maybe((idx > 0).then(|| up_msg).flatten())
            .style(list_row_style(t));
        let down = button(icons::icon(icons::CHEVRON_DOWN, 14.0, t.muted))
            .padding(4)
            .on_press_maybe((idx + 1 < total).then(|| down_msg).flatten())
            .style(list_row_style(t));
        let remove = button(icons::icon(icons::CLOSE, 14.0, t.error))
            .padding(4)
            .on_press_maybe(remove_msg)
            .style(list_row_style(t));

        row![base, up, down, remove].spacing(6).align_y(Alignment::Center).into()
    }

    fn playlist_detail_view(&self) -> Element<'_, Message> {
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

    fn nav_button(&self, icon_src: &'static str, label: &'static str, target: View) -> Element<'_, Message> {
        let active = self.view == target;
        let t = self.tokens;
        let color = if active { t.accent } else { t.muted };
        let mut label_text = text(label).size(13).style(tstyle(color));
        if active {
            label_text = label_text.font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            });
        }
        let content = row![icons::icon(icon_src, 16.0, color), label_text]
            .spacing(10)
            .align_y(Alignment::Center);

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
            scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).into()
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
                scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).into()
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
                column![
                    text(format!("GOOD {},", time_of_day().to_uppercase()))
                        .size(13)
                        .style(tstyle(t.muted)),
                    text(username).size(36).style(tstyle(t.accent)).font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::MONOSPACE
                    }),
                ]
                .spacing(4),
                self.home_recent_songs_view(),
                self.home_recent_artists(),
                self.home_section("RANDOM PICKS", &self.home_random),
                self.home_genres(),
            ]
            .spacing(28)
            .padding(iced::Padding { right: 16.0, ..iced::Padding::ZERO }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(thin_scrollbar()))
        .style(thin_scroll_style(t))
        .into()
    }

    fn home_recent_songs_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        if self.home_recent_plays.is_empty() {
            return column![].into();
        }
        let mut cards = row![].spacing(12);
        for play in self.home_recent_plays.iter().take(5) {
            let artist = play.artist_name.clone().unwrap_or_default();
            let card_content = column![
                self.cover_image(play.cover_art_id.as_deref(), 130.0),
                text(play.track_title.clone()).size(12).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::MONOSPACE
                }),
                text(artist).size(11).style(tstyle(t.muted)),
            ]
            .spacing(6)
            .width(Length::Fixed(130.0));

            let card: Element<'_, Message> = if let Some(aid) = play.album_id.clone() {
                button(card_content)
                    .padding(4)
                    .on_press(Message::Navigate(View::AlbumDetail(aid)))
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
            } else {
                button(card_content)
                    .padding(4)
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
            };
            cards = cards.push(card);
        }
        column![
            text("RECENTLY PLAYED").size(11).style(tstyle(t.muted)),
            cards,
        ]
        .spacing(12)
        .into()
    }

    /// Insert a decoded cover handle, evicting the oldest entries once the
    /// in-memory budget is exceeded.
    fn cache_cover(&mut self, id: String, handle: ImageHandle) {
        if self.cover_cache.insert(id.clone(), handle).is_none() {
            self.cover_cache_order.push_back(id);
            while self.cover_cache_order.len() > MAX_COVER_HANDLES {
                if let Some(old) = self.cover_cache_order.pop_front() {
                    self.cover_cache.remove(&old);
                }
            }
        }
    }

    /// Rebuild the deduplicated recent-artists list. Called when
    /// `home_recent_plays` changes, not every frame.
    fn recompute_home_recent_artists(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.home_recent_artists_cache = self
            .home_recent_plays
            .iter()
            .filter_map(|p| {
                let id = p.artist_id.as_ref()?;
                let name = p.artist_name.as_ref()?;
                if seen.insert(id.clone()) {
                    Some((id.clone(), name.clone(), p.cover_art_id.clone()))
                } else {
                    None
                }
            })
            .collect();
    }

    fn home_recent_artists(&self) -> Element<'_, Message> {
        let t = self.tokens;
        if self.home_recent_artists_cache.is_empty() {
            return column![].into();
        }

        let mut cards = row![].spacing(12);
        for (id, name, cover_art_id) in self.home_recent_artists_cache.iter().take(5) {
            let (id, name, cover_art_id) = (id.clone(), name.clone(), cover_art_id.clone());
            cards = cards.push(
                button(
                    column![
                        self.cover_image(cover_art_id.as_deref(), 130.0),
                        text(name).size(12).style(tstyle(t.text)).font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::MONOSPACE
                        }),
                    ]
                    .spacing(6)
                    .width(Length::Fixed(130.0)),
                )
                .padding(4)
                .on_press(Message::Navigate(View::ArtistDetail(id)))
                .style(move |_th, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: if h { Some(Background::Color(t.surface)) } else { None },
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                }),
            );
        }

        column![
            text("RECENTLY PLAYED ARTISTS").size(11).style(tstyle(t.muted)),
            cards,
        ]
        .spacing(12)
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
            chips,
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
        for a in albums.iter().take(5) {
            cards = cards.push(self.album_card(a));
        }
        column![
            text(title).size(11).style(tstyle(t.muted)),
            cards,
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
            scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).into()
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
                scrollable(col).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t)).into()
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
                                    slider(-12.0..=12.0, b.gain, move |g| Message::EqBandChanged(i, g)).step(0.5).width(Length::Fill).style(slider_style(t)),
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
                        .width(Length::Fill)
                        .style(text_input_style(t)),
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

        container(column![header, scrollable(body).height(Length::Fill).direction(scrollable::Direction::Vertical(thin_scrollbar())).style(thin_scroll_style(t))].spacing(12))
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
        let title_text = song.map(|s| s.title.clone()).unwrap_or_else(|| "No track selected".to_string());
        let mut title_col = column![text(title_text).size(13).style(tstyle(t.text))].spacing(2);
        if let Some(s) = song {
            let subtitle = match &s.track_info {
                Some(info) if !info.is_empty() => format!("{} · {}", s.artist, info),
                _ => s.artist.clone(),
            };
            title_col = title_col.push(text(subtitle).size(11).style(tstyle(t.muted)));
        }
        let title_col = title_col.width(Length::Fill);
        let volume = row![
            icons::icon(icons::VOLUME, 16.0, t.muted),
            slider(0.0..=1.0, self.volume, Message::SetVolume).step(0.01).width(Length::Fixed(55.0)).style(slider_style(t)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        let left = container(row![cover, title_col, volume].spacing(10).align_y(Alignment::Center))
            .width(Length::Fixed(320.0));

        let dur = self.duration.unwrap_or(0.0).max(0.1) as f32;
        let pos = (self.position as f32).clamp(0.0, dur);
        let center = container(
            row![
                text(fmt_time(self.position)).size(11).style(tstyle(t.muted)),
                slider(0.0..=dur, pos, Message::SeekTo).step(0.5).width(Length::Fill).style(slider_style(t)),
                text(fmt_time(self.duration.unwrap_or(0.0))).size(11).style(tstyle(t.muted)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill);

        let playing = matches!(self.playback_state, PlaybackState::Playing);
        let pp_icon = if playing { icons::PAUSE } else { icons::PLAY };
        let repeat_color = if self.repeat_one || self.repeat_all { t.accent } else { t.muted };
        let shuffle_color = if self.shuffle { t.accent } else { t.muted };
        let viz_color = if self.right_panel == Some(Panel::Visualizer) { t.accent } else { t.muted };
        let lyr_color = if self.right_panel == Some(Panel::Lyrics) { t.accent } else { t.muted };
        let q_color = if self.right_panel == Some(Panel::Queue) { t.accent } else { t.muted };
        let sim_color = if self.right_panel == Some(Panel::Similar) { t.accent } else { t.muted };
        let stats_color = if self.right_panel == Some(Panel::AudioStats) { t.accent } else { t.muted };

        let controls = row![
            ctrl_button(icons::SHUFFLE, 15.0, shuffle_color, t, Message::ToggleShuffle),
            ctrl_button(icons::PREV, 15.0, t.text, t, Message::Prev),
            main_ctrl_button(pp_icon, 20.0, t, Message::TogglePlay),
            ctrl_button(icons::NEXT, 15.0, t.text, t, Message::Next),
            ctrl_button(icons::REPEAT, 16.0, repeat_color, t, Message::CycleRepeat),
            ctrl_button(icons::LYRICS, 16.0, lyr_color, t, Message::TogglePanel(Panel::Lyrics)),
            ctrl_button(icons::QUEUE, 16.0, q_color, t, Message::TogglePanel(Panel::Queue)),
            ctrl_button(icons::HEXAGON, 16.0, sim_color, t, Message::TogglePanel(Panel::Similar)),
            ctrl_button(icons::BAR_CHART, 16.0, stats_color, t, Message::TogglePanel(Panel::AudioStats)),
            ctrl_button(icons::WAVEFORM, 16.0, viz_color, t, Message::TogglePanel(Panel::Visualizer)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let right = container(controls).width(Length::Fixed(410.0));

        column![
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(fill_bg(t.border)),
            container(row![left, center, right].spacing(12).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fixed(60.0))
                .padding(iced::Padding { top: 8.0, right: 30.0, bottom: 8.0, left: 30.0 })
                .style(fill_bg(t.surface)),
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let bus = Subscription::run_with(BusSub(self.backend.bus.clone()), bus_stream);
        Subscription::batch([
            bus,
            if self.right_panel == Some(Panel::Visualizer) {
                iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::VisualizerTick)
            } else {
                Subscription::none()
            },
            if self.toasts.is_empty() {
                Subscription::none()
            } else {
                iced::time::every(std::time::Duration::from_millis(500)).map(|_| Message::ToastTick)
            },
        ])
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
    .align_y(Alignment::Center)
    .padding([15, 10])
    .into()
}

/// Settings category heading with a bottom separator line.
fn sett_panel_title<'a>(title: impl Into<String>, t: Tokens) -> Element<'a, Message> {
    column![
        container(
            text(title.into()).size(16).style(tstyle(t.text)).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            }),
        )
        .padding(iced::Padding { top: 16.0, right: 10.0, bottom: 12.0, left: 10.0 })
        .width(Length::Fill),
        container(text(""))
            .height(Length::Fixed(1.0))
            .width(Length::Fill)
            .style(fill_bg(t.border)),
    ]
    .spacing(0)
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
        toggler(on).on_toggle(on_toggle).style(toggler_style(t)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn icon_button<'a>(src: &'static str, size: f32, color: Color, t: Tokens, msg: Message) -> Element<'a, Message> {
    button(icons::icon(src, size, color))
        .padding(8)
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
        .padding(10)
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

/// Approximate row heights (cover/avatar + padding) and assumed viewport
/// height, used by the windowed lists.
const ALBUM_ROW_H: f32 = 60.0;
const ARTIST_ROW_H: f32 = 60.0;
const TRACK_ROW_H: f32 = 52.0;
const VIEWPORT_H: f32 = 640.0;

/// Max number of decoded cover-art image handles kept in memory at once.
const MAX_COVER_HANDLES: usize = 512;

/// Visible window for a virtualized list of `total` rows of height `row_h`,
/// given the current `scroll` offset. Returns the first/last visible indices
/// and the spacer heights that stand in for the off-screen rows so the
/// scrollbar stays correct for large lists.
fn list_window(total: usize, scroll: f32, row_h: f32) -> (usize, usize, f32, f32) {
    let first = ((scroll / row_h).floor().max(0.0) as usize).min(total);
    let count = (VIEWPORT_H / row_h).ceil() as usize + 4;
    let end = (first + count).min(total);
    let top = first as f32 * row_h;
    let bottom = total.saturating_sub(end) as f32 * row_h;
    (first, end, top, bottom)
}

fn rgb_to_color(c: crate::commands::cover_colors::Rgb) -> iced::Color {
    iced::Color::from_rgb8(c.r, c.g, c.b)
}

/// Build the 8-stop gradient LUT the visualizer shaders expect, smoothly
/// cycling `c0 -> c1 -> c2 -> c0` (the same 3-stop palette cycling the Android
/// visualizer uses).
fn ramp8(c0: iced::Color, c1: iced::Color, c2: iced::Color) -> Vec<iced::Color> {
    let lerp = |a: iced::Color, b: iced::Color, t: f32| iced::Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    };
    (0..8)
        .map(|i| {
            let t = i as f32 / 8.0;
            if t < 1.0 / 3.0 {
                lerp(c0, c1, t * 3.0)
            } else if t < 2.0 / 3.0 {
                lerp(c1, c2, (t - 1.0 / 3.0) * 3.0)
            } else {
                lerp(c2, c0, (t - 2.0 / 3.0) * 3.0)
            }
        })
        .collect()
}

fn text_input_style(t: Tokens) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme, status| {
        let focused = matches!(status, text_input::Status::Focused { .. });
        text_input::Style {
            background: Background::Color(t.bg),
            border: Border {
                color: if focused { t.accent } else { t.border },
                width: 1.0,
                radius: 2.0.into(),
            },
            icon: t.muted,
            placeholder: t.muted,
            value: t.text,
            selection: t.accent_dim,
        }
    }
}

fn slider_style(t: Tokens) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_theme, status| {
        let active = matches!(status, slider::Status::Hovered | slider::Status::Dragged);
        slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    Background::Color(t.accent),
                    Background::Color(t.surface2),
                ),
                width: 4.0,
                border: Border { radius: 10.0.into(), ..Border::default() },
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius: if active { 8.5 } else { 7.0 } },
                background: Background::Color(t.accent),
                border_width: if active { 1.0 } else { 0.0 },
                border_color: if active { t.bg } else { Color::TRANSPARENT },
            },
        }
    }
}

fn thin_scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new().width(10).margin(2).scroller_width(5)
}

fn thin_scroll_style(t: Tokens) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |_, _| {
        let rail = scrollable::Rail {
            background: Some(Background::Color(Color { a: 0.08, ..t.muted })),
            border: Border { radius: 3.0.into(), ..Border::default() },
            scroller: scrollable::Scroller {
                background: Background::Color(Color { a: 0.55, ..t.muted }),
                border: Border { radius: 3.0.into(), ..Border::default() },
            },
        };
        scrollable::Style {
            container: container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Background::Color(t.surface),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: t.muted,
            },
        }
    }
}

fn toggler_style(t: Tokens) -> impl Fn(&Theme, toggler::Status) -> toggler::Style {
    move |_theme, status| {
        let on = matches!(
            status,
            toggler::Status::Active { is_toggled: true } | toggler::Status::Hovered { is_toggled: true }
        );
        toggler::Style {
            background: if on { Background::Color(t.accent) } else { Background::Color(t.surface2) },
            background_border_width: if on { 0.0 } else { 1.0 },
            background_border_color: if on { t.accent } else { t.border },
            foreground: if on { Background::Color(t.bg) } else { Background::Color(t.muted) },
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: None,
            border_radius: None,
            padding_ratio: 0.15,
        }
    }
}

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

/// Decode a cover art file and bake rounded corners into the RGBA pixels.
/// The corner radius is scaled proportionally so that images displayed at
/// 130 px get ~28 px visual radius regardless of the source file's native size.
fn load_rounded_cover(path: &str) -> ImageHandle {
    let img = match image::open(path) {
        Ok(i) => i.into_rgba8(),
        Err(_) => return ImageHandle::from_path(path),
    };
    let w = img.width();
    let h = img.height();
    let r = (14.0 * (w.min(h) as f32 / 130.0))
        .min(w as f32 / 2.0)
        .min(h as f32 / 2.0);
    let mut pixels = img.into_raw();
    for y in 0..h {
        for x in 0..w {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            let wf = w as f32;
            let hf = h as f32;
            let outside = if xf < r && yf < r {
                let (dx, dy) = (r - xf, r - yf);
                dx * dx + dy * dy > r * r
            } else if xf > wf - r && yf < r {
                let (dx, dy) = (xf - (wf - r), r - yf);
                dx * dx + dy * dy > r * r
            } else if xf < r && yf > hf - r {
                let (dx, dy) = (r - xf, yf - (hf - r));
                dx * dx + dy * dy > r * r
            } else if xf > wf - r && yf > hf - r {
                let (dx, dy) = (xf - (wf - r), yf - (hf - r));
                dx * dx + dy * dy > r * r
            } else {
                false
            };
            if outside {
                pixels[((y * w + x) * 4 + 3) as usize] = 0;
            }
        }
    }
    ImageHandle::from_rgba(w, h, pixels)
}
