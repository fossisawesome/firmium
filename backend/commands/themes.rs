use include_dir::{include_dir, Dir};

/// The 19 built-in theme TOMLs, embedded into the binary at compile time. A
/// native single-binary app has no resource-bundling step, so the built-ins
/// must be compiled in. User themes on disk still override these.
static BUILTIN_THEMES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/themes");

/// Color variables for a theme, matching the design tokens used by the UI.
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ThemeColors {
    pub bg: String,
    pub surface: String,
    pub surface2: String,
    pub border: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub accent_dim: String,
    pub error: String,
    pub font: Option<String>,
    pub timing: Option<String>,
}

/// Raw shape of a .toml theme file on disk.
#[derive(serde::Deserialize)]
struct ThemeFile {
    name: String,
    color_scheme: Option<String>,
    colors: ThemeColors,
}

/// A resolved theme entry.
#[derive(serde::Serialize, Clone)]
pub struct ThemeEntry {
    pub id: String,
    pub name: String,
    pub color_scheme: String,
    pub colors: ThemeColors,
}

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

/// Return all available themes. User themes from `~/.config/<id>/themes/`
/// override the compile-time built-ins of the same id. Firmium sorts first,
/// the rest alphabetically by display name.
pub fn list_themes() -> Vec<ThemeEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut themes: Vec<ThemeEntry> = Vec::new();

    // User themes take priority — collect them first and record their IDs.
    for t in load_themes_from_dir(&crate::paths::config_dir().join("themes")) {
        seen.insert(t.id.clone());
        themes.push(t);
    }

    // Built-ins embedded at compile time.
    for file in BUILTIN_THEMES.files() {
        let path = file.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") { continue }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        if seen.contains(id) { continue }
        if let Some(content) = file.contents_utf8() {
            if let Some(t) = parse_theme(id, content) {
                seen.insert(id.to_string());
                themes.push(t);
            }
        }
    }

    // Keep Firmium first; sort the rest alphabetically by display name.
    themes.sort_by(|a, b| {
        if a.id == "firmium" { return std::cmp::Ordering::Less }
        if b.id == "firmium" { return std::cmp::Ordering::Greater }
        a.name.cmp(&b.name)
    });

    themes
}
