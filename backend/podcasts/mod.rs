pub mod feed;
pub mod store;

use std::sync::Arc;

pub use store::{PodcastChannel, PodcastEpisode, PodcastStore};

use crate::state::AppState;

pub async fn add_channel(
    state: Arc<AppState>,
    store: Arc<PodcastStore>,
    feed_url: String,
) -> Result<PodcastChannel, String> {
    let parsed = feed::fetch_and_parse(&state.http, &feed_url).await?;
    let channel = store.add_channel(
        &feed_url,
        &parsed.title,
        parsed.description.as_deref(),
        parsed.image_url.as_deref(),
    )?;
    store.insert_episodes(&channel.id, &parsed.episodes)?;
    Ok(channel)
}

/// Returns how many *new* episodes were added.
pub async fn refresh_channel(
    state: Arc<AppState>,
    store: Arc<PodcastStore>,
    channel_id: String,
    feed_url: String,
) -> Result<usize, String> {
    let parsed = feed::fetch_and_parse(&state.http, &feed_url).await?;
    store.insert_episodes(&channel_id, &parsed.episodes)
}

pub fn list_channels(store: Arc<PodcastStore>) -> Result<Vec<PodcastChannel>, String> {
    store.list_channels()
}

pub fn list_episodes(store: Arc<PodcastStore>, channel_id: String) -> Result<Vec<PodcastEpisode>, String> {
    store.list_episodes(&channel_id)
}

pub fn unsubscribe(store: Arc<PodcastStore>, channel_id: String) -> Result<(), String> {
    store.unsubscribe(&channel_id)
}

/// One-shot capability probe: does the connected server implement the Subsonic
/// podcast endpoints? Navidrome doesn't (github.com/navidrome/navidrome/issues/793).
/// Result is logged only — this slice always uses client-side RSS regardless.
pub async fn probe_server_podcast_support(state: Arc<AppState>) -> bool {
    let (server, username, password) = {
        let conn = state.connection.read();
        match (&conn.server, &conn.username, &conn.password) {
            (Some(s), Some(u), Some(p)) => (s.clone(), u.clone(), p.clone()),
            _ => return false,
        }
    };
    let auth = crate::commands::auth::generate_auth_params(username, password);
    let mut url = match reqwest::Url::parse(&format!("{server}/rest/getPodcasts")) {
        Ok(u) => u,
        Err(_) => return false,
    };
    {
        let mut query = url.query_pairs_mut();
        for key in ["u", "t", "s", "v", "c", "f"] {
            query.append_pair(key, auth[key].as_str().unwrap_or(""));
        }
    }

    let Ok(resp) = state.http.get(url).send().await else { return false };
    let Ok(json) = resp.json::<serde_json::Value>().await else { return false };
    let supported = json
        .get("subsonic-response")
        .and_then(|r| r.get("podcasts"))
        .is_some();
    eprintln!("Podcast server-capability probe: supported={supported}");
    supported
}
