// Prevents an extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `firmium <cmd> [arg]` controls an already-running instance over the IPC
    // socket/pipe instead of launching a second GUI, e.g. `firmium play-pause`.
    let cli_args: Vec<String> = std::env::args().skip(1).collect();
    if !cli_args.is_empty() {
        std::process::exit(firmium_lib::cli::run(&cli_args.join(" ")));
    }
    firmium_lib::run();
}
