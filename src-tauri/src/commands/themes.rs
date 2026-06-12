use tauri::Manager;

/// Color variables for a theme, matching the CSS custom properties in style.css.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct ThemeColors {
    bg: String,
    surface: String,
    surface2: String,
    border: String,
    text: String,
    muted: String,
    accent: String,
    accent_dim: String,
    error: String,
    font: Option<String>,
    timing: Option<String>,
}

/// Raw shape of a .toml theme file on disk.
#[derive(serde::Deserialize)]
struct ThemeFile {
    name: String,
    color_scheme: Option<String>,
    colors: ThemeColors,
}

/// Serialized theme entry returned to the frontend via list_themes.
#[derive(serde::Serialize)]
pub struct ThemeEntry {
    id: String,
    name: String,
    color_scheme: String,
    colors: ThemeColors,
}

// Themes embedded at compile time by build.rs — used on Android where
// std::fs cannot read APK assets.
#[cfg(target_os = "android")]
include!(concat!(env!("OUT_DIR"), "/embedded_themes.rs"));

/// Parse a TOML string into a ThemeEntry, returning None if invalid.
fn parse_theme(id: &str, content: &str) -> Option<ThemeEntry> {
    let tf = toml::from_str::<ThemeFile>(content).ok()?;
    Some(ThemeEntry {
        id: id.to_string(),
        name: tf.name,
        color_scheme: tf.color_scheme.unwrap_or_else(|| "dark".to_string()),
        colors: tf.colors,
    })
}

/// Read all valid .toml files from a directory into ThemeEntry values.
fn load_themes_from_dir(dir: &std::path::Path) -> Vec<ThemeEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") { continue }
        let id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        if let Some(t) = parse_theme(&id, &content) { result.push(t) }
    }
    result
}

/// Return all available themes. On Android built-ins come from the compile-time
/// embedded array; on desktop they are read from the resource directory (or source
/// dir in dev). User themes from the app config dir override built-ins on all platforms.
#[tauri::command]
pub fn list_themes(app_handle: tauri::AppHandle) -> Vec<ThemeEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut themes: Vec<ThemeEntry> = Vec::new();

    // User themes take priority — collect them first and record their IDs.
    if let Ok(config_dir) = app_handle.path().app_config_dir() {
        for t in load_themes_from_dir(&config_dir.join("themes")) {
            seen.insert(t.id.clone());
            themes.push(t);
        }
    }

    // Read themes from disk (release: resource dir; debug: source themes/ dir).
    #[cfg(debug_assertions)]
    let bundled_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../themes");
    #[cfg(not(debug_assertions))]
    let bundled_dir = app_handle.path().resource_dir()
        .map(|d| d.join("themes"))
        .unwrap_or_default();

    for t in load_themes_from_dir(&bundled_dir) {
        if !seen.contains(&t.id) { seen.insert(t.id.clone()); themes.push(t); }
    }

    // Fall back to compile-time embedded themes for any IDs not found on disk
    // (std::fs can't read bundled assets from inside the APK on Android).
    #[cfg(target_os = "android")]
    for (id, content) in EMBEDDED_THEMES {
        if seen.contains(*id) { continue }
        if let Some(t) = parse_theme(id, content) { themes.push(t) }
    }

    // Keep Firmium first; sort the rest alphabetically by display name.
    themes.sort_by(|a, b| {
        if a.id == "firmium" { return std::cmp::Ordering::Less }
        if b.id == "firmium" { return std::cmp::Ordering::Greater }
        a.name.cmp(&b.name)
    });

    themes
}
