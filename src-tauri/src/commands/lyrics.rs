use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start: i64,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResult {
    pub lines: Vec<LyricLine>,
    pub synced: bool,
}

/// Converts an LRC timestamp "mm:ss.xx" or "mm:ss.xxx" to milliseconds.
fn parse_lrc_timestamp(mm: &str, ss: &str, frac: &str) -> i64 {
    let frac_ms: i64 = if frac.len() == 2 {
        frac.parse::<i64>().unwrap_or(0) * 10
    } else {
        frac.parse::<i64>().unwrap_or(0)
    };
    (mm.parse::<i64>().unwrap_or(0) * 60 + ss.parse::<i64>().unwrap_or(0)) * 1000 + frac_ms
}

/// Parses a single LRC line of the form "[mm:ss.xx]text", returning the
/// timestamp in ms and the remaining text if it matches.
fn parse_lrc_line(line: &str) -> Option<LyricLine> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find(']')?;
    let (timestamp, text) = (&rest[..close], &rest[close + 1..]);
    let (mm, ss_frac) = timestamp.split_once(':')?;
    let (ss, frac) = ss_frac.split_once('.')?;
    if mm.len() > 2 || mm.is_empty() || ss.len() != 2 || !(2..=3).contains(&frac.len()) {
        return None;
    }
    if !mm.chars().all(|c| c.is_ascii_digit())
        || !ss.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some(LyricLine {
        start: parse_lrc_timestamp(mm, ss, frac),
        value: text.trim_start().to_string(),
    })
}

/// Parses an LRC-format string into time-sorted lyric lines.
fn parse_lrc_lines(lrc_text: &str) -> Vec<LyricLine> {
    let mut lines: Vec<LyricLine> = lrc_text.lines().filter_map(parse_lrc_line).collect();
    lines.sort_by_key(|l| l.start);
    lines
}

#[tauri::command]
pub fn parse_lrc(lrc_text: String) -> Vec<LyricLine> {
    parse_lrc_lines(&lrc_text)
}

/// Strips a leading run of qualifier words (case-insensitive) from a title,
/// e.g. "Song (Live)" -> "Song", "Song [Remix]" -> "Song".
fn strip_qualifier_suffix(title: &str) -> String {
    const QUALIFIERS: &[&str] = &[
        "remix", "live", "extended", "acoustic", "instrumental", "remaster",
        "cover", "edit", "version", "feat.", "feat", "featuring",
    ];
    let mut result = title.to_string();
    loop {
        let trimmed = result.trim_end();
        let open = if trimmed.ends_with(')') {
            '('
        } else if trimmed.ends_with(']') {
            '['
        } else {
            break;
        };
        let Some(start) = trimmed.rfind(open) else { break };
        let inner = &trimmed[start + 1..trimmed.len() - 1];
        let first_word = inner
            .split(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_lowercase();
        if QUALIFIERS.iter().any(|q| first_word.starts_with(q)) {
            result = trimmed[..start].trim_end().to_string();
        } else {
            break;
        }
    }
    result
}

/// Strips a trailing " - feat. ..." / " - featuring ..." suffix from a title.
fn strip_feat_suffix(title: &str) -> String {
    let lower = title.to_lowercase();
    for marker in [" - feat.", " - feat ", " - featuring"] {
        if let Some(idx) = lower.find(marker) {
            return title[..idx].trim_end().to_string();
        }
    }
    title.to_string()
}

/// Splits an artist string on "feat"/"ft"/"featuring"/"/" and keeps the first part.
fn primary_artist(artist: &str) -> String {
    let lower = artist.to_lowercase();
    let mut cut = artist.len();
    for marker in [" feat.", " feat ", " featuring ", " ft.", " ft ", "/"] {
        if let Some(idx) = lower.find(marker) {
            cut = cut.min(idx);
        }
    }
    artist[..cut].trim().to_string()
}

/// Normalizes a song title/artist pair for better lrclib matching.
fn normalize_lrclib_query(artist: &str, title: &str) -> (String, String) {
    let title = strip_feat_suffix(&strip_qualifier_suffix(title));
    let artist = primary_artist(artist);
    (artist, title)
}

#[derive(Debug, Deserialize)]
struct LrclibResponse {
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    synced_lyrics: Option<String>,
    #[serde(default)]
    plain_lyrics: Option<String>,
}

/// Queries lrclib.net for synced/plain lyrics. Returns `Ok(None)` if no
/// lyrics are found (lrclib 404).
#[tauri::command]
pub async fn fetch_lrclib_lyrics(
    artist: String,
    title: String,
    duration: f64,
) -> Result<Option<LyricsResult>, String> {
    let (artist, title) = normalize_lrclib_query(&artist, &title);
    let url = reqwest::Url::parse_with_params(
        "https://lrclib.net/api/get",
        &[
            ("artist_name", artist.as_str()),
            ("track_name", title.as_str()),
            ("duration", duration.round().to_string().as_str()),
        ],
    )
    .map_err(|e| e.to_string())?;
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header("Lrclib-Client", "Firmium (https://github.com/fossisawesome/firmium)")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !res.status().is_success() {
        return Err(format!("LRCLIB {}", res.status()));
    }

    let data: LrclibResponse = res.json().await.map_err(|e| e.to_string())?;

    if data.instrumental {
        return Ok(Some(LyricsResult {
            lines: vec![LyricLine { start: 0, value: "♪ Instrumental ♪".to_string() }],
            synced: false,
        }));
    }
    if let Some(synced) = data.synced_lyrics {
        let lines = parse_lrc_lines(&synced);
        if !lines.is_empty() {
            return Ok(Some(LyricsResult { lines, synced: true }));
        }
    }
    if let Some(plain) = data.plain_lyrics {
        let lines = plain
            .lines()
            .map(|v| LyricLine { start: 0, value: v.to_string() })
            .collect();
        return Ok(Some(LyricsResult { lines, synced: false }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc_text() {
        let lrc = "[00:01.00]First\n[00:00.50]Zero\n[01:02.123]Third\nnot a line";
        let lines = parse_lrc_lines(lrc);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start, 500);
        assert_eq!(lines[0].value, "Zero");
        assert_eq!(lines[1].start, 1000);
        assert_eq!(lines[2].start, 62123);
        assert_eq!(lines[2].value, "Third");
    }

    #[test]
    fn strips_qualifier_suffix() {
        assert_eq!(strip_qualifier_suffix("Song (Live)"), "Song");
        assert_eq!(strip_qualifier_suffix("Song [Remix]"), "Song");
        assert_eq!(strip_qualifier_suffix("Song (feat. Someone)"), "Song");
        assert_eq!(strip_qualifier_suffix("Song (Edition Deluxe)"), "Song");
        assert_eq!(strip_qualifier_suffix("Song (Deluxe Edition)"), "Song (Deluxe Edition)");
        assert_eq!(strip_qualifier_suffix("Just A Song"), "Just A Song");
    }

    #[test]
    fn strips_feat_suffix() {
        assert_eq!(strip_feat_suffix("Song - feat. Someone"), "Song");
        assert_eq!(strip_feat_suffix("Song - featuring Someone"), "Song");
        assert_eq!(strip_feat_suffix("Song"), "Song");
    }

    #[test]
    fn splits_primary_artist() {
        assert_eq!(primary_artist("Artist feat. Other"), "Artist");
        assert_eq!(primary_artist("Artist ft. Other"), "Artist");
        assert_eq!(primary_artist("Artist / Other"), "Artist");
        assert_eq!(primary_artist("Solo Artist"), "Solo Artist");
    }
}
