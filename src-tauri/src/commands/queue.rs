use std::collections::HashSet;
use std::sync::Arc;

use rand::seq::{IndexedRandom, SliceRandom};
use tauri::{AppHandle, State};

use crate::audio::AudioPlayer;
use crate::commands::local_library::{find_local_match_internal, get_local_track_path_internal};
use crate::commands::mappers::Song;
use crate::commands::subsonic::{build_stream_url, fire_report_playback, fire_save_play_queue, fire_scrobble};
use crate::queue_state::{emit_queue_state, QueueState};
use crate::state::AppState;

// ── Internal helpers ─────────────────────────────────────────────────────────

pub(crate) fn replay_gain_db_pub(song: &Song) -> Option<f32> {
    replay_gain_db(song)
}

fn replay_gain_db(song: &Song) -> Option<f32> {
    let rg = song.replay_gain.as_ref()?.as_object()?;
    rg.get("trackGain")
        .or_else(|| rg.get("albumGain"))
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
}

/// Resolves the stream URL for a song: local file → local match → Subsonic stream.
pub(crate) async fn stream_url_for(song: &Song, app: &AppHandle, app_state: &AppState) -> Result<String, String> {
    if song.id.starts_with("local:") {
        let path = get_local_track_path_internal(app, app_state, &song.id)?;
        return Ok(format!("file://{path}"));
    }
    if let Some(path) = find_local_match_internal(app, app_state, &song.title, &song.artist, &song.album) {
        return Ok(format!("file://{path}"));
    }
    build_stream_url(app_state, &song.id)
}

/// Schedules a debounced (4s) save of the current play queue to the server.
/// Aborts any pending timer before scheduling the new one.
pub(crate) fn schedule_save_play_queue(
    app: &AppHandle,
    app_state: &Arc<AppState>,
    queue_state: &Arc<QueueState>,
    player_id_for_position: Option<String>,
    audio_player: &Arc<AudioPlayer>,
) {
    let mut inner = queue_state.inner.lock();

    if let Some(handle) = inner.save_timer.take() {
        handle.abort();
        drop(handle);
    }

    let ids: Vec<String> = inner.queue.iter()
        .filter(|t| !t.id.starts_with("local:"))
        .map(|t| t.id.clone())
        .collect();
    let current_id = inner.queue.get(inner.queue_idx as usize)
        .filter(|t| !t.id.starts_with("local:"))
        .map(|t| t.id.clone());

    drop(inner);

    if ids.is_empty() || current_id.is_none() { return; }

    let app = app.clone();
    let app_state = Arc::clone(app_state);
    let audio_player = Arc::clone(audio_player);

    let handle = tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
        let pos_ms = player_id_for_position
            .as_deref()
            .and_then(|pid| audio_player.get_current_position(pid).ok())
            .map(|p| (p * 1000.0) as i64);
        fire_save_play_queue(app, app_state, ids, current_id, pos_ms);
    });

    queue_state.inner.lock().save_timer = Some(handle);
}

