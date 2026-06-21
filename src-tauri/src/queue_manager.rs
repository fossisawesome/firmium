/// Background task that listens to playback events and drives crossfade, gapless
/// preload, track-advance, and scrobbling — all decisions that previously lived in
/// TypeScript `playback.ts`.
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Listener};

use crate::audio::AudioPlayer;
use crate::commands::listenbrainz::fire_listenbrainz_listen;
use crate::commands::queue::{compute_next_idx, has_shuffle_next, next_shuffle_idx, play_at, schedule_save_play_queue, stream_url_for};
use crate::commands::subsonic::{fire_report_playback, fire_scrobble};
use crate::db::fire_record_play;
use crate::queue_state::{emit_queue_state, QueueState};
use crate::state::AppState;

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PositionPayload {
    player_id: String,
    position: f64,
    duration: f64,
}

struct CrossfadeContext {
    next_idx: usize,
    old_player_id: String,
    fade_ms: u64,
    volume: f32,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct FinishedPayload {
    player_id: String,
}

pub fn start(
    app: AppHandle,
    queue_state: Arc<QueueState>,
    app_state: Arc<AppState>,
    audio_player: Arc<AudioPlayer>,
) {
    // ── Position events ───────────────────────────────────────────────────────
    {
        let qs = Arc::clone(&queue_state);
        let as_ = Arc::clone(&app_state);
        let ap = Arc::clone(&audio_player);
        let app2 = app.clone();
        app.listen("playback-position", move |event| {
            let payload: PositionPayload = match serde_json::from_str(event.payload()) {
                Ok(p) => p,
                Err(_) => return,
            };
            handle_position(&app2, &qs, &as_, &ap, payload);
        });
    }

    // ── Finished events ───────────────────────────────────────────────────────
    {
        let qs = Arc::clone(&queue_state);
        let as_ = Arc::clone(&app_state);
        let ap = Arc::clone(&audio_player);
        let app3 = app.clone();
        app.listen("playback-finished", move |event| {
            let payload: FinishedPayload = match serde_json::from_str(event.payload()) {
                Ok(p) => p,
                Err(_) => return,
            };
            let qs = Arc::clone(&qs);
            let as_ = Arc::clone(&as_);
            let ap = Arc::clone(&ap);
            let app4 = app3.clone();
            tauri::async_runtime::spawn(async move {
                handle_finished(app4, qs, as_, ap, payload).await;
            });
        });
    }
}

fn should_start_crossfade(
    position: f64,
    duration: f64,
    crossfade_secs: f32,
    crossfade_enabled: bool,
    crossfade_started: bool,
    repeat_one: bool,
    has_next: bool,
) -> bool {
    !crossfade_started
        && crossfade_enabled
        && !repeat_one
        && has_next
        && duration > 0.0
        && position >= duration - crossfade_secs as f64
}

fn should_start_preload(
    position: f64,
    duration: f64,
    preload_started: bool,
    gapless_enabled: bool,
    crossfade_enabled: bool,
    repeat_one: bool,
    has_next: bool,
) -> bool {
    !preload_started
        && gapless_enabled
        && !crossfade_enabled
        && !repeat_one
        && has_next
        && duration > 0.0
        && position >= (duration - 30.0).max(0.0)
}

fn handle_position(
    app: &AppHandle,
    queue_state: &Arc<QueueState>,
    app_state: &Arc<AppState>,
    audio_player: &Arc<AudioPlayer>,
    payload: PositionPayload,
) {
    // Guard: only process events for the current player
    let (
        current_player_id,
        crossfade_enabled,
        crossfade_started,
        gapless_enabled,
        preload_started,
        repeat_one,
        repeat_all,
        shuffle,
        queue_len,
        queue_idx,
        crossfade_secs,
        volume,
        cached_duration,
        last_save_pos,
        shuffle_played,
    ) = {
        let mut inner = queue_state.inner.lock();
        if inner.current_player_id.as_deref() != Some(payload.player_id.as_str()) {
            return;
        }
        // Cache duration on first tick
        if inner.cached_duration.is_none() && payload.duration > 0.0 {
            inner.cached_duration = Some(payload.duration);
        }
        (
            inner.current_player_id.clone(),
            inner.crossfade_enabled,
            inner.crossfade_started,
            inner.gapless_enabled,
            inner.preload_started,
            inner.repeat_one,
            inner.repeat_all,
            inner.shuffle_enabled,
            inner.queue.len(),
            inner.queue_idx,
            inner.crossfade_duration,
            inner.volume,
            inner.cached_duration,
            inner.last_queue_save_position,
            inner.shuffle_played.clone(),
        )
    }; // lock released

    let position = payload.position;
    let dur = match cached_duration { Some(d) => d, None => return };

    // Throttled play-queue save every 30s
    if position - last_save_pos >= 30.0 {
        queue_state.inner.lock().last_queue_save_position = position;
        schedule_save_play_queue(app, app_state, queue_state, current_player_id.clone(), audio_player);
    }

    // Crossfade trigger. Gate on whether a next track exists without consuming
    // the shuffle pass; the actual index is picked once, below.
    let cf_has_next = if shuffle && queue_len > 1 {
        has_shuffle_next(queue_len, queue_idx as usize, &shuffle_played, repeat_all)
    } else {
        compute_next_idx(queue_idx, queue_len, repeat_all).is_some()
    };
    if should_start_crossfade(position, dur, crossfade_secs, crossfade_enabled, crossfade_started, repeat_one, cf_has_next) {
        // Pick (and consume, for shuffle) the next index, marking crossfade_started
        // under the same lock so do_crossfade uses exactly this index.
        let next_idx = {
            let mut inner = queue_state.inner.lock();
            let idx = if shuffle && queue_len > 1 {
                next_shuffle_idx(queue_len, queue_idx as usize, &mut inner.shuffle_played, repeat_all)
            } else {
                compute_next_idx(queue_idx, queue_len, repeat_all)
            };
            if idx.is_some() {
                inner.crossfade_started = true;
            }
            idx
        };
        if let Some(next_idx) = next_idx {
            let qs = Arc::clone(queue_state);
            let as_ = Arc::clone(app_state);
            let ap = Arc::clone(audio_player);
            let app2 = app.clone();
            let old_pid = current_player_id.unwrap_or_default();
            let fade_ms = (crossfade_secs * 1000.0) as u64;
            tauri::async_runtime::spawn(async move {
                do_crossfade(app2, qs, as_, ap, CrossfadeContext { next_idx, old_player_id: old_pid, fade_ms, volume }).await;
            });
        }
    }

    // Gapless preload trigger
    let preload_next_idx = compute_next_idx(queue_idx, queue_len, repeat_all);
    if should_start_preload(position, dur, preload_started, gapless_enabled, crossfade_enabled, repeat_one, preload_next_idx.is_some()) {
        let next_idx = preload_next_idx.unwrap();
        queue_state.inner.lock().preload_started = true;
        let qs = Arc::clone(queue_state);
        let as_ = Arc::clone(app_state);
        let ap = Arc::clone(audio_player);
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            do_gapless_preload(app2, qs, as_, ap, next_idx).await;
        });
    }
}

