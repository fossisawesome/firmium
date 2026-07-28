use firmium_backend::{commands::mappers::Album, errors::UserError};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum View {
    Home,
    Favorites,
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
pub(crate) enum PlaylistListItem {
    Local(usize),
    ServerOnly(usize),
}

impl View {
    #[allow(dead_code)]
    pub(crate) fn title(&self) -> &'static str {
        match self {
            View::Home => "Home",
            View::Favorites => "Favorites",
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
pub(crate) enum RecapRange {
    Week,
    Month,
    ThreeMonths,
    Year,
    All,
}

impl RecapRange {
    /// Inclusive lower bound (unix seconds) for this window, given `now`.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn from_ts(self, now: i64) -> i64 {
        let day = 86_400;
        match self {
            RecapRange::Week => now - 7 * day,
            RecapRange::Month => now - 30 * day,
            RecapRange::ThreeMonths => now - 90 * day,
            RecapRange::Year => now - 365 * day,
            RecapRange::All => 0,
        }
    }

    pub(crate) fn label(self) -> &'static str {
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
pub(crate) const RECAP_CARDS: usize = 9;

/// Which collapsible right-side panel is open (mutually exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Visualizer,
    Queue,
    Lyrics,
    Equalizer,
    AudioStats,
    Similar,
}

/// Which Settings category is selected in the two-column settings layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsCategory {
    Appearance,
    Playback,
    Visualizer,
    Equalizer,
    Downloads,
    Services,
    Account,
    Debug,
}

/// Home page album sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HomeSection {
    Recent,
    Newest,
    Random,
}

/// Mood Mix energy band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Energy {
    Chill,
    Mid,
    High,
}

#[derive(Debug, Clone)]
pub(crate) struct Toast {
    pub id: u64,
    pub category: UserError,
    pub text: String,
    pub spawned: std::time::Instant,
}

/// Album sort options, referenced from Subsonic API getAlbumList2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlbumsSort {
    Random,
    Newest,
    Frequent,
    Recent,
    Starred,
    AlphabeticalByName,
    AlphabeticalByArtist,
}

impl AlbumsSort {
    pub(crate) fn get_sort_param(self) -> &'static str {
        match self {
            AlbumsSort::Random => "random",
            AlbumsSort::Newest => "newest",
            AlbumsSort::Frequent => "frequent",
            AlbumsSort::Recent => "recent",
            AlbumsSort::Starred => "starred",
            AlbumsSort::AlphabeticalByName => "alphabeticalByName",
            AlbumsSort::AlphabeticalByArtist => "alphabeticalByArtist",
        }
    }
}
