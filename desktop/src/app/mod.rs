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
use iced::widget::{button, column, container, row, scrollable, stack, text};
use iced::{Alignment, Background, Border, Element, Length, Task, Theme};

use firmium_backend::commands::equalizer::EqState;
use firmium_backend::commands::lyrics::LyricsResult;
use firmium_backend::commands::mappers::{Album, Artist, SimilarMatch, Song};
use firmium_backend::podcasts::{PodcastChannel, PodcastEpisode};
use crate::playlists::Playlist;
use firmium_backend::commands::subsonic::{AlbumTracks, ArtistDetails, ArtistInfo, Genre, PlaylistTracks, RemotePlayQueue, SearchResult};
use firmium_backend::commands::themes::ThemeEntry;
use firmium_backend::config::{Config, SavedAccount};
use firmium_backend::errors::UserError;
use firmium_backend::init::Backend;
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
    // Layout/structure axis, independent of `theme_id` (colors): "default" or "spotify".
    ui_theme_id: String,

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
    forward_stack: Vec<View>,

    // ── Library data ──────────────────────────────────────────────────────────
    albums: Vec<Album>,
    albums_scroll: f32,
    home_recent: Vec<Album>,
    home_newest: Vec<Album>,
    home_random: Vec<Album>,
    home_recent_plays: Vec<firmium_backend::db::RecentPlay>,
    // Deduplicated (artist_id, name, cover_art_id) derived from home_recent_plays;
    // recomputed only when home_recent_plays changes so view() doesn't rebuild it per frame.
    home_recent_artists_cache: Vec<(String, String, Option<String>)>,
    album_detail: Option<AlbumTracks>,
    album_detail_id: Option<String>,
    favorites: Option<firmium_backend::commands::mappers::Starred>,
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

    // ── Visualizer: Bars ──────────────────────────────────────────────────────
    bars_monstercat: f32,
    bars_waves: bool,
    bars_waves_smoothing: u32,
    bars_gradient_mode: crate::viz::config::BarsGradientMode,
    bars_gradient_orientation: crate::viz::config::BarsGradientOrientation,
    bars_peak_gradient_mode: crate::viz::config::BarsPeakGradientMode,
    bars_peak_mode: crate::viz::config::BarsPeakMode,
    bars_peak_hold_time: f32,
    bars_peak_fade_time: f32,
    bars_peak_height: f32,
    bars_border_width: f32,
    bars_led_bars: bool,
    bars_led_segment_height: f32,
    bars_depth_3d: f32,
    bars_flash_intensity: f32,
    bars_max_bars: u32,
    bars_trails: f32,
    bars_echo: f32,

    // ── Visualizer: Lines ─────────────────────────────────────────────────────
    lines_point_count: u32,
    lines_line_thickness: f32,
    lines_outline_thickness: f32,
    lines_outline_opacity: f32,
    lines_animation_speed: f32,
    lines_gradient_mode: crate::viz::config::GradientMode,
    lines_fill_opacity: f32,
    lines_glow_intensity: f32,
    lines_mirror: bool,
    lines_style: crate::viz::config::LineStyle,
    lines_trails: f32,
    lines_echo: f32,

    // ── Visualizer: Scope ─────────────────────────────────────────────────────
    scope_radius: f32,
    scope_sensitivity: f32,
    scope_point_count: u32,
    scope_line_thickness: f32,
    scope_fill_opacity: f32,
    scope_glow_intensity: f32,
    scope_outline_thickness: f32,
    scope_outline_opacity: f32,
    scope_gradient_mode: crate::viz::config::GradientMode,
    scope_animation_speed: f32,
    scope_style: crate::viz::config::LineStyle,
    scope_particles: bool,
    scope_particle_count: u32,
    scope_particle_speed: f32,
    scope_beam: bool,
    scope_trails: f32,
    scope_echo: f32,

    viz_palette: Option<firmium_backend::commands::cover_colors::OrbPalette>,
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
    scrollbar_width: u32,

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
    recap: Option<firmium_backend::db::RecapStats>,
    recap_range: RecapRange,
    recap_card: usize,

    // ── Listening stats ───────────────────────────────────────────────────────
    history_summary: Option<firmium_backend::db::PlayHistorySummary>,

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
    pub(crate) fn make_scrollbar(&self) -> scrollable::Scrollbar {
        scrollbar_width(self.scrollbar_width)
    }

    pub fn new(backend: Backend) -> Self {
        let cfg = Config::load();
        let themes = firmium_backend::commands::themes::list_themes();
        let theme_id = cfg.theme_id.clone().unwrap_or_else(|| "firmium".to_string());
        let tokens = themes
            .iter()
            .find(|t| t.id == theme_id)
            .map(Tokens::from_entry)
            .unwrap_or_default();
        let volume = cfg.volume.unwrap_or(0.8).clamp(0.0, 1.0);
        let vdef = crate::viz::config::VizConfig::default();

        // Push persisted playback settings into the backend queue state.
        firmium_backend::commands::queue::init_playback_settings(
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
            ui_theme_id: cfg.ui_theme_id.clone().unwrap_or_else(|| "default".to_string()),
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
            forward_stack: Vec::new(),
            albums: Vec::new(),
            albums_scroll: 0.0,
            home_recent: Vec::new(),
            home_newest: Vec::new(),
            home_random: Vec::new(),
            home_recent_plays: Vec::new(),
            home_recent_artists_cache: Vec::new(),
            album_detail: None,
            album_detail_id: None,
            favorites: None,
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

            bars_monstercat: cfg.bars_monstercat.unwrap_or(1.0),
            bars_waves: cfg.bars_waves.unwrap_or(false),
            bars_waves_smoothing: cfg.bars_waves_smoothing.unwrap_or(5),
            bars_gradient_mode: cfg
                .bars_gradient_mode
                .unwrap_or(crate::viz::config::BarsGradientMode::Static),
            bars_gradient_orientation: cfg
                .bars_gradient_orientation
                .unwrap_or(crate::viz::config::BarsGradientOrientation::Vertical),
            bars_peak_gradient_mode: cfg
                .bars_peak_gradient_mode
                .unwrap_or(crate::viz::config::BarsPeakGradientMode::Static),
            bars_peak_mode: cfg.bars_peak_mode.unwrap_or(crate::viz::config::BarsPeakMode::Fall),
            bars_peak_hold_time: cfg.bars_peak_hold_time.unwrap_or(vdef.peak_hold_time),
            bars_peak_fade_time: cfg.bars_peak_fade_time.unwrap_or(vdef.peak_fade_time),
            bars_peak_height: cfg.bars_peak_height.unwrap_or(vdef.peak_thickness),
            bars_border_width: cfg.bars_border_width.unwrap_or(vdef.border_width),
            bars_led_bars: cfg.bars_led_bars.unwrap_or(vdef.led_bars),
            bars_led_segment_height: cfg
                .bars_led_segment_height
                .unwrap_or(vdef.led_segment_height),
            bars_depth_3d: cfg.bars_depth_3d.unwrap_or(vdef.bar_depth_3d),
            bars_flash_intensity: cfg.bars_flash_intensity.unwrap_or(vdef.bars_flash_intensity),
            bars_max_bars: cfg.bars_max_bars.unwrap_or(vdef.bars_max_bars),
            bars_trails: cfg.bars_trails.unwrap_or(vdef.bars_trails),
            bars_echo: cfg.bars_echo.unwrap_or(vdef.bars_echo),

            lines_point_count: cfg.lines_point_count.unwrap_or(vdef.lines_point_count),
            lines_line_thickness: cfg.lines_line_thickness.unwrap_or(vdef.line_thickness),
            lines_outline_thickness: cfg
                .lines_outline_thickness
                .unwrap_or(vdef.lines_outline_thickness),
            lines_outline_opacity: cfg
                .lines_outline_opacity
                .unwrap_or(vdef.lines_outline_opacity),
            lines_animation_speed: cfg
                .lines_animation_speed
                .unwrap_or(vdef.lines_animation_speed),
            lines_gradient_mode: cfg
                .lines_gradient_mode
                .unwrap_or(crate::viz::config::GradientMode::Static),
            lines_fill_opacity: cfg.lines_fill_opacity.unwrap_or(vdef.lines_fill_opacity),
            lines_glow_intensity: cfg.lines_glow_intensity.unwrap_or(vdef.lines_glow_intensity),
            lines_mirror: cfg.lines_mirror.unwrap_or(vdef.lines_mirror),
            lines_style: cfg.lines_style.unwrap_or(crate::viz::config::LineStyle::Smooth),
            lines_trails: cfg.lines_trails.unwrap_or(vdef.lines_trails),
            lines_echo: cfg.lines_echo.unwrap_or(vdef.lines_echo),

            scope_radius: cfg.scope_radius.unwrap_or(vdef.scope_radius),
            scope_sensitivity: cfg.scope_sensitivity.unwrap_or(vdef.scope_sensitivity),
            scope_point_count: cfg.scope_point_count.unwrap_or(vdef.scope_point_count),
            scope_line_thickness: cfg.scope_line_thickness.unwrap_or(vdef.scope_line_thickness),
            scope_fill_opacity: cfg.scope_fill_opacity.unwrap_or(vdef.scope_fill_opacity),
            scope_glow_intensity: cfg.scope_glow_intensity.unwrap_or(vdef.scope_glow_intensity),
            scope_outline_thickness: cfg
                .scope_outline_thickness
                .unwrap_or(vdef.scope_outline_thickness),
            scope_outline_opacity: cfg
                .scope_outline_opacity
                .unwrap_or(vdef.scope_outline_opacity),
            scope_gradient_mode: cfg
                .scope_gradient_mode
                .unwrap_or(crate::viz::config::GradientMode::Static),
            scope_animation_speed: cfg
                .scope_animation_speed
                .unwrap_or(vdef.scope_animation_speed),
            scope_style: cfg.scope_style.unwrap_or(crate::viz::config::LineStyle::Smooth),
            scope_particles: cfg.scope_particles.unwrap_or(vdef.scope_particles),
            scope_particle_count: cfg.scope_particle_count.unwrap_or(vdef.scope_particle_count),
            scope_particle_speed: cfg.scope_particle_speed.unwrap_or(vdef.scope_particle_speed),
            scope_beam: cfg.scope_beam.unwrap_or(vdef.scope_beam),
            scope_trails: cfg.scope_trails.unwrap_or(vdef.scope_trails),
            scope_echo: cfg.scope_echo.unwrap_or(vdef.scope_echo),

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
            scrollbar_width: cfg.scrollbar_width.unwrap_or(10).clamp(6, 20),
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
        let backend_viz = app.backend.audio_player.visualizer();
        if app.bars_waves {
            backend_viz.set_waves(true, app.bars_waves_smoothing);
        } else {
            backend_viz.set_monstercat(app.bars_monstercat);
        }
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
                        firmium_backend::commands::credentials::get_password(Some("firmium-desktop"), key)
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
                        firmium_backend::commands::credentials::get_password(Some(&server), &user).ok()
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
            ui_theme_id: Some(self.ui_theme_id.clone()),
            volume: Some(self.volume),
            accounts: self.accounts.clone(),
            download_format: Some(self.download_format.clone()),
            lrclib_enabled: Some(self.lrclib_enabled),
            lyrics_word_fill: Some(self.lyrics_word_fill),
            window_decorations: Some(self.window_decorations),
            viz_cover_colors: Some(self.viz_cover_colors),
            font_family: Some(self.font_family.clone()),
            scrollbar_width: Some(self.scrollbar_width),

            bars_monstercat: Some(self.bars_monstercat),
            bars_waves: Some(self.bars_waves),
            bars_waves_smoothing: Some(self.bars_waves_smoothing),
            bars_gradient_mode: Some(self.bars_gradient_mode),
            bars_gradient_orientation: Some(self.bars_gradient_orientation),
            bars_peak_gradient_mode: Some(self.bars_peak_gradient_mode),
            bars_peak_mode: Some(self.bars_peak_mode),
            bars_peak_hold_time: Some(self.bars_peak_hold_time),
            bars_peak_fade_time: Some(self.bars_peak_fade_time),
            bars_peak_height: Some(self.bars_peak_height),
            bars_border_width: Some(self.bars_border_width),
            bars_led_bars: Some(self.bars_led_bars),
            bars_led_segment_height: Some(self.bars_led_segment_height),
            bars_depth_3d: Some(self.bars_depth_3d),
            bars_flash_intensity: Some(self.bars_flash_intensity),
            bars_max_bars: Some(self.bars_max_bars),
            bars_trails: Some(self.bars_trails),
            bars_echo: Some(self.bars_echo),

            lines_point_count: Some(self.lines_point_count),
            lines_line_thickness: Some(self.lines_line_thickness),
            lines_outline_thickness: Some(self.lines_outline_thickness),
            lines_outline_opacity: Some(self.lines_outline_opacity),
            lines_animation_speed: Some(self.lines_animation_speed),
            lines_gradient_mode: Some(self.lines_gradient_mode),
            lines_fill_opacity: Some(self.lines_fill_opacity),
            lines_glow_intensity: Some(self.lines_glow_intensity),
            lines_mirror: Some(self.lines_mirror),
            lines_style: Some(self.lines_style),
            lines_trails: Some(self.lines_trails),
            lines_echo: Some(self.lines_echo),

            scope_radius: Some(self.scope_radius),
            scope_sensitivity: Some(self.scope_sensitivity),
            scope_point_count: Some(self.scope_point_count),
            scope_line_thickness: Some(self.scope_line_thickness),
            scope_fill_opacity: Some(self.scope_fill_opacity),
            scope_glow_intensity: Some(self.scope_glow_intensity),
            scope_outline_thickness: Some(self.scope_outline_thickness),
            scope_outline_opacity: Some(self.scope_outline_opacity),
            scope_gradient_mode: Some(self.scope_gradient_mode),
            scope_animation_speed: Some(self.scope_animation_speed),
            scope_style: Some(self.scope_style),
            scope_particles: Some(self.scope_particles),
            scope_particle_count: Some(self.scope_particle_count),
            scope_particle_speed: Some(self.scope_particle_speed),
            scope_beam: Some(self.scope_beam),
            scope_trails: Some(self.scope_trails),
            scope_echo: Some(self.scope_echo),
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
        self.albums = firmium_backend::commands::local_library::get_local_albums(&self.backend.app_state).unwrap_or_default();
        self.artists = firmium_backend::commands::local_library::get_local_artists(&self.backend.app_state).unwrap_or_default();
        self.home_recent = firmium_backend::commands::local_library::get_local_recent_albums(&self.backend.app_state, 12).unwrap_or_default();
        self.home_newest = firmium_backend::commands::local_library::get_local_newest_albums(&self.backend.app_state, 12).unwrap_or_default();
        self.home_random = firmium_backend::commands::local_library::get_local_random_albums(&self.backend.app_state, 12).unwrap_or_default();
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
        let spotify = self.ui_theme_id == "spotify";
        let cards: Vec<Element<Message>> = self.toasts.iter().map(|toast| {
            let close = icon_button(icons::CLOSE, 14.0, t.muted, t, spotify, Message::DismissToast(toast.id));
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

        let sidebar = if self.ui_theme_id == "spotify" {
            self.sidebar_spotify()
        } else {
            self.sidebar_default()
        };

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
    pub(crate) fn sidebar_default(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";

        let brand = container(
            row![
                icon_button(icons::USER, 16.0, t.muted, t, spotify, Message::ToggleAccountSwitcher),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(20);

        let nav = column![
            self.nav_button(icons::HOME, "Home", View::Home),
            self.nav_button(icons::HEART_OUTLINE, "Favorites", View::Favorites),
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

        container(column![brand, nav])
            .width(Length::Fixed(220.0))
            .height(Length::Fill)
            .style(fill_bg(t.surface))
            .into()
    }

    /// Spotify-style sidebar: a small top card (account + fixed nav) and a separate,
    /// taller "Your Library" card below it, both inset from a pure-black shell —
    /// replacing the default's single flat nav column.
    pub(crate) fn sidebar_spotify(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";

        let card_style = move |_theme: &Theme| container::Style {
            background: Some(Background::Color(t.surface)),
            border: Border { radius: 8.0.into(), ..Border::default() },
            ..container::Style::default()
        };

        let brand = container(
            row![icon_button(icons::USER, 16.0, t.muted, t, spotify, Message::ToggleAccountSwitcher)]
                .spacing(10)
                .align_y(Alignment::Center),
        )
        .padding(iced::Padding { top: 12.0, right: 12.0, bottom: 0.0, left: 12.0 });

        let fixed_nav = column![
            self.nav_button(icons::HOME, "Home", View::Home),
            self.nav_button(icons::HEART_OUTLINE, "Favorites", View::Favorites),
            self.nav_button(icons::SEARCH, "Search", View::Search),
            self.nav_button(icons::DISC, "Albums", View::Albums),
            self.nav_button(icons::USER, "Artists", View::Artists),
            self.nav_button(icons::PODCAST, "Podcasts", View::Podcasts),
            self.nav_button(icons::MUSIC, "Mix", View::Mix),
            self.nav_button(icons::SETTINGS, "Settings", View::Settings),
        ]
        .spacing(4)
        .padding(iced::Padding { top: 4.0, right: 8.0, bottom: 12.0, left: 8.0 });

        let top_card = container(column![brand, fixed_nav].spacing(0))
            .width(Length::Fill)
            .style(card_style);

        let library_header = container(
            row![
                icons::icon(icons::LIST, 16.0, t.text),
                text("Your Library").size(13).style(tstyle(t.text)).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::with_name("Inter")
                }),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(iced::Padding { top: 14.0, right: 12.0, bottom: 8.0, left: 12.0 });

        let filter_pill = move |label: &'static str, active: bool| -> Element<'static, Message> {
            container(text(label).size(12).style(tstyle(if active { t.bg } else { t.text })))
                .padding([6, 14])
                .style(move |_theme: &Theme| container::Style {
                    background: Some(Background::Color(if active { t.text } else { t.surface2 })),
                    border: Border { radius: 500.0.into(), ..Border::default() },
                    ..container::Style::default()
                })
                .into()
        };
        let filters = row![filter_pill("Playlists", true), filter_pill("Artists", false)]
            .spacing(8)
            .padding(iced::Padding { top: 0.0, right: 12.0, bottom: 8.0, left: 12.0 });

        let mut lib_list = column![].spacing(2).padding([4, 8]);
        for item in &self.playlist_items {
            lib_list = lib_list.push(self.spotify_library_row(item));
        }
        let library_scroll = scrollable(lib_list)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(self.make_scrollbar()))
            .style(thin_scroll_style(t));

        let library_card = container(column![library_header, filters, library_scroll].spacing(0))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(card_style);

        container(column![top_card, library_card].spacing(8).padding(8))
            .width(Length::Fixed(260.0))
            .height(Length::Fill)
            .style(fill_bg(t.bg))
            .into()
    }

    /// A single playlist row in the Spotify sidebar's library list: small square
    /// cover-art thumbnail + name (no drag handles — this is a compact nav
    /// shortcut, not the full playlist-management row used on the Playlists screen).
    pub(crate) fn spotify_library_row(&self, item: &PlaylistListItem) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let (nav_id, name, cover_id): (String, String, Option<String>) = match item {
            PlaylistListItem::Local(i) => {
                let p = &self.playlists[*i];
                let cover = p.tracks.iter().find_map(|s| s.cover_art_id.clone());
                (p.id.clone(), p.name.clone(), cover)
            }
            PlaylistListItem::ServerOnly(i) => {
                let sp = &self.server_playlists[*i];
                let sid = sp.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let nm = sp.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
                let cover = sp.get("coverArt").and_then(|v| v.as_str()).map(|s| s.to_string());
                (format!("server-{sid}"), nm, cover)
            }
        };
        button(
            row![self.cover_image(cover_id.as_deref(), 40.0), text(name).size(13).style(tstyle(t.text))]
                .spacing(10)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([6, 8])
        .on_press(Message::Navigate(View::PlaylistDetail(nav_id)))
        .style(list_row_style(t, spotify))
        .into()
    }

    pub(crate) fn nav_button(&self, icon_src: &'static str, label: &'static str, target: View) -> Element<'_, Message> {
        let active = self.view == target;
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let color = if active {
            if spotify { t.text } else { t.accent }
        } else {
            t.muted
        };
        let mut label_text = text(label).size(13).style(tstyle(color));
        if active && !spotify {
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