async fn handle_finished(
    app: AppHandle,
    queue_state: Arc<QueueState>,
    app_state: Arc<AppState>,
    audio_player: Arc<AudioPlayer>,
    payload: FinishedPayload,
) {
    let (repeat_one, repeat_all, shuffle, queue_idx, queue_len, finished_song, cached_dur, auto_continue) = {
        let inner = queue_state.inner.lock();
        if inner.current_player_id.as_deref() != Some(payload.player_id.as_str()) {
            return; // stale event — a new track already started
        }
        (
            inner.repeat_one,
            inner.repeat_all,
            inner.shuffle_enabled,
            inner.queue_idx,
            inner.queue.len(),
            inner.queue.get(inner.queue_idx as usize).cloned(),
            inner.cached_duration,
            inner.auto_continue,
        )
    };

    // Scrobble completion (Subsonic + ListenBrainz + local play history)
    if let Some(song) = finished_song.clone() {
        let dur_ms = (cached_dur.unwrap_or(0.0) * 1000.0) as i64;
        fire_scrobble(app.clone(), Arc::clone(&app_state), song.id.clone(), true);
        fire_report_playback(app.clone(), Arc::clone(&app_state), song.id.clone(), dur_ms, "stopped".into());
        fire_record_play(&app, &song, dur_ms / 1000);
        fire_listenbrainz_listen(Arc::clone(&app_state), song);
    }

    // Determine next action
    if repeat_one {
        let _ = play_at(&app, &queue_state, &app_state, &audio_player, queue_idx as usize).await;
        return;
    }

    let next_idx = if shuffle && queue_len > 1 {
        let mut inner = queue_state.inner.lock();
        next_shuffle_idx(queue_len, queue_idx as usize, &mut inner.shuffle_played, repeat_all)
    } else {
        compute_next_idx(queue_idx, queue_len, repeat_all)
    };

    if let Some(next_idx) = next_idx {
        let _ = play_at(&app, &queue_state, &app_state, &audio_player, next_idx).await;
    } else {
        // End of queue
        {
            let mut inner = queue_state.inner.lock();
            inner.current_player_id = None;
            inner.queue_idx = -1;
        }
        let _ = app.emit("queue-state-changed", queue_state.inner.lock().snapshot());
        // Smart Radio: ask the frontend to seed and append more tracks from the
        // last-played song. The frontend owns the seeding cascade (shared with
        // Mood Mix and Start Radio) and appends via set_queue.
        if auto_continue {
            if let Some(song) = finished_song {
                let _ = app.emit("queue-exhausted", song);
            }
        }
    }
}

