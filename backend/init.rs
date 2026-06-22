//! Backend bootstrap. Replaces the old Tauri `run()` setup: builds the shared
//! handles (event bus, audio player, app/queue state, play history) and starts
//! the queue-manager task. The iced `App` holds the returned `Backend`.

use std::sync::Arc;

use crate::audio::AudioPlayer;
use crate::db::PlayHistory;
use crate::events::EventBus;
use crate::queue_state::QueueState;
use crate::state::AppState;

/// All shared backend handles, constructed once at startup and held by the App.
pub struct Backend {
    pub bus: EventBus,
    pub audio_player: Arc<AudioPlayer>,
    pub app_state: Arc<AppState>,
    pub queue_state: Arc<QueueState>,
    /// `None` if the play-history DB failed to initialize (stats features no-op).
    pub history: Option<Arc<PlayHistory>>,
}

impl Backend {
    /// Build the backend. Must be called from within a Tokio runtime context —
    /// `AudioPlayer::new` and `queue_manager::start` spawn background tasks.
    pub fn new() -> Result<Self, String> {
        let bus = EventBus::new();
        let audio_player = Arc::new(AudioPlayer::new(bus.clone())?);
        let app_state = Arc::new(AppState::new(bus.clone()));
        let queue_state = Arc::new(QueueState::new());

        // A failure here (e.g. unwritable data dir) must not crash the app.
        let history = match PlayHistory::new() {
            Ok(h) => Some(Arc::new(h)),
            Err(e) => {
                eprintln!("Play history DB init failed: {e}");
                None
            }
        };

        crate::queue_manager::start(
            bus.clone(),
            Arc::clone(&queue_state),
            Arc::clone(&app_state),
            Arc::clone(&audio_player),
            history.clone(),
        );

        Ok(Backend { bus, audio_player, app_state, queue_state, history })
    }
}
