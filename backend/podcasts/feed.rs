// ============================================================================
// RSS/Atom feed fetch + parse (podcast feeds)
// ============================================================================
// Pure fetch+parse — no DB access here. `feed-rs` normalizes RSS 2.0 + iTunes
// namespace extensions (enclosure, duration, image) into one model.

use crate::podcasts::store::NewEpisode;

pub struct ParsedFeed {
    pub title: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub episodes: Vec<NewEpisode>,
}

pub async fn fetch_and_parse(http: &reqwest::Client, feed_url: &str) -> Result<ParsedFeed, String> {
    let bytes = http
        .get(feed_url)
        .send()
        .await
        .map_err(|e| format!("failed to fetch feed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("failed to read feed body: {e}"))?;

    let feed = feed_rs::parser::parse(&bytes[..]).map_err(|e| format!("failed to parse feed: {e}"))?;

    let title = feed.title.map(|t| t.content).unwrap_or_else(|| "Untitled Podcast".to_string());
    let description = feed.description.map(|d| d.content);
    let image_url = feed.logo.map(|l| l.uri).or_else(|| feed.icon.map(|i| i.uri));

    let mut episodes = Vec::new();
    for entry in feed.entries {
        let Some(media) = entry.media.first() else { continue };
        let Some(content) = media.content.first() else { continue };
        let Some(audio_url) = content.url.as_ref().map(|u| u.to_string()) else { continue };

        let guid = entry.id.clone();
        let ep_title = entry.title.map(|t| t.content).unwrap_or_else(|| "Untitled Episode".to_string());
        let ep_description = entry.summary.map(|s| s.content);
        let duration_seconds = content.duration.map(|d| d.as_secs() as i64);
        let published_at = entry.published.map(|d| d.timestamp());

        episodes.push(NewEpisode {
            guid,
            title: ep_title,
            description: ep_description,
            audio_url,
            duration_seconds,
            published_at,
        });
    }

    Ok(ParsedFeed { title, description, image_url, episodes })
}
