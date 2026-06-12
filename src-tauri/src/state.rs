// ============================================================================
// CONNECTION STATE
// ============================================================================
// Holds the active OpenSubsonic server connection (set via `set_connection`)
// and a shared async HTTP client for `commands/subsonic.rs`. Separate from
// `audio.rs`'s blocking client, which is dedicated to the playback thread.

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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connection: RwLock::new(ConnectionState::default()),
            http: reqwest::Client::builder()
                .user_agent("Firmium")
                .build()
                .expect("failed to build reqwest client"),
        }
    }
}
