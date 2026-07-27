pub use firmium_backend::types::{AudioDevice, PlaybackState};

mod app;
mod theme;
mod icons;
mod fonts;
mod playlists;
mod viz;

use app::{App, Message};
use firmium_backend::init::Backend;

fn main() -> iced::Result {
    // `firmium <cmd> [arg]` controls an already-running instance over the IPC
    // socket/pipe instead of launching a second GUI — e.g. `firmium play-pause`.
    let cli_args: Vec<String> = std::env::args().skip(1).collect();
    if !cli_args.is_empty() {
        std::process::exit(run_cli(&cli_args.join(" ")));
    }

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

    let mut builder = iced::application(
        move || boot(backend.lock().unwrap().take().expect("boot called twice")),
        App::update,
        App::view,
    )
    .title("Firmium")
    .theme(App::theme)
    .subscription(App::subscription)
    .settings(iced::Settings { id: Some("firmium".to_string()), ..Default::default() })
    .font(include_bytes!("../assets/fonts/LiberationMono-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/FiraCode-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Hack-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Cousine-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/BigBlueTerminalPlus.ttf").as_slice())
    .window(iced::window::Settings {
        platform_specific: platform_specific(),
        icon: window_icon(),
        ..Default::default()
    })
    .window_size(iced::Size::new(1200.0, 800.0))
    .decorations(decorations);

    if let Some(font) = crate::fonts::resolve_font(&font_family) {
        builder = builder.default_font(font);
    }

    let result = builder.run();

    // `runtime`'s Drop blocks the current thread until every outstanding
    // spawn_blocking task (e.g. decode-feeder loops in audio/session.rs,
    // which only stop at end-of-track/error/cancel, never on window close)
    // finishes — so closing the window mid-playback would hang the process
    // and require a kill. Shut down without waiting: the OS reclaims threads
    // on exit anyway.
    runtime.shutdown_background();

    result
}

/// On X11 and Wayland, winit uses `application_id` as the window class/app_id
#[cfg(target_os = "linux")]
fn platform_specific() -> iced::window::settings::PlatformSpecific {
    iced::window::settings::PlatformSpecific {
        application_id: "firmium".to_string(),
        ..Default::default()
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_specific() -> iced::window::settings::PlatformSpecific {
    Default::default()
}

/// Decodes the bundled app icon for use as the window's title bar/taskbar icon
/// (X11, Windows; Wayland ignores this and relies on `application_id` instead).
fn window_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../assets/app-icons/128x128.png");
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    iced::window::icon::from_rgba(image.into_raw(), width, height).ok()
}

/// Build the initial App state and spawn startup tasks.
/// Backend is already constructed before the window opens.
fn boot(backend: Backend) -> (App, iced::Task<Message>) {
    let app = App::new(backend);
    let task = app.initial_task();
    (app, task)
}

/// Sends one line to the running instance's IPC socket/pipe and prints the
/// reply. Synchronous std I/O — a one-shot CLI command doesn't need a Tokio
/// runtime. Returns the process exit code.
fn run_cli(cmd: &str) -> i32 {
    match cli_send(cmd) {
        Ok(reply) => {
            println!("{reply}");
            if reply.starts_with("error") { 1 } else { 0 }
        }
        Err(e) => {
            eprintln!("firmium: {e} (is the app running?)");
            1
        }
    }
}

#[cfg(unix)]
fn cli_send(cmd: &str) -> std::io::Result<String> {
    use std::io::{BufRead, BufReader, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(firmium_backend::ipc::socket_path())?;
    writeln!(stream, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}

#[cfg(windows)]
fn cli_send(cmd: &str) -> std::io::Result<String> {
    use std::io::{BufRead, BufReader, Write};
    let mut pipe = std::fs::OpenOptions::new().read(true).write(true).open(firmium_backend::ipc::PIPE_NAME)?;
    writeln!(pipe, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(pipe).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}
