use crate::commands::mappers::Song;

use super::types::Energy;

pub(crate) fn time_of_day() -> &'static str {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() % 86400)
        .unwrap_or(0);
    match (secs / 3600) as u32 {
        5..=11 => "Morning",
        12..=16 => "Afternoon",
        17..=20 => "Evening",
        _ => "Night",
    }
}
pub(crate) fn fmt_freq(f: f32) -> String {
    if f >= 1000.0 {
        let k = f / 1000.0;
        if k.fract() == 0.0 {
            format!("{k:.0}k")
        } else {
            format!("{k:.1}k")
        }
    } else {
        format!("{f:.0}")
    }
}

/// Keep songs whose BPM falls in the energy band; fall back to the whole pool
/// if too few have BPM tags. Caps at 60 tracks.
pub(crate) fn filter_energy(songs: Vec<Song>, e: Energy) -> Vec<Song> {
    let in_band = |s: &Song| match s.bpm {
        Some(b) => match e {
            Energy::Chill => b < 95.0,
            Energy::Mid => (95.0..130.0).contains(&b),
            Energy::High => b >= 130.0,
        },
        None => false,
    };
    let filtered: Vec<Song> = songs.iter().filter(|s| in_band(s)).cloned().collect();
    let mut out = if filtered.len() >= 10 { filtered } else { songs };
    out.truncate(60);
    out
}

pub(crate) fn fmt_time(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "0:00".to_string();
    }
    let s = secs as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
/// "3h 24m" / "47m" style duration for Recap totals.
pub(crate) fn fmt_hours(secs: i64) -> String {
    let secs = secs.max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}
