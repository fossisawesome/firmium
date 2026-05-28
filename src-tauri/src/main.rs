// Suppress the console window on Windows release builds — unless --debug is passed,
// in which case we allocate one below so log output is visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate app_lib;

fn main() {
    // On Windows release builds the process has no console (windows_subsystem = "windows").
    // If --debug is passed, attach a new console so terminal output is visible.
    #[cfg(all(windows, not(debug_assertions)))]
    if std::env::args().any(|a| a == "--debug") {
        extern "system" { fn AllocConsole() -> i32; }
        unsafe { AllocConsole(); }
    }

    app_lib::run()
}
