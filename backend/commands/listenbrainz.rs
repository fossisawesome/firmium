// ============================================================================
// LISTENBRAINZ SCROBBLING
// ============================================================================
// Submits a "listen" to ListenBrainz on track completion, in parallel with the
// Subsonic scrobble. The user token is stored in the OS keyring (same service
// as other credentials) under `listenbrainz_token`; submission is a no-op when
// no token is stored, so presence of the token is what enables the feature.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::mappers::Song;
use crate::state::AppState;

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use keyring::Entry;

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
const SERVICE: &str = "firmium-desktop";
const LB_TOKEN_KEY: &str = "listenbrainz_token";
const LB_SUBMIT_URL: &str = "https://api.listenbrainz.org/1/submit-listens";

/// Reads the stored ListenBrainz token from the keyring, or `None` if unset/empty.
fn load_token() -> Option<String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        if let Ok(entry) = Entry::new(SERVICE, LB_TOKEN_KEY) {
            if let Ok(token) = entry.get_password() {
                let token = token.trim().to_string();
                if !token.is_empty() {
                    return Some(token);
                }
            }
        }
    }
    None
}

/// Fire-and-forget ListenBrainz "single" listen submission, mirroring
/// `fire_scrobble`. No-op when no token is stored. Errors are logged only.
pub(crate) fn fire_listenbrainz_listen(_state: Arc<AppState>, song: Song) {
    tokio::spawn(async move {
        // The keyring lookup blocks (D-Bus, possibly an unlock prompt), so it
        // stays off the caller and off the async workers.
        let Ok(Some(token)) = tokio::task::spawn_blocking(load_token).await else { return };
        let listened_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut track_metadata = serde_json::json!({
            "artist_name": song.artist,
            "track_name": song.title,
        });
        if !song.album.is_empty() {
            track_metadata["release_name"] = serde_json::Value::String(song.album.clone());
        }

        let body = serde_json::json!({
            "listen_type": "single",
            "payload": [{ "listened_at": listened_at, "track_metadata": track_metadata }],
        });

        let res = crate::tls::http()
            .post(LB_SUBMIT_URL)
            .header("Authorization", format!("Token {token}"))
            .json(&body)
            .send()
            .await;
        if let Err(e) = res {
            eprintln!("ListenBrainz submit failed: {e}");
        }
    });
}
