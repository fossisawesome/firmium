mod cover;
mod export;
mod format;
mod message;
mod styles;
mod subscription;
mod types;
mod update;
mod view;
mod viz_colors;

use std::collections::HashMap;
use std::collections::VecDeque;

use iced::widget::image::Handle as ImageHandle;
use iced::widget::{button, column, container, row, stack, text};
use iced::{Alignment, Background, Border, Element, Length, Task, Theme};

use crate::commands::equalizer::EqState;
use crate::commands::lyrics::LyricsResult;
use crate::commands::mappers::{Album, Artist, SimilarMatch, Song};
use crate::podcasts::{PodcastChannel, PodcastEpisode};
use crate::playlists::Playlist;
use crate::commands::subsonic::{AlbumTracks, ArtistDetails, ArtistInfo, Genre, PlaylistTracks, RemotePlayQueue, SearchResult};
use crate::commands::themes::ThemeEntry;
use crate::config::{Config, SavedAccount};
use crate::errors::UserError;
use crate::init::Backend;
use crate::theme::Tokens;
use crate::viz::VizMode;
use crate::{icons, PlaybackState};

pub use message::Message;
use styles::*;
use types::{Panel, PlaylistListItem, RecapRange, SettingsCategory, Toast, View};

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
    font_family: String,

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
            font_family: cfg.font_family.clone().unwrap_or_else(|| "Liberation Mono".to_string()),
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

    pub(crate) fn save_config(&self) {
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
            font_family: Some(self.font_family.clone()),
        }
        .save();
    }

    /// Upsert the active connection into the saved-accounts list.
    pub(crate) fn remember_current_account(&mut self) {
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

    pub(crate) fn populate_offline_library(&mut self) {
        self.albums = crate::commands::local_library::get_local_albums(&self.backend.app_state).unwrap_or_default();
        self.artists = crate::commands::local_library::get_local_artists(&self.backend.app_state).unwrap_or_default();
        self.home_recent = crate::commands::local_library::get_local_recent_albums(&self.backend.app_state, 12).unwrap_or_default();
        self.home_newest = crate::commands::local_library::get_local_newest_albums(&self.backend.app_state, 12).unwrap_or_default();
        self.home_random = crate::commands::local_library::get_local_random_albums(&self.backend.app_state, 12).unwrap_or_default();
    }

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

    pub(crate) fn show_toast(&mut self, err: UserError) {
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

    pub(crate) fn toast_host(&self) -> Element<'_, Message> {
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

    pub(crate) fn shell(&self) -> Element<'_, Message> {
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
}

impl App {
    pub(crate) fn nav_button(&self, icon_src: &'static str, label: &'static str, target: View) -> Element<'_, Message> {
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
}
