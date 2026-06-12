use std::io::Write as _;
use tauri::Manager;

use crate::DebugMode;

// ============================================================================
// LOGGING
// ============================================================================

/// Append a pre-formatted log entry (timestamp + level + message built by JS) to app-logs.txt.
/// In debug mode, also echoes to stderr so frontend console output is visible in the terminal.
#[tauri::command]
pub fn write_log(app_handle: tauri::AppHandle, entry: String) -> Result<(), String> {
    if app_handle.state::<DebugMode>().0 {
        eprintln!("[js] {}", entry);
    }
    let log_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let log_file = log_dir.join("app-logs.txt");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{}", entry).map_err(|e| e.to_string())
}

/// Delete the app-logs.txt file.
#[tauri::command]
pub fn delete_logs(app_handle: tauri::AppHandle) -> Result<(), String> {
    let log_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_file = log_dir.join("app-logs.txt");
    std::fs::remove_file(&log_file).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e.to_string()) }
    })
}

/// Return the absolute path to app-logs.txt so the UI can display it.
#[tauri::command]
pub fn get_log_path(app_handle: tauri::AppHandle) -> Result<String, String> {
    let log_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(log_dir.join("app-logs.txt").to_string_lossy().into_owned())
}

/// Expose debug mode to the frontend so it can block devtools shortcuts when false.
#[tauri::command]
pub fn is_debug_mode(app_handle: tauri::AppHandle) -> bool {
    app_handle.state::<DebugMode>().0
}

/// Return the app version string from Cargo.toml at compile time.
#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