/// Core play-at-index function. Must never hold QueueState lock while calling AudioPlayer.
pub(crate) async fn play_at(
    app: &AppHandle,
    queue_state: &Arc<QueueState>,
    app_state: &Arc<AppState>,
    audio_player: &Arc<AudioPlayer>,
    idx: usize,
) -> Result<(), String> {
    // 1. Lock → extract data, update idx, reset flags → unlock
    let (song, volume, old_player_id, preloaded_id, preloaded_track_id, rg_enabled) = {
        let mut inner = queue_state.inner.lock();
        let song = inner.queue.get(idx).cloned()
            .ok_or_else(|| format!("Queue index {idx} out of range"))?;
        inner.queue_idx = idx as i32;
        inner.reset_track_progress();
        (
            song,
            inner.volume,
            inner.current_player_id.clone(),
            inner.preloaded_player_id.clone(),
            inner.preloaded_track_id.clone(),
            inner.replay_gain_enabled,
        )
    }; // lock released

    let rg = if rg_enabled { replay_gain_db(&song) } else { None };

    // 2. Check for gapless promotion (preloaded session for this exact track)
    if let (Some(preloaded_pid), Some(preloaded_tid)) = (&preloaded_id, &preloaded_track_id) {
        if preloaded_tid == &song.id {
            // Stop the old session (if different from preloaded)
            if let Some(ref old) = old_player_id {
                if old != preloaded_pid {
                    let _ = audio_player.stop(old);
                }
            }
            let pid = preloaded_pid.clone();
            let _ = audio_player.resume(&pid);
            let _ = audio_player.set_volume(&pid, volume);
            {
                let mut inner = queue_state.inner.lock();
                inner.current_player_id = Some(pid.clone());
                inner.preloaded_player_id = None;
                inner.preloaded_track_id = None;
            }
            emit_queue_state(app, queue_state);
            fire_scrobble(app.clone(), Arc::clone(app_state), song.id.clone(), false);
            fire_report_playback(app.clone(), Arc::clone(app_state), song.id.clone(), 0, "starting".into());
            schedule_save_play_queue(app, app_state, queue_state, Some(pid), audio_player);
            return Ok(());
        }
    }

    // 3. Stop old session — no lock held
    if let Some(ref old) = old_player_id {
        let _ = audio_player.stop(old);
    }
    // A preloaded session exists but wasn't for this track (e.g. gapless preloaded
    // the sequential next while shuffle jumped elsewhere) — drop it so it doesn't leak.
    if let Some(ref pre) = preloaded_id {
        let _ = audio_player.stop(pre);
    }

    // 4. Resolve stream URL — async, no lock
    let stream_url = stream_url_for(&song, app, app_state).await?;

    // 5. Start playback — no lock held
    let new_pid = AudioPlayer::play_stream(audio_player, &stream_url, song.id.clone(), rg)?;
    let _ = audio_player.set_volume(&new_pid, volume);

    // 6. Lock again → store new player_id, clear preloaded → unlock
    {
        let mut inner = queue_state.inner.lock();
        inner.current_player_id = Some(new_pid.clone());
        inner.preloaded_player_id = None;
        inner.preloaded_track_id = None;
    }

    emit_queue_state(app, queue_state);
    fire_scrobble(app.clone(), Arc::clone(app_state), song.id.clone(), false);
    fire_report_playback(app.clone(), Arc::clone(app_state), song.id.clone(), 0, "starting".into());
    schedule_save_play_queue(app, app_state, queue_state, Some(new_pid), audio_player);

    Ok(())
}

/// Computes the next queue index respecting repeat-all. Returns None if at end and no repeat.
pub(crate) fn compute_next_idx(queue_idx: i32, queue_len: usize, repeat_all: bool) -> Option<usize> {
    let next = queue_idx + 1;
    if next < queue_len as i32 {
        Some(next as usize)
    } else if repeat_all && queue_len > 0 {
        Some(0)
    } else {
        None
    }
}

/// Whether a shuffle pass has any track left to play after `current_idx`
/// (an unplayed index, or any other index when repeat-all will reshuffle).
/// Non-mutating — used to gate the crossfade trigger without consuming the pass.
pub(crate) fn has_shuffle_next(
    queue_len: usize,
    current_idx: usize,
    played: &HashSet<usize>,
    repeat_all: bool,
) -> bool {
    if queue_len == 0 { return false; }
    if repeat_all { return queue_len > 1; }
    (0..queue_len).any(|i| i != current_idx && !played.contains(&i))
}

/// Picks the next shuffle index, marking `current_idx` played so each track
/// plays once per pass. Returns None when the pass is exhausted and repeat-all
/// is off. With repeat-all on, clears the pass and starts a fresh one (never
/// immediately repeating the current track when others exist).
pub(crate) fn next_shuffle_idx(
    queue_len: usize,
    current_idx: usize,
    played: &mut HashSet<usize>,
    repeat_all: bool,
) -> Option<usize> {
    if queue_len == 0 { return None; }
    played.insert(current_idx);

    let unplayed: Vec<usize> = (0..queue_len).filter(|i| !played.contains(i)).collect();
    if let Some(&pick) = unplayed.choose(&mut rand::rng()) {
        return Some(pick);
    }

    // Pass exhausted.
    if !repeat_all {
        return None;
    }
    played.clear();
    played.insert(current_idx);
    let candidates: Vec<usize> = (0..queue_len).filter(|&i| i != current_idx).collect();
    candidates.choose(&mut rand::rng()).copied().or(Some(current_idx))
}

fn fisher_yates(songs: &mut [Song]) {
    songs.shuffle(&mut rand::rng());
}

#[cfg(test)]
mod tests {
    use super::{compute_next_idx, has_shuffle_next, next_shuffle_idx};
    use std::collections::HashSet;

    // ── compute_next_idx ─────────────────────────────────────────────────────

