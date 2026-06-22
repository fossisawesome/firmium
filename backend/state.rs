// ============================================================================
// CONNECTION STATE
// ============================================================================
// Holds the active OpenSubsonic server connection (set via `set_connection`)
// and a shared async HTTP client for `commands/subsonic.rs`. Separate from
// `audio.rs`'s blocking client, which is dedicated to the playback thread.

use crate::commands::local_library::LocalLibraryCache;
use crate::events::EventBus;
use parking_lot::RwLock;

#[derive(Default)]
pub struct ConnectionState {
    pub server: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub open_subsonic_extensions: Option<Vec<String>>,
}

pub struct AppState {
    pub connection: RwLock<ConnectionState>,
    pub http: reqwest::Client,
    /// Cached scan of the local library folder (`~/Music/Firmium`). `None` until
    /// the first scan; invalidated after downloads/imports so the next read rescans.
    pub local_library: RwLock<Option<LocalLibraryCache>>,
    /// Event bus — the OpenSubsonic client emits `SessionExpired` here on HTTP
    /// 401 / API error 40/41 (replaces the old `firmium:session-expired` Tauri event).
    pub bus: EventBus,
}

impl AppState {
    pub fn new(bus: EventBus) -> Self {
        Self {
            connection: RwLock::new(ConnectionState::default()),
            http: reqwest::Client::builder()
                .user_agent("Firmium")
                .build()
                .expect("failed to build reqwest client"),
            local_library: RwLock::new(None),
            bus,
        }
    }
}
