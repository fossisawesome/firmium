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

/// Writes `contents` to `path` via a temp file + rename, so a crash or full
/// disk never leaves a truncated file behind.
pub fn write_atomic(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// Reads and parses `path`. A missing file yields `None`; a file that exists
/// but fails to parse is moved to `<path>.bak` (so the next save cannot
/// destroy it) and also yields `None`.
pub fn read_or_backup<T>(path: &std::path::Path, parse: impl FnOnce(&str) -> Option<T>) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    let parsed = parse(&text);
    if parsed.is_none() {
        let mut bak = path.as_os_str().to_owned();
        bak.push(".bak");
        let _ = std::fs::rename(path, PathBuf::from(bak));
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_file_is_moved_aside_not_lost() {
        let dir = std::env::temp_dir().join(format!("firmium-paths-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("x.json");
        write_atomic(&f, "{ broken").unwrap();
        let parsed: Option<serde_json::Value> = read_or_backup(&f, |s| serde_json::from_str(s).ok());
        assert!(parsed.is_none());
        assert!(!f.exists());
        assert_eq!(std::fs::read_to_string(dir.join("x.json.bak")).unwrap(), "{ broken");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
