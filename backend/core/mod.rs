//! App-level logic shared by UI front ends: settings, accounts, local-first
//! playlists. Front ends (the Tauri shell) only forward user intent here and
//! render the state it returns; nothing in this module touches a UI toolkit.

pub mod accounts;
pub mod library;
pub mod playlists;
pub mod settings;
pub mod stats;

use std::sync::Arc;

use parking_lot::Mutex;

use crate::config::Config;
use crate::init::Backend;

pub use accounts::AuthState;
pub use settings::Settings;

/// Everything the shell holds: the shared backend handles plus persisted
/// config and the local playlist list.
pub struct Core {
    pub backend: Backend,
    pub config: Mutex<Config>,
    pub playlists: Mutex<Vec<playlists::Playlist>>,
    /// Artist photo URLs by artist id (`None` = looked up, none available).
    pub(crate) artist_images: Mutex<std::collections::HashMap<String, Option<String>>>,
}

impl Core {
    /// Must run inside a Tokio runtime (`Backend::new` spawns tasks).
    pub fn new() -> Result<Arc<Self>, String> {
        let backend = Backend::new()?;
        let config = Config::load();
        let core = Arc::new(Core {
            backend,
            config: Mutex::new(config),
            playlists: Mutex::new(playlists::load()),
            artist_images: Mutex::new(std::collections::HashMap::new()),
        });
        core.apply_persisted_playback();
        core.start_auto_continue();
        Ok(core)
    }

    pub(crate) fn save_config(&self) {
        self.config.lock().save();
    }
}
