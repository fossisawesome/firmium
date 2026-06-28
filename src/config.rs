//! User config persisted at `~/.config/<id>/config.toml`. Replaces the bits of
//! state the Svelte app kept in localStorage (server, last theme, volume).
//! Passwords stay in the OS keyring, not here.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A server + username pair the user has logged into. Passwords for each live in
/// the OS keyring (keyed by server + username), never here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedAccount {
    pub server: String,
    pub username: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub server: Option<String>,
    pub username: Option<String>,
    pub theme_id: Option<String>,
    pub volume: Option<f32>,
    #[serde(default)]
    pub accounts: Vec<SavedAccount>,
    #[serde(default)]
    pub download_format: Option<String>,
    #[serde(default)]
    pub lrclib_enabled: Option<bool>,
    #[serde(default)]
    pub lyrics_word_fill: Option<bool>,
    #[serde(default)]
    pub window_decorations: Option<bool>,
    #[serde(default)]
    pub viz_cover_colors: Option<bool>,
}

fn config_path() -> PathBuf {
    crate::paths::config_dir().join("config.toml")
}

impl Config {
    pub fn load() -> Self {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(config_path(), s);
        }
    }
}
