// Backend modules live under ../backend/ but are declared here at the crate
// root via #[path] so every existing `crate::...` reference inside them keeps
// resolving unchanged (no per-file path rewrites needed during the migration).
#[path = "../backend/types.rs"]
mod types;
pub use types::{AudioDevice, PlaybackState};

#[path = "../backend/events.rs"]
mod events;
#[path = "../backend/paths.rs"]
mod paths;
#[path = "../backend/state.rs"]
mod state;
#[path = "../backend/visualizer.rs"]
mod visualizer;
#[path = "../backend/audio/mod.rs"]
mod audio;
#[path = "../backend/db.rs"]
mod db;
#[path = "../backend/queue_state.rs"]
mod queue_state;
#[path = "../backend/queue_manager.rs"]
mod queue_manager;
#[path = "../backend/commands/mod.rs"]
mod commands;
#[path = "../backend/init.rs"]
mod init;

mod app;
mod theme;
mod icons;
mod config;
mod viz;

use app::{App, Message};
use init::Backend;

fn main() -> iced::Result {
    // Own a Tokio runtime for the whole process and enter it so the backend's
    // background tasks (audio decode feeders, visualizer analysis, queue manager)
    // spawn onto it. iced's winit event loop blocks the main thread inside run().
    let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
    let _guard = runtime.enter();

    // Apply the persisted window-decorations preference at boot (winit only
    // supports toggling at runtime, so the initial state must be set here).
    let decorations = crate::config::Config::load().window_decorations.unwrap_or(true);

    iced::application(boot, App::update, App::view)
        .title("Firmium")
        .theme(App::theme)
        .subscription(App::subscription)
        .default_font(iced::Font::with_name("Liberation Mono"))
        .font(include_bytes!("../assets/fonts/LiberationMono-Regular.ttf").as_slice())
        .window_size(iced::Size::new(1200.0, 800.0))
        .decorations(decorations)
        .run()
}

/// Boot: build the backend (inside the entered runtime), the initial App state,
/// and the auto-login task (if saved credentials exist).
fn boot() -> (App, iced::Task<Message>) {
    let backend = Backend::new().expect("backend init failed");
    let app = App::new(backend);
    let task = app.initial_task();
    (app, task)
}
