// Backend modules live under ../backend/ but are declared here at the crate
// root via #[path] so every existing `crate::...` reference inside them keeps
// resolving unchanged (no per-file path rewrites needed during the migration).
#[path = "../backend/types.rs"]
mod types;
pub use types::{AudioDevice, PlaybackState};

#[path = "../backend/events.rs"]
mod events;
#[path = "../backend/errors.rs"]
mod errors;
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
#[path = "../backend/podcasts/mod.rs"]
mod podcasts;
#[path = "../backend/init.rs"]
mod init;
#[path = "../backend/ipc.rs"]
mod ipc;

mod app;
mod theme;
mod icons;
mod config;
mod fonts;
mod playlists;
mod viz;

use app::{App, Message};
use init::Backend;

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
    let decorations = crate::config::Config::load().window_decorations.unwrap_or(true);

    // Build the backend before the window opens so slow one-time init (cpal
    // device negotiation, SQLite open, TLS client construction) blocks before
    // winit creates a window rather than after — invisible delay instead of a
    // visible "not responding" freeze.
    let backend = std::sync::Arc::new(std::sync::Mutex::new(
        Some(Backend::new().expect("backend init failed"))
    ));

    // BootFn requires Fn (not FnOnce); use Mutex<Option<_>> to take the backend
    // exactly once on the single boot call.
    let font_family = crate::config::Config::load()
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
    .font(include_bytes!("../assets/fonts/LiberationMono-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/FiraCode-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Hack-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/Cousine-Regular.ttf").as_slice())
    .font(include_bytes!("../assets/fonts/BigBlueTerminalPlus.ttf").as_slice())
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
    let mut stream = std::os::unix::net::UnixStream::connect(ipc::socket_path())?;
    writeln!(stream, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}

#[cfg(windows)]
fn cli_send(cmd: &str) -> std::io::Result<String> {
    use std::io::{BufRead, BufReader, Write};
    let mut pipe = std::fs::OpenOptions::new().read(true).write(true).open(ipc::PIPE_NAME)?;
    writeln!(pipe, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(pipe).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}
