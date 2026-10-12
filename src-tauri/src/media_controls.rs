//! OS media controls: MPRIS on Linux, SMTC on Windows, Now Playing on macOS (via `souvlaki`).
//! Mirrors backend playback state out to the OS and routes media keys / headset buttons
//! back into the same queue commands the control socket uses.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use firmium_backend::commands::queue;
use firmium_backend::core::Core;
use firmium_backend::events::BackendEvent;
use firmium_backend::PlaybackState;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use tauri::AppHandle;
#[cfg(windows)]
use tauri::Manager;

pub fn start(app: &AppHandle, core: &Arc<Core>) {
    // Windows needs the window handle; other platforms ignore it.
    #[cfg(windows)]
    let hwnd = app.get_webview_window("main").and_then(|w| w.hwnd().ok()).map(|h| h.0 as *mut std::ffi::c_void);
    #[cfg(not(windows))]
    let hwnd = None;
    let _ = app;

    let mut controls = match MediaControls::new(PlatformConfig {
        dbus_name: "firmium",
        display_name: "Firmium",
        hwnd,
    }) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("media controls unavailable: {e:?}");
            return;
        }
    };

    let playing = Arc::new(AtomicBool::new(false));
    let handler_core = Arc::clone(core);
    let handler_playing = Arc::clone(&playing);
    let attached = controls.attach(move |event| {
        let core = Arc::clone(&handler_core);
        let is_playing = handler_playing.load(Ordering::Relaxed);
        tauri::async_runtime::spawn(async move {
            let b = &core.backend;
            let toggle = || queue::toggle_play(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone());
            let result = match event {
                MediaControlEvent::Toggle => toggle().await,
                MediaControlEvent::Play if !is_playing => toggle().await,
                MediaControlEvent::Pause | MediaControlEvent::Stop if is_playing => toggle().await,
                MediaControlEvent::Next => queue::queue_next(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone()).await,
                MediaControlEvent::Previous => queue::queue_prev(b.queue_state.clone(), b.app_state.clone(), b.audio_player.clone()).await,
                MediaControlEvent::SetPosition(MediaPosition(pos)) => queue::seek_queue(&b.queue_state, &b.audio_player, pos.as_secs_f64()),
                _ => Ok(()),
            };
            if let Err(e) = result {
                eprintln!("media control action failed: {e}");
            }
        });
    });
    if let Err(e) = attached {
        eprintln!("media controls attach failed: {e:?}");
        return;
    }

    let mut rx = core.backend.bus.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut position = Duration::ZERO;
        loop {
            let ev = match rx.recv().await {
                Ok(ev) => ev,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            };
            match ev {
                BackendEvent::QueueStateChanged(s) => {
                    if let Some(song) = usize::try_from(s.queue_idx).ok().and_then(|i| s.queue.get(i)) {
                        let _ = controls.set_metadata(MediaMetadata {
                            title: Some(&song.title),
                            artist: Some(&song.artist),
                            album: Some(&song.album),
                            cover_url: None,
                            duration: Some(Duration::from_secs_f64(song.duration.max(0.0))),
                        });
                    }
                }
                BackendEvent::PlaybackPosition { position: p, .. } => position = Duration::from_secs_f64(p.max(0.0)),
                BackendEvent::PlaybackStateChanged { state, .. } => {
                    playing.store(state == PlaybackState::Playing, Ordering::Relaxed);
                    let progress = Some(MediaPosition(position));
                    let _ = controls.set_playback(match state {
                        PlaybackState::Playing => MediaPlayback::Playing { progress },
                        PlaybackState::Paused => MediaPlayback::Paused { progress },
                        PlaybackState::Loading | PlaybackState::Stopped => MediaPlayback::Stopped,
                    });
                }
                _ => {}
            }
        }
    });
}
