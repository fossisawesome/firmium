use crate::commands::lyrics::LyricsResult;
use crate::commands::mappers::{Album, Artist, SimilarMatch, Song};
use crate::commands::subsonic::{AlbumTracks, ArtistDetails, ArtistInfo, Genre, PlaylistTracks, RemotePlayQueue, SearchResult};
use crate::config::SavedAccount;
use crate::errors::UserError;
use crate::events::BackendEvent;
use crate::podcasts::{PodcastChannel, PodcastEpisode};
use crate::viz::VizMode;
use crate::viz::config::{
    BarsGradientMode, BarsGradientOrientation, BarsPeakGradientMode, BarsPeakMode, GradientMode,
    LineStyle,
};

use super::types::{Energy, HomeSection, Panel, RecapRange, SettingsCategory, View};

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
    SelectUiTheme(String),
    SelectFont(String),
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
    SetScrollbarWidth(u32),
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

    // ── Visualizer: Bars ──────────────────────────────────────────────────────
    SetBarsMonstercat(f32),
    SetBarsWaves(bool),
    SetBarsWavesSmoothing(u32),
    SetBarsGradientMode(BarsGradientMode),
    SetBarsGradientOrientation(BarsGradientOrientation),
    SetBarsPeakGradientMode(BarsPeakGradientMode),
    SetBarsPeakMode(BarsPeakMode),
    SetBarsPeakHoldTime(f32),
    SetBarsPeakFadeTime(f32),
    SetBarsPeakHeight(f32),
    SetBarsBorderWidth(f32),
    SetBarsLedBars(bool),
    SetBarsLedSegmentHeight(f32),
    SetBarsDepth3d(f32),
    SetBarsFlashIntensity(f32),
    SetBarsMaxBars(u32),
    SetBarsTrails(f32),
    SetBarsEcho(f32),

    // ── Visualizer: Lines ─────────────────────────────────────────────────────
    SetLinesPointCount(u32),
    SetLinesLineThickness(f32),
    SetLinesOutlineThickness(f32),
    SetLinesOutlineOpacity(f32),
    SetLinesAnimationSpeed(f32),
    SetLinesGradientMode(GradientMode),
    SetLinesFillOpacity(f32),
    SetLinesGlowIntensity(f32),
    SetLinesMirror(bool),
    SetLinesStyle(LineStyle),
    SetLinesTrails(f32),
    SetLinesEcho(f32),

    // ── Visualizer: Scope ─────────────────────────────────────────────────────
    SetScopeRadius(f32),
    SetScopeSensitivity(f32),
    SetScopePointCount(u32),
    SetScopeLineThickness(f32),
    SetScopeFillOpacity(f32),
    SetScopeGlowIntensity(f32),
    SetScopeOutlineThickness(f32),
    SetScopeOutlineOpacity(f32),
    SetScopeGradientMode(GradientMode),
    SetScopeAnimationSpeed(f32),
    SetScopeStyle(LineStyle),
    SetScopeParticles(bool),
    SetScopeParticleCount(u32),
    SetScopeParticleSpeed(f32),
    SetScopeBeam(bool),
    SetScopeTrails(f32),
    SetScopeEcho(f32),
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