async fn do_crossfade(
    app: AppHandle,
    queue_state: Arc<QueueState>,
    app_state: Arc<AppState>,
    audio_player: Arc<AudioPlayer>,
    ctx: CrossfadeContext,
) {
    let CrossfadeContext { next_idx, old_player_id, fade_ms, volume } = ctx;
    // Extract next song, update queue_idx, reset per-track flags
    let (song, outgoing, rg_enabled, curve) = {
        let mut inner = queue_state.inner.lock();
        let song = match inner.queue.get(next_idx).cloned() {
            Some(s) => s,
            None => return,
        };
        let outgoing = inner.queue.get(inner.queue_idx as usize).cloned();
        inner.queue_idx = next_idx as i32;
        inner.reset_track_progress();
        (song, outgoing, inner.replay_gain_enabled, inner.crossfade_curve.clone())
    };

    // Scrobble outgoing (Subsonic + ListenBrainz + local play history)
    if let Some(outgoing) = outgoing {
        fire_scrobble(app.clone(), Arc::clone(&app_state), outgoing.id.clone(), true);
        fire_report_playback(app.clone(), Arc::clone(&app_state), outgoing.id.clone(), 0, "stopped".into());
        fire_record_play(&app, &outgoing, outgoing.duration as i64);
        fire_listenbrainz_listen(Arc::clone(&app_state), outgoing);
    }

    // Resolve URL — async, no lock held
    let stream_url = match stream_url_for(&song, &app, &app_state).await {
        Ok(u) => u,
        Err(e) => { eprintln!("Crossfade stream URL error: {e}"); return; }
    };

    // Do crossfade — no lock held
    let rg = if rg_enabled { crate::commands::queue::replay_gain_db_pub(&song) } else { None };
    let new_pid = match AudioPlayer::crossfade_to(&audio_player, &old_player_id, &stream_url, song.id.clone(), fade_ms, volume, rg, &curve) {
        Ok(pid) => pid,
        Err(e) => { eprintln!("Crossfade failed: {e}"); return; }
    };

    // Store new player_id
    { queue_state.inner.lock().current_player_id = Some(new_pid.clone()); }

    emit_queue_state(&app, &queue_state);
    fire_scrobble(app.clone(), Arc::clone(&app_state), song.id.clone(), false);
    fire_report_playback(app.clone(), Arc::clone(&app_state), song.id.clone(), 0, "starting".into());
    schedule_save_play_queue(&app, &app_state, &queue_state, Some(new_pid), &audio_player);
}

