use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MAX_CACHE_BYTES: u64 = 200 * 1024 * 1024; // 200 MB

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

fn covers_dir() -> PathBuf {
    let dir = crate::paths::cache_dir().join("covers");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Replaces characters that aren't safe in a filename with '_'.
fn sanitize_cover_id(cover_id: &str) -> String {
    cover_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

/// Finds an existing cached file for this cover id, regardless of extension.
fn find_cached(dir: &Path, safe_id: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let prefix = format!("{safe_id}.");
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            return Some(entry.path());
        }
    }
    None
}

fn extension_for_content_type(content_type: Option<&str>) -> &'static str {
    match content_type {
        Some(ct) if ct.contains("png") => "png",
        Some(ct) if ct.contains("webp") => "webp",
        Some(ct) if ct.contains("gif") => "gif",
        _ => "jpg",
    }
}

/// Deletes the oldest-by-mtime files in `dir` until its total size is under
/// `MAX_CACHE_BYTES`.
fn evict_if_needed(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(PathBuf, u64, std::time::SystemTime)> = entries
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some((e.path(), meta.len(), meta.modified().ok()?))
        })
        .collect();

    let mut total: u64 = files.iter().map(|(_, size, _)| size).sum();
    if total <= MAX_CACHE_BYTES {
        return;
    }

    files.sort_by_key(|(_, _, mtime)| *mtime);
    for (path, size, _) in files {
        if total <= MAX_CACHE_BYTES {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= size;
        }
    }
}

/// Returns a filesystem path to the cached cover art for `cover_id`,
/// downloading it from `url` first if not already cached. The frontend
/// converts the path with `convertFileSrc()` for use in an `<img src>`.
pub async fn get_cover_art(cover_id: String, url: String) -> Result<String, String> {
    let dir = covers_dir();
    let safe_id = sanitize_cover_id(&cover_id);

    if let Some(path) = find_cached(&dir, &safe_id) {
        return Ok(path.to_string_lossy().into_owned());
    }

    let res = http_client().get(&url).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Cover art unavailable (HTTP {})", res.status()));
    }
    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let bytes = res.bytes().await.map_err(|e| e.to_string())?;

    let ext = extension_for_content_type(content_type.as_deref());
    let path = dir.join(format!("{safe_id}.{ext}"));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    evict_if_needed(&dir);

    Ok(path.to_string_lossy().into_owned())
}

/// Deletes all cached cover art.
pub fn clear_cover_cache() -> Result<(), String> {
    let dir = covers_dir();
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
