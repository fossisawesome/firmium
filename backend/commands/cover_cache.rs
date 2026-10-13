use std::path::{Path, PathBuf};

const MAX_CACHE_BYTES: u64 = 200 * 1024 * 1024; // 200 MB
const MAX_COVER_BYTES: u64 = 32 * 1024 * 1024; // 32 MB — far above any real cover, caps server OOM abuse

fn http_client() -> reqwest::Client {
    crate::tls::http()
}

fn covers_dir() -> PathBuf {
    let dir = crate::paths::cache_dir().join("covers");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Replaces characters that aren't safe in a filename with '_'.
fn sanitize_cover_id(cover_id: &str) -> String {
    let safe: String = cover_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    // A server-controlled "." or ".." would otherwise become a traversal-prone filename prefix.
    if safe.is_empty() || safe.chars().all(|c| c == '.') {
        "unknown".to_string()
    } else {
        safe
    }
}

/// Extensions a cached cover can have (see `extension_for_content_type`).
const COVER_EXTS: [&str; 4] = ["jpg", "png", "webp", "gif"];

/// Finds an existing cached file for this cover id, regardless of extension.
/// Touches the file's mtime on a hit so eviction is least-recently-used.
fn find_cached(dir: &Path, safe_id: &str) -> Option<PathBuf> {
    for ext in COVER_EXTS {
        let path = dir.join(format!("{safe_id}.{ext}"));
        if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) {
            let _ = file.set_modified(std::time::SystemTime::now());
            return Some(path);
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

/// Reads the response body, failing once it exceeds `MAX_COVER_BYTES` (covers
/// chunked responses that carry no `Content-Length`).
async fn read_capped(mut res: reqwest::Response) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        if (out.len() + chunk.len()) as u64 > MAX_COVER_BYTES {
            return Err("Cover art exceeds maximum size".to_string());
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
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
    if res.content_length().is_some_and(|len| len > MAX_COVER_BYTES) {
        return Err("Cover art exceeds maximum size".to_string());
    }
    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let bytes = read_capped(res).await?;

    let ext = extension_for_content_type(content_type.as_deref());
    let path = dir.join(format!("{safe_id}.{ext}"));
    // Write to a temp name first so a concurrent reader never sees a partial image.
    let tmp = dir.join(format!("{safe_id}.{ext}.tmp"));
    let write_dir = dir.clone();
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        evict_if_needed(&write_dir);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let path = dir.join(format!("{safe_id}.{ext}"));

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

#[cfg(test)]
mod tests {
    use super::sanitize_cover_id;

    #[test]
    fn cover_id_cannot_escape_the_cache_dir() {
        assert_eq!(sanitize_cover_id(".."), "__"); // dots are replaced, so no traversal component survives
        assert_eq!(sanitize_cover_id(""), "unknown");
        assert_eq!(sanitize_cover_id("../x"), "___x");
        assert_eq!(sanitize_cover_id("al-12_ok"), "al-12_ok");
    }
}
