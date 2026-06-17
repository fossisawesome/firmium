/// Background task that listens to playback events and drives crossfade, gapless
/// preload, track-advance, and scrobbling — all decisions that previously lived in
/// TypeScript `playback.ts`.
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Listener};

use crate::audio::AudioPlayer;
use crate::commands::queue::{compute_next_idx, play_at, random_idx_excluding, schedule_save_play_queue, stream_url_for};
use crate::commands::subsonic::{fire_report_playback, fire_scrobble};
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
        )
    }; // lock released

    let position = payload.position;
    let dur = match cached_duration { Some(d) => d, None => return };

    // Throttled play-queue save every 30s
    if position - last_save_pos >= 30.0 {
        queue_state.inner.lock().last_queue_save_position = position;
        schedule_save_play_queue(app, app_state, queue_state, current_player_id.clone(), audio_player);
    }

    // Crossfade trigger
    if !crossfade_started && crossfade_enabled && !repeat_one && dur > 0.0
        && position >= dur - crossfade_secs as f64
    {
        let next_idx = if shuffle && queue_len > 1 {
            Some(random_idx_excluding(queue_len, queue_idx as usize))
        } else {
            compute_next_idx(queue_idx, queue_len, repeat_all)
        };
        if let Some(next_idx) = next_idx {
            queue_state.inner.lock().crossfade_started = true;
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

    // Gapless preload trigger (30s before end, crossfade off)
    if !preload_started && gapless_enabled && !crossfade_enabled && !repeat_one && dur > 0.0 {
        let preload_at = (dur - 30.0).max(0.0);
        if position >= preload_at {
            let next_idx = compute_next_idx(queue_idx, queue_len, repeat_all);
            if let Some(next_idx) = next_idx {
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
    }
}

async fn handle_finished(
    app: AppHandle,
    queue_state: Arc<QueueState>,
    app_state: Arc<AppState>,
    audio_player: Arc<AudioPlayer>,
    payload: FinishedPayload,
) {
    let (repeat_one, repeat_all, shuffle, queue_idx, queue_len, finished_track_id, cached_dur) = {
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
            inner.queue.get(inner.queue_idx as usize).map(|s| s.id.clone()),
            inner.cached_duration,
        )
    };

    // Scrobble completion
    if let Some(track_id) = finished_track_id {
        let dur_ms = (cached_dur.unwrap_or(0.0) * 1000.0) as i64;
        fire_scrobble(app.clone(), Arc::clone(&app_state), track_id.clone(), true);
        fire_report_playback(app.clone(), Arc::clone(&app_state), track_id, dur_ms, "stopped".into());
    }

    // Determine next action
    if repeat_one {
        let _ = play_at(&app, &queue_state, &app_state, &audio_player, queue_idx as usize).await;
    } else if shuffle && queue_len > 1 {
        let next = random_idx_excluding(queue_len, queue_idx as usize);
        let _ = play_at(&app, &queue_state, &app_state, &audio_player, next).await;
    } else if let Some(next_idx) = compute_next_idx(queue_idx, queue_len, repeat_all) {
        let _ = play_at(&app, &queue_state, &app_state, &audio_player, next_idx).await;
    } else {
        // End of queue — stop
        {
            let mut inner = queue_state.inner.lock();
            inner.current_player_id = None;
            inner.queue_idx = -1;
        }
        let _ = app.emit("queue-state-changed", queue_state.inner.lock().snapshot());
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
    let (song, outgoing_id) = {
        let mut inner = queue_state.inner.lock();
        let song = match inner.queue.get(next_idx).cloned() {
            Some(s) => s,
            None => return,
        };
        let outgoing_id = inner.queue.get(inner.queue_idx as usize).map(|s| s.id.clone());
        inner.queue_idx = next_idx as i32;
        inner.reset_track_progress();
        (song, outgoing_id)
    };

    // Scrobble outgoing
    if let Some(oid) = outgoing_id {
        fire_scrobble(app.clone(), Arc::clone(&app_state), oid.clone(), true);
        fire_report_playback(app.clone(), Arc::clone(&app_state), oid, 0, "stopped".into());
    }

    // Resolve URL — async, no lock held
    let stream_url = match stream_url_for(&song, &app, &app_state).await {
        Ok(u) => u,
        Err(e) => { eprintln!("Crossfade stream URL error: {e}"); return; }
    };

    // Do crossfade — no lock held
    let rg = crate::commands::queue::replay_gain_db_pub(&song);
    let new_pid = match AudioPlayer::crossfade_to(&audio_player, &old_player_id, &stream_url, song.id.clone(), fade_ms, volume, rg) {
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
    let song = {
        let inner = queue_state.inner.lock();
        inner.queue.get(next_idx).cloned()
    };
    let song = match song { Some(s) => s, None => return };

    let stream_url = match stream_url_for(&song, &app, &app_state).await {
        Ok(u) => u,
        Err(e) => { eprintln!("Gapless preload URL error: {e}"); return; }
    };

    let rg = crate::commands::queue::replay_gain_db_pub(&song);
    match AudioPlayer::preload_stream(&audio_player, &stream_url, song.id.clone(), rg) {
        Ok(preload_pid) => {
            let mut inner = queue_state.inner.lock();
            inner.preloaded_player_id = Some(preload_pid);
            inner.preloaded_track_id = Some(song.id);
        }
        Err(e) => eprintln!("Gapless preload failed: {e}"),
    }
}