    #[test]
    fn next_idx_last_track_repeat_all_wraps_to_zero() {
        assert_eq!(compute_next_idx(4, 5, true), Some(0));
    }

    #[test]
    fn next_idx_last_track_no_repeat_returns_none() {
        assert_eq!(compute_next_idx(4, 5, false), None);
    }

    #[test]
    fn next_idx_mid_queue_returns_next() {
        assert_eq!(compute_next_idx(2, 5, false), Some(3));
    }

    #[test]
    fn next_idx_empty_queue_returns_none() {
        assert_eq!(compute_next_idx(0, 0, true), None);
    }

    #[test]
    fn next_idx_single_track_repeat_all_wraps_to_zero() {
        assert_eq!(compute_next_idx(0, 1, true), Some(0));
    }

    // ── next_shuffle_idx ──────────────────────────────────────────────────────

    #[test]
    fn shuffle_plays_each_track_once_then_ends() {
        for _ in 0..200 {
            let mut played = HashSet::new();
            let mut cur = 0usize;
            let mut seen = vec![cur];
            for _ in 0..3 {
                let n = next_shuffle_idx(4, cur, &mut played, false)
                    .expect("pass not yet exhausted");
                assert!(!seen.contains(&n), "shuffle repeated {n} before exhausting the pass");
                seen.push(n);
                cur = n;
            }
            assert_eq!(seen.len(), 4, "every track should play exactly once");
            // Pass now exhausted, no repeat-all → ends.
            assert_eq!(next_shuffle_idx(4, cur, &mut played, false), None);
        }
    }

    #[test]
    fn shuffle_repeat_all_reshuffles_without_immediate_repeat() {
        for _ in 0..200 {
            let mut played: HashSet<usize> = (0..4).collect();
            let n = next_shuffle_idx(4, 2, &mut played, true);
            assert!(n.is_some(), "repeat-all should start a fresh pass");
            assert_ne!(n, Some(2), "must not immediately repeat the current track");
        }
    }

    #[test]
    fn shuffle_single_track_no_repeat_ends() {
        let mut played = HashSet::new();
        assert_eq!(next_shuffle_idx(1, 0, &mut played, false), None);
    }

    #[test]
    fn shuffle_single_track_repeat_all_replays() {
        let mut played = HashSet::new();
        assert_eq!(next_shuffle_idx(1, 0, &mut played, true), Some(0));
    }

    // ── has_shuffle_next ──────────────────────────────────────────────────────

    #[test]
    fn has_shuffle_next_true_when_unplayed_remain() {
        let played: HashSet<usize> = [0].into_iter().collect();
        assert!(has_shuffle_next(4, 0, &played, false));
    }

    #[test]
    fn has_shuffle_next_false_when_pass_exhausted() {
        let played: HashSet<usize> = (0..4).collect();
        assert!(!has_shuffle_next(4, 2, &played, false));
    }

