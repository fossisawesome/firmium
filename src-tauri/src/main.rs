// Suppress the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate app_lib;

fn main() {
    app_lib::run()
}
