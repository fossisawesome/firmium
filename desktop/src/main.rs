#![cfg_attr(windows, windows_subsystem = "windows")]

pub use firmium_backend::types::{AudioDevice, PlaybackState};

mod app;
#[cfg(not(windows))]
mod cli;
#[cfg(windows)]
mod file_logging;
mod theme;
mod icons;
mod fonts;
mod playlists;
mod viz;

use app::{App, Message};
use firmium_backend::init::Backend;

fn main() -> iced::Result {
    #[cfg(not(windows))]
    {
        // Unix keeps the existing `firmium <cmd>` entry point; Windows uses firmium-cli.exe.
        let cli_args: Vec<String> = std::env::args().skip(1).collect();
        if !cli_args.is_empty() {
            std::process::exit(cli::run_cli(&cli_args.join(" ")));
        }
    }

    #[cfg(windows)]
    let _log_file = match file_logging::init() {
        Ok(file) => Some(file),
        Err(error) => {
            eprintln!("Could not open GUI log: {error}");
            None
        }
    };

    // Own a Tokio runtime for the whole process and enter it so the backend's
    // background tasks (audio decode feeders, visualizer analysis, queue manager)
    // spawn onto it. iced's winit event loop blocks the main thread inside run().
    let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
    let _guard = runtime.enter();

    // Apply the persisted window-decorations preference at boot (winit only
    // supports toggling at runtime, so the initial state must be set here).
    let decorations = firmium_backend::config::Config::load().window_decorations.unwrap_or(true);

    // Build the backend before the window opens so slow one-time init (cpal
    // device negotiation, SQLite open, TLS client construction) blocks before
    // winit creates a window rather than after — invisible delay instead of a
    // visible "not responding" freeze.
    let backend = std::sync::Arc::new(std::sync::Mutex::new(
        Some(Backend::new().expect("backend init failed"))
    ));

    // BootFn requires Fn (not FnOnce); use Mutex<Option<_>> to take the backend
    // exactly once on the single boot call.
    let font_family = firmium_backend::config::Config::load()
        .font_family
        .unwrap_or_else(|| "Liberation Mono".to_string());

    let icon_image = image::load_from_memory(include_bytes!("../assets/app-icons/64x64.png"))
        .expect("failed to decode app icon")
        .into_rgba8();
    let icon = iced::window::icon::from_rgba(
        icon_image.as_raw().clone(),
        icon_image.width(),
        icon_image.height(),
    )
    .expect("invalid app icon");

    let mut builder = iced::application(
        move || boot(backend.lock().unwrap().take().expect("boot called twice")),
        App::update,
        App::view,
    )
    .title("Firmium")
    .theme(App::theme)
    .subscription(App::subscription)
    .window(iced::window::Settings { icon: Some(icon), ..Default::default() })
    .font(include_bytes!("../assets/fonts/LiberationMono-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/FiraCode-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Hack-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Cousine-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/BigBlueTerminalPlus.ttf").as_slice())
    .window_size(iced::Size::new(1200.0, 800.0))
    .exit_on_close_request(false)
    .decorations(decorations);

    if let Some(font) = crate::fonts::resolve_font(&font_family) {
        builder = builder.default_font(font);
    }

    let result = builder.run();

    // Some in-flight blocking tasks may outlive the GUI; do not wait for them
    // after the window close handler has stopped playback.
    runtime.shutdown_background();

    result
}

/// Build the initial App state and spawn startup tasks.
/// Backend is already constructed before the window opens.
fn boot(backend: Backend) -> (App, iced::Task<Message>) {
    let app = App::new(backend);
    let task = app.initial_task();
    (app, task)
}