    #[test]
    fn has_shuffle_next_true_when_exhausted_but_repeat_all() {
        let played: HashSet<usize> = (0..4).collect();
        assert!(has_shuffle_next(4, 2, &played, true));
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Clamp an arbitrary curve string to the two supported values.
pub(crate) fn normalize_crossfade_curve(curve: &str) -> String {
    if curve == "logarithmic" { "logarithmic".to_string() } else { "linear".to_string() }
}

/// Restores persisted playback settings from localStorage on startup.
/// Does not emit queue-state-changed — startup avoids the event race.
#[tauri::command]
pub fn init_playback_settings(
    state: State<'_, Arc<QueueState>>,
    volume: f32,
    crossfade_enabled: bool,
    crossfade_duration: f32,
    crossfade_curve: String,
    gapless_enabled: bool,
    replay_gain_enabled: bool,
    auto_continue: bool,
) -> Result<(), String> {
    let mut inner = state.inner.lock();
    inner.volume = volume.clamp(0.0, 1.0);
    inner.crossfade_enabled = crossfade_enabled;
    inner.crossfade_duration = crossfade_duration.clamp(1.0, 12.0);
    inner.crossfade_curve = normalize_crossfade_curve(&crossfade_curve);
    inner.gapless_enabled = gapless_enabled;
    inner.replay_gain_enabled = replay_gain_enabled;
    inner.auto_continue = auto_continue;
    Ok(())
}

/// Toggles Smart Radio auto-continue (seed more tracks when the queue ends).
#[tauri::command]
pub fn set_auto_continue(state: State<'_, Arc<QueueState>>, enabled: bool) -> Result<(), String> {
    state.inner.lock().auto_continue = enabled;
    Ok(())
}

#[tauri::command]
pub fn set_replay_gain_enabled(
    state: State<'_, Arc<QueueState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    enabled: bool,
) -> Result<(), String> {
    state.inner.lock().replay_gain_enabled = enabled;
    if !enabled {
        audio_player.set_all_replay_gain_factors(1.0);
    }
    Ok(())
}

#[tauri::command]
pub fn set_repeat_mode(
    app: AppHandle,
    state: State<'_, Arc<QueueState>>,
    repeat_one: bool,
    repeat_all: bool,
) -> Result<(), String> {
    {
        let mut inner = state.inner.lock();
        inner.repeat_one = repeat_one;
        inner.repeat_all = repeat_all;
    }
    emit_queue_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn toggle_shuffle(app: AppHandle, state: State<'_, Arc<QueueState>>) -> Result<(), String> {
    {
        let mut inner = state.inner.lock();
        inner.shuffle_enabled ^= true;
        // Start a fresh shuffle pass on every toggle.
        inner.shuffle_played.clear();
    }
    emit_queue_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_crossfade_settings(
    app: AppHandle,
    state: State<'_, Arc<QueueState>>,
    enabled: bool,
    duration_secs: f32,
) -> Result<(), String> {
    {
        let mut inner = state.inner.lock();
        inner.crossfade_enabled = enabled;
        inner.crossfade_duration = duration_secs.clamp(1.0, 12.0);
    }
    emit_queue_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_crossfade_curve(
    app: AppHandle,
    state: State<'_, Arc<QueueState>>,
    curve: String,
) -> Result<(), String> {
    { state.inner.lock().crossfade_curve = normalize_crossfade_curve(&curve); }
    emit_queue_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_gapless_enabled(
    app: AppHandle,
    state: State<'_, Arc<QueueState>>,
    enabled: bool,
) -> Result<(), String> {
    { state.inner.lock().gapless_enabled = enabled; }
    emit_queue_state(&app, &state);
    Ok(())
}

/// Replace the entire queue and start playing at start_idx.
#[tauri::command]
pub async fn set_queue(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    songs: Vec<Song>,
    start_idx: usize,
) -> Result<(), String> {
    {
        let mut inner = queue_state.inner.lock();
        inner.queue = songs;
        inner.shuffle_enabled = false;
        inner.shuffle_played.clear();
    }
    play_at(&app, &queue_state, &app_state, &audio_player, start_idx).await
}

/// Append songs to the end of the current queue and start playing the first
/// appended track. Computes the insertion point under the lock so it can't race
/// a `queue-state-changed` event (used by Smart Radio auto-continue).
#[tauri::command]
pub async fn append_and_play(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    songs: Vec<Song>,
) -> Result<(), String> {
    if songs.is_empty() { return Ok(()); }
    let start_idx = {
        let mut inner = queue_state.inner.lock();
        let start = inner.queue.len();
        inner.queue.extend(songs);
        start
    };
    play_at(&app, &queue_state, &app_state, &audio_player, start_idx).await
}

/// Replace queue without interrupting playback if the current track is still present.
#[tauri::command]
pub async fn set_queue_seamless(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    songs: Vec<Song>,
    start_idx: usize,
) -> Result<(), String> {
    let current_track_id: Option<String> = {
        let inner = queue_state.inner.lock();
        inner.queue.get(inner.queue_idx as usize).map(|s| s.id.clone())
    };

    let match_idx = current_track_id.as_deref()
        .and_then(|id| songs.iter().position(|s| s.id == id));

    {
        let mut inner = queue_state.inner.lock();
        inner.queue = songs;
        inner.shuffle_played.clear();
    }

    if let Some(mid) = match_idx {
        // Current track is still in the new queue — just update the index, keep playing
        {
            let mut inner = queue_state.inner.lock();
            inner.queue_idx = mid as i32;
        }
        emit_queue_state(&app, &queue_state);
        Ok(())
    } else {
        play_at(&app, &queue_state, &app_state, &audio_player, start_idx).await
    }
}

/// Fisher-Yates shuffle then play index 0.
#[tauri::command]
pub async fn shuffle_and_play(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    songs: Vec<Song>,
) -> Result<(), String> {
    let mut shuffled = songs;
    fisher_yates(&mut shuffled);
    {
        let mut inner = queue_state.inner.lock();
        inner.queue = shuffled;
        inner.shuffle_enabled = true;
        inner.shuffle_played.clear();
    }
    play_at(&app, &queue_state, &app_state, &audio_player, 0).await
}

/// Jump to a specific queue position.
#[tauri::command]
pub async fn play_queue_index(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    idx: usize,
) -> Result<(), String> {
    play_at(&app, &queue_state, &app_state, &audio_player, idx).await
}

/// Next track — shuffle-aware, repeat-all-aware.
#[tauri::command]
pub async fn queue_next(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    let (queue_idx, queue_len, shuffle, repeat_all) = {
        let inner = queue_state.inner.lock();
        (inner.queue_idx, inner.queue.len(), inner.shuffle_enabled, inner.repeat_all)
    };

    let next_idx = if shuffle && queue_len > 1 {
        let mut inner = queue_state.inner.lock();
        match next_shuffle_idx(queue_len, queue_idx as usize, &mut inner.shuffle_played, repeat_all) {
            Some(idx) => idx,
            None => return Ok(()),
        }
    } else if let Some(idx) = compute_next_idx(queue_idx, queue_len, repeat_all) {
        idx
    } else {
        return Ok(());
    };

    play_at(&app, &queue_state, &app_state, &audio_player, next_idx).await
}

/// Previous track — seek to 0 if position > 3s, else go to previous index.
#[tauri::command]
pub async fn queue_prev(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    let (queue_idx, current_player_id) = {
        let inner = queue_state.inner.lock();
        (inner.queue_idx, inner.current_player_id.clone())
    };

    // Seek to start if current position > 3s
    if let Some(ref pid) = current_player_id {
        if let Ok(pos) = audio_player.get_current_position(pid) {
            if pos > 3.0 {
                return audio_player.seek(pid, 0.0);
            }
        }
    }

    if queue_idx > 0 {
        play_at(&app, &queue_state, &app_state, &audio_player, (queue_idx - 1) as usize).await
    } else {
        Ok(())
    }
}

/// Toggle play/pause, or start playback at the current queue index if stopped.
#[tauri::command]
pub async fn toggle_play(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    app_state: State<'_, Arc<AppState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    let (queue_idx, current_player_id) = {
        let inner = queue_state.inner.lock();
        (inner.queue_idx, inner.current_player_id.clone())
    };

    if let Some(ref pid) = current_player_id {
        match audio_player.get_state(pid) {
            Ok(crate::PlaybackState::Playing) => {
                audio_player.pause(pid)?;
                if let Some(ref apid) = current_player_id {
                    fire_report_playback(app, Arc::clone(&app_state),
                        queue_state.inner.lock().queue.get(queue_idx as usize).map(|s| s.id.clone()).unwrap_or_default(),
                        (audio_player.get_current_position(apid).unwrap_or(0.0) * 1000.0) as i64,
                        "paused".into());
                }
                return Ok(());
            }
            Ok(crate::PlaybackState::Paused) => {
                audio_player.resume(pid)?;
                fire_report_playback(app, Arc::clone(&app_state),
                    queue_state.inner.lock().queue.get(queue_idx as usize).map(|s| s.id.clone()).unwrap_or_default(),
                    0, "playing".into());
                return Ok(());
            }
            _ => {}
        }
    }

    // Not playing/paused — start at current queue index
    if queue_idx >= 0 {
        play_at(&app, &queue_state, &app_state, &audio_player, queue_idx as usize).await
    } else {
        Ok(())
    }
}

/// Seek the current player to `position` seconds.
#[tauri::command]
pub fn seek_queue(
    queue_state: State<'_, Arc<QueueState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    position: f64,
) -> Result<(), String> {
    let pid = queue_state.inner.lock().current_player_id.clone()
        .ok_or("No active player")?;
    audio_player.seek(&pid, position)
}

/// Sets the global volume and applies it to the active player session.
#[tauri::command]
pub fn set_queue_volume(
    app: AppHandle,
    queue_state: State<'_, Arc<QueueState>>,
    audio_player: State<'_, Arc<AudioPlayer>>,
    volume: f32,
) -> Result<(), String> {
    let volume = volume.clamp(0.0, 1.0);
    let pid = {
        let mut inner = queue_state.inner.lock();
        inner.volume = volume;
        inner.current_player_id.clone()
    };
    if let Some(ref pid) = pid {
        let _ = audio_player.set_volume(pid, volume);
    }
    emit_queue_state(&app, &queue_state);
    Ok(())
}
