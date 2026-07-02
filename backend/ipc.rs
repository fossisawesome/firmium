//! CLI control socket. A running instance listens for line-based commands from
//! the `firmium <cmd>` CLI invocation (see `src/main.rs`) and drives playback
//! by calling the same `commands::queue` fns the UI's `update_transport` uses
//! — no `Message`/`Task` round-trip needed since these fns only take backend
//! Arc handles, not `App` state.
//!
//! Linux/macOS: Unix domain socket. Windows: named pipe. One command per
//! connection: client writes a line, server writes one reply line, closes.

use std::sync::Arc;

use crate::audio::AudioPlayer;
use crate::events::EventBus;
use crate::queue_state::QueueState;
use crate::state::AppState;

/// Linux/macOS: `$XDG_RUNTIME_DIR/firmium.sock`, falling back to the app cache
/// dir if the runtime dir isn't set (e.g. some display managers, CI).
#[cfg(unix)]
pub fn socket_path() -> std::path::PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) => std::path::PathBuf::from(dir).join("firmium.sock"),
        None => crate::paths::cache_dir().join("firmium.sock"),
    }
}

#[cfg(windows)]
pub const PIPE_NAME: &str = r"\\.\pipe\firmium-com.fossisawesome.firmium";

/// Starts the IPC listener as a background task. Must be called from within a
/// Tokio runtime (mirrors `queue_manager::start`).
pub fn start(bus: EventBus, queue_state: Arc<QueueState>, app_state: Arc<AppState>, audio_player: Arc<AudioPlayer>) {
    tokio::spawn(async move {
        if let Err(e) = run(bus, queue_state, app_state, audio_player).await {
            eprintln!("IPC listener failed to start: {e}");
        }
    });
}

#[cfg(unix)]
async fn run(bus: EventBus, queue_state: Arc<QueueState>, app_state: Arc<AppState>, audio_player: Arc<AudioPlayer>) -> Result<(), String> {
    use tokio::net::UnixListener;

    let path = socket_path();
    // Stale socket from a crashed previous run — a live instance would have
    // held this listener open, so removing it here is safe.
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|e| format!("bind {}: {e}", path.display()))?;

    loop {
        let Ok((stream, _)) = listener.accept().await else { continue };
        tokio::spawn(handle_conn(stream, bus.clone(), queue_state.clone(), app_state.clone(), audio_player.clone()));
    }
}

#[cfg(windows)]
async fn run(bus: EventBus, queue_state: Arc<QueueState>, app_state: Arc<AppState>, audio_player: Arc<AudioPlayer>) -> Result<(), String> {
    use tokio::net::windows::named_pipe::ServerOptions;

    loop {
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(PIPE_NAME)
            .map_err(|e| format!("create pipe {PIPE_NAME}: {e}"))?;
        server.connect().await.map_err(|e| format!("pipe connect: {e}"))?;
        tokio::spawn(handle_conn(server, bus.clone(), queue_state.clone(), app_state.clone(), audio_player.clone()));
    }
}

async fn handle_conn<S>(stream: S, bus: EventBus, queue_state: Arc<QueueState>, app_state: Arc<AppState>, audio_player: Arc<AudioPlayer>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut lines = BufReader::new(read_half).lines();
    let Ok(Some(line)) = lines.next_line().await else { return };

    let reply = dispatch(&line, &bus, &queue_state, &app_state, &audio_player).await;
    let _ = write_half.write_all(format!("{reply}\n").as_bytes()).await;
}

async fn dispatch(line: &str, bus: &EventBus, queue_state: &Arc<QueueState>, app_state: &Arc<AppState>, audio_player: &Arc<AudioPlayer>) -> String {
    let mut parts = line.trim().splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let arg = parts.next();

    let result = match cmd {
        "play-pause" => crate::commands::queue::toggle_play(queue_state.clone(), app_state.clone(), audio_player.clone()).await,
        "next" => crate::commands::queue::queue_next(queue_state.clone(), app_state.clone(), audio_player.clone()).await,
        "prev" => crate::commands::queue::queue_prev(queue_state.clone(), app_state.clone(), audio_player.clone()).await,
        "volume" => match arg.and_then(|a| a.parse::<f32>().ok()) {
            Some(pct) => {
                crate::commands::queue::set_queue_volume(bus, queue_state, audio_player, pct / 100.0);
                Ok(())
            }
            None => Err("usage: volume <0-100>".to_string()),
        },
        "seek" => match arg.and_then(|a| a.parse::<f64>().ok()) {
            Some(secs) => crate::commands::queue::seek_queue(queue_state, audio_player, secs),
            None => Err("usage: seek <seconds>".to_string()),
        },
        "queue-index" => match arg.and_then(|a| a.parse::<usize>().ok()) {
            Some(idx) => crate::commands::queue::play_queue_index(queue_state.clone(), app_state.clone(), audio_player.clone(), idx).await,
            None => Err("usage: queue-index <n>".to_string()),
        },
        "shuffle-toggle" => {
            crate::commands::queue::toggle_shuffle(bus, queue_state);
            Ok(())
        }
        "repeat-cycle" => {
            let (one, all) = {
                let inner = queue_state.inner.lock();
                if !inner.repeat_one && !inner.repeat_all {
                    (false, true)
                } else if inner.repeat_all {
                    (true, false)
                } else {
                    (false, false)
                }
            };
            crate::commands::queue::set_repeat_mode(bus, queue_state, one, all);
            Ok(())
        }
        "status" => return status_json(queue_state, audio_player),
        other => Err(format!("unknown command: {other}")),
    };

    match result {
        Ok(()) => "ok".to_string(),
        Err(e) => format!("error: {e}"),
    }
}

fn status_json(queue_state: &QueueState, audio_player: &AudioPlayer) -> String {
    let (song, queue_idx, player_id, repeat_one, repeat_all, shuffle, volume) = {
        let inner = queue_state.inner.lock();
        (
            inner.queue.get(inner.queue_idx.max(0) as usize).cloned(),
            inner.queue_idx,
            inner.current_player_id.clone(),
            inner.repeat_one,
            inner.repeat_all,
            inner.shuffle_enabled,
            inner.volume,
        )
    };

    let (state, position, duration) = match player_id.as_deref() {
        Some(pid) => (
            audio_player.get_state(pid).ok(),
            audio_player.get_current_position(pid).ok(),
            audio_player.get_duration(pid).ok().flatten(),
        ),
        None => (None, None, None),
    };

    let payload = serde_json::json!({
        "playing": matches!(state, Some(crate::PlaybackState::Playing)),
        "track": song.map(|s| serde_json::json!({ "id": s.id, "title": s.title, "artist": s.artist })),
        "queueIndex": queue_idx,
        "position": position,
        "duration": duration,
        "volume": volume,
        "shuffle": shuffle,
        "repeatOne": repeat_one,
        "repeatAll": repeat_all,
    });
    payload.to_string()
}
