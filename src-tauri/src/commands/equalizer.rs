//! Equalizer profile storage (`~/.config/firmium/eq.toml`) and the Tauri
//! commands the Settings UI uses to read/write it. Mirrors the TOML pattern in
//! `commands/themes.rs`. Applying a profile pushes live band coefficients into
//! the running `AudioPlayer` via `set_eq_runtime`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use crate::audio::eq::{BandKind, EqBand, EqRuntimeConfig};
use crate::audio::AudioPlayer;

/// Default Q for graphic-EQ bands (fixed 10-band ISO set, defined in the UI).
const GRAPHIC_Q: f32 = 1.4;
const PARAMETRIC_DEFAULT_Q: f32 = 1.0;

// ── On-disk schema ──────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct BandSpec {
    pub freq: f32,
    pub gain: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<f32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Profile {
    #[serde(rename = "type")]
    pub kind: String,
    pub bands: Vec<BandSpec>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DeviceEq {
    pub active_profile: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct EqSettings {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct EqFile {
    #[serde(default)]
    pub settings: EqSettings,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
    #[serde(default)]
    pub devices: HashMap<String, DeviceEq>,
}

// ── DTOs returned to the frontend ───────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub name: String,
    pub kind: String,
    pub bands: Vec<BandSpec>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EqState {
    pub enabled: bool,
    pub profiles: Vec<ProfileInfo>,
    pub default_device: Option<String>,
    /// Active profile name for the default device (convenience).
    pub active_profile: Option<String>,
    /// device name → active profile name, for every assigned device.
    pub device_profiles: HashMap<String, String>,
}

// ── File IO ─────────────────────────────────────────────────────────────────

fn config_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| format!("No config dir: {e}"))?;
    Ok(dir.join("eq.toml"))
}

fn read_file<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> EqFile {
    let Ok(path) = config_path(app) else { return EqFile::default() };
    let Ok(content) = std::fs::read_to_string(&path) else { return EqFile::default() };
    toml::from_str(&content).unwrap_or_default()
}

fn write_file<R: tauri::Runtime>(app: &tauri::AppHandle<R>, file: &EqFile) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
    }
    let content = toml::to_string_pretty(file).map_err(|e| format!("Failed to serialize eq.toml: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("Failed to write eq.toml: {e}"))
}

// ── Band conversion ─────────────────────────────────────────────────────────

/// Convert a stored profile to runtime EQ bands. Graphic profiles use shelves at
/// the extremes and peaking in the middle; parametric profiles are all peaking
/// with the user-supplied Q.
fn profile_to_bands(profile: &Profile) -> Vec<EqBand> {
    let parametric = profile.kind == "parametric";
    let last = profile.bands.len().saturating_sub(1);
    profile
        .bands
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let kind = if parametric {
                BandKind::Peaking
            } else if i == 0 {
                BandKind::LowShelf
            } else if i == last {
                BandKind::HighShelf
            } else {
                BandKind::Peaking
            };
            EqBand {
                kind,
                freq: b.freq,
                gain_db: b.gain,
                q: b.q.unwrap_or(if parametric { PARAMETRIC_DEFAULT_Q } else { GRAPHIC_Q }),
            }
        })
        .collect()
}

/// Resolve the runtime EQ config for the current default output device. Called
/// at `AudioPlayer` startup and after any profile mutation.
pub fn resolve_runtime<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> EqRuntimeConfig {
    let file = read_file(app);
    let bands = AudioPlayer::default_output_name()
        .and_then(|dev| file.devices.get(&dev))
        .and_then(|d| file.profiles.get(&d.active_profile))
        .map(profile_to_bands)
        .unwrap_or_default();
    EqRuntimeConfig { enabled: file.settings.enabled, bands }
}

/// Recompute the active runtime config from disk and push it into the player.
fn reapply<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(player) = app.try_state::<Arc<AudioPlayer>>() {
        let cfg = resolve_runtime(app);
        player.set_eq_runtime(cfg.enabled, cfg.bands);
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_eq_state<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) -> EqState {
    let file = read_file(&app_handle);
    let default_device = AudioPlayer::default_output_name();
    let active_profile = default_device
        .as_ref()
        .and_then(|dev| file.devices.get(dev))
        .map(|d| d.active_profile.clone());

    let device_profiles: HashMap<String, String> = file
        .devices
        .iter()
        .map(|(dev, d)| (dev.clone(), d.active_profile.clone()))
        .collect();

    let mut profiles: Vec<ProfileInfo> = file
        .profiles
        .into_iter()
        .map(|(name, p)| ProfileInfo { name, kind: p.kind, bands: p.bands })
        .collect();
    profiles.sort_by_key(|a| a.name.to_lowercase());

    EqState { enabled: file.settings.enabled, profiles, default_device, active_profile, device_profiles }
}

#[tauri::command]
pub fn save_eq_profile<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    name: String,
    kind: String,
    bands: Vec<BandSpec>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }
    let mut file = read_file(&app_handle);
    file.profiles.insert(name, Profile { kind, bands });
    write_file(&app_handle, &file)?;
    reapply(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn delete_eq_profile<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, name: String) -> Result<(), String> {
    let mut file = read_file(&app_handle);
    file.profiles.remove(&name);
    // Drop device assignments that pointed at the deleted profile.
    file.devices.retain(|_, d| d.active_profile != name);
    write_file(&app_handle, &file)?;
    reapply(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_eq_active_profile<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    device: String,
    profile: String,
) -> Result<(), String> {
    let mut file = read_file(&app_handle);
    file.devices.insert(device, DeviceEq { active_profile: profile });
    write_file(&app_handle, &file)?;
    reapply(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_eq_bands<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    profile: String,
    bands: Vec<BandSpec>,
) -> Result<(), String> {
    let mut file = read_file(&app_handle);
    if let Some(p) = file.profiles.get_mut(&profile) {
        p.bands = bands;
    } else {
        return Err(format!("Unknown profile: {profile}"));
    }
    write_file(&app_handle, &file)?;
    reapply(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_eq_enabled<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, enabled: bool) -> Result<(), String> {
    let mut file = read_file(&app_handle);
    file.settings.enabled = enabled;
    write_file(&app_handle, &file)?;
    reapply(&app_handle);
    Ok(())
}