async fn do_gapless_preload(
    app: AppHandle,
    queue_state: Arc<QueueState>,
    app_state: Arc<AppState>,
    audio_player: Arc<AudioPlayer>,
    next_idx: usize,
) {
    let (song, rg_enabled) = {
        let inner = queue_state.inner.lock();
        let song = inner.queue.get(next_idx).cloned();
        (song, inner.replay_gain_enabled)
    };
    let song = match song { Some(s) => s, None => return };

    let stream_url = match stream_url_for(&song, &app, &app_state).await {
        Ok(u) => u,
        Err(e) => { eprintln!("Gapless preload URL error: {e}"); return; }
    };

    let rg = if rg_enabled { crate::commands::queue::replay_gain_db_pub(&song) } else { None };
    match AudioPlayer::preload_stream(&audio_player, &stream_url, song.id.clone(), rg) {
        Ok(preload_pid) => {
            let mut inner = queue_state.inner.lock();
            inner.preloaded_player_id = Some(preload_pid);
            inner.preloaded_track_id = Some(song.id);
        }
        Err(e) => eprintln!("Gapless preload failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{should_start_crossfade, should_start_preload};

    // ── should_start_crossfade ────────────────────────────────────────────────

    #[test]
    fn crossfade_triggers_when_all_conditions_met() {
        assert!(should_start_crossfade(57.0, 60.0, 5.0, true, false, false, true));
    }

    #[test]
    fn crossfade_false_when_already_started() {
        assert!(!should_start_crossfade(57.0, 60.0, 5.0, true, true, false, true));
    }

    #[test]
    fn crossfade_false_when_disabled() {
        assert!(!should_start_crossfade(57.0, 60.0, 5.0, false, false, false, true));
    }

    #[test]
    fn crossfade_false_when_too_early() {
        assert!(!should_start_crossfade(50.0, 60.0, 5.0, true, false, false, true));
    }

    #[test]
    fn crossfade_false_when_repeat_one() {
        assert!(!should_start_crossfade(57.0, 60.0, 5.0, true, false, true, true));
    }

    #[test]
    fn crossfade_false_when_no_next() {
        assert!(!should_start_crossfade(57.0, 60.0, 5.0, true, false, false, false));
    }

    #[test]
    fn crossfade_false_when_duration_zero() {
        assert!(!should_start_crossfade(0.0, 0.0, 5.0, true, false, false, true));
    }

    // ── should_start_preload ──────────────────────────────────────────────────

    #[test]
    fn preload_triggers_when_all_conditions_met() {
        assert!(should_start_preload(35.0, 60.0, false, true, false, false, true));
    }

    #[test]
    fn preload_false_when_already_started() {
        assert!(!should_start_preload(35.0, 60.0, true, true, false, false, true));
    }

    #[test]
    fn preload_false_when_gapless_disabled() {
        assert!(!should_start_preload(35.0, 60.0, false, false, false, false, true));
    }

    #[test]
    fn preload_false_when_crossfade_enabled() {
        assert!(!should_start_preload(35.0, 60.0, false, true, true, false, true));
    }

    #[test]
    fn preload_false_when_repeat_one() {
        assert!(!should_start_preload(35.0, 60.0, false, true, false, true, true));
    }

    #[test]
    fn preload_false_when_no_next() {
        assert!(!should_start_preload(35.0, 60.0, false, true, false, false, false));
    }

    #[test]
    fn preload_false_when_too_early() {
        assert!(!should_start_preload(25.0, 60.0, false, true, false, false, true));
    }

    #[test]
    fn preload_short_track_threshold_clamps_to_zero() {
        // duration=20 → threshold = (20-30).max(0) = 0 → triggers at position=0
        assert!(should_start_preload(0.0, 20.0, false, true, false, false, true));
    }
}
