//! Filesystem paths for app data / config / cache.
//!
//! Mirrors the directories Tauri used (identifier `com.fossisawesome.firmium`)
//! so existing user data — play history DB, cover cache, EQ profiles, user
//! themes — is preserved across the migration. Replaces `app.path().*`.

use std::path::PathBuf;

const APP_ID: &str = "com.fossisawesome.firmium";

fn ensure(dir: PathBuf) -> PathBuf {
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// `~/.local/share/com.fossisawesome.firmium` (Linux). Holds `play_history.db`.
pub fn data_dir() -> PathBuf {
    ensure(dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join(APP_ID))
}

/// `~/.config/com.fossisawesome.firmium` (Linux).
/// Holds `eq.toml`, `eq-profiles/`, `themes/`, `config.toml`.
pub fn config_dir() -> PathBuf {
    ensure(dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join(APP_ID))
}

/// `~/.cache/com.fossisawesome.firmium` (Linux). Holds `covers/`.
pub fn cache_dir() -> PathBuf {
    ensure(dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join(APP_ID))
}

/// Music directory for downloads / the local library (`~/Music`).
pub fn audio_dir() -> PathBuf {
    dirs::audio_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Music")))
        .unwrap_or_else(|| PathBuf::from("."))
}
