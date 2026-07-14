use std::io;
use std::sync::Arc;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod keymap;
mod ui;

use app::{App, AppEvent, SideEffect, View};
use keymap::Keymap;

#[tokio::main]
async fn main() -> io::Result<()> {
    let keymap = Keymap::load();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, keymap).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    keymap: Keymap,
) -> io::Result<()> {
    let mut app = App::new();
    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));

    // Results of spawned backend calls come back on this channel as AppEvents.
    let (result_tx, mut result_rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

    loop {
        terminal.draw(|frame| ui::render(&app, frame))?;

        if app.should_quit {
            break;
        }

        let mut backend_recv = app.backend.as_ref().map(|b| b.bus.subscribe());

        let effect = tokio::select! {
            Some(Ok(Event::Key(key))) = events.next() => {
                handle_key(&mut app, &keymap, key)
            }
            Ok(backend_event) = async {
                match &mut backend_recv {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                app.update(AppEvent::Backend(backend_event))
            }
            Some(result_event) = result_rx.recv() => {
                app.update(result_event)
            }
            _ = tick.tick() => {
                app.update(AppEvent::Tick)
            }
        };

        if let Some(effect) = effect {
            perform_side_effect(&app, effect, result_tx.clone());
        }
    }
    Ok(())
}

/// Text-entry views (Login, and Search before it's submitted) route raw
/// characters into the input buffer instead of through keymap action
/// resolution; everywhere else, keys resolve via the keymap.
fn handle_key(app: &mut App, keymap: &Keymap, key: crossterm::event::KeyEvent) -> Option<SideEffect> {
    let typing_mode = matches!(app.view, View::Login)
        || (app.view == View::Search && !app.search_submitted);

    if typing_mode {
        match key.code {
            KeyCode::Char(c) => return app.update(AppEvent::CharInput(c)),
            KeyCode::Backspace => return app.update(AppEvent::Backspace),
            KeyCode::Tab if app.view == View::Login => return app.update(AppEvent::LoginFieldNext),
            KeyCode::Enter if app.view == View::Login => return app.update(AppEvent::LoginSubmit),
            KeyCode::Enter if app.view == View::Search => return app.update(AppEvent::SearchSubmit),
            KeyCode::Esc if app.view == View::Search => return app.update(AppEvent::SearchCancel),
            _ => return None,
        }
    }

    if let Some(action) = keymap.resolve(key) {
        return app.update(AppEvent::Key(action));
    }
    None
}

fn perform_side_effect(
    app: &App,
    effect: SideEffect,
    result_tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
) {
    match effect {
        SideEffect::Login { server, username, password } => {
            tokio::spawn(async move {
                let result = async {
                    let backend = firmium_backend::init::Backend::new()?;
                    firmium_backend::commands::subsonic::set_connection(
                        &backend.app_state,
                        Some(server.clone()),
                        Some(username.clone()),
                        Some(password.clone()),
                    );
                    firmium_backend::commands::subsonic::validate_connection(Arc::clone(&backend.app_state))
                        .await
                        .map_err(|e| e.message())?;
                    let _ = firmium_backend::commands::credentials::save_password(
                        Some(&server), &username, &password,
                    );
                    let mut cfg = firmium_backend::config::Config::load();
                    cfg.server = Some(server.clone());
                    cfg.username = Some(username.clone());
                    cfg.save();
                    Ok::<_, String>(Arc::new(backend))
                }
                .await;
                let _ = result_tx.send(AppEvent::LoginResult(result));
            });
        }
        SideEffect::TogglePlay | SideEffect::NextTrack | SideEffect::PrevTrack => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let queue_state = Arc::clone(&backend.queue_state);
                let app_state = Arc::clone(&backend.app_state);
                let audio_player = Arc::clone(&backend.audio_player);
                let _ = match effect {
                    SideEffect::TogglePlay => {
                        firmium_backend::commands::queue::toggle_play(queue_state, app_state, audio_player).await
                    }
                    SideEffect::NextTrack => {
                        firmium_backend::commands::queue::queue_next(queue_state, app_state, audio_player).await
                    }
                    SideEffect::PrevTrack => {
                        firmium_backend::commands::queue::queue_prev(queue_state, app_state, audio_player).await
                    }
                    _ => unreachable!(),
                };
            });
        }
        SideEffect::LoadHomeAlbums => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let app_state = Arc::clone(&backend.app_state);
                let result = firmium_backend::commands::subsonic::get_recent_albums(app_state, 50)
                    .await
                    .map_err(|e| e.message());
                let _ = result_tx.send(AppEvent::AlbumsLoaded(result));
            });
        }
        SideEffect::LoadArtists => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let app_state = Arc::clone(&backend.app_state);
                let result = firmium_backend::commands::subsonic::get_artists(app_state)
                    .await
                    .map_err(|e| e.message());
                let _ = result_tx.send(AppEvent::ArtistsLoaded(result));
            });
        }
        SideEffect::LoadPlaylists => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let app_state = Arc::clone(&backend.app_state);
                let result = firmium_backend::commands::subsonic::get_playlists(app_state)
                    .await
                    .map(|raw| {
                        raw.iter()
                            .map(|v| {
                                let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                (id, name)
                            })
                            .collect()
                    })
                    .map_err(|e| e.message());
                let _ = result_tx.send(AppEvent::PlaylistsLoaded(result));
            });
        }
        SideEffect::RunSearch(query) => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let app_state = Arc::clone(&backend.app_state);
                let result = firmium_backend::commands::subsonic::search(app_state, query)
                    .await
                    .map(|r| r.songs)
                    .map_err(|e| e.message());
                let _ = result_tx.send(AppEvent::SearchResultsLoaded(result));
            });
        }
        SideEffect::PlayAlbum(id) => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let app_state = Arc::clone(&backend.app_state);
                let queue_state = Arc::clone(&backend.queue_state);
                let audio_player = Arc::clone(&backend.audio_player);
                match firmium_backend::commands::subsonic::get_album_tracks(Arc::clone(&app_state), id).await {
                    Ok(tracks) => {
                        let _ = firmium_backend::commands::queue::set_queue(
                            queue_state, app_state, audio_player, tracks.tracks, 0,
                        )
                        .await;
                    }
                    Err(e) => {
                        let _ = result_tx.send(AppEvent::AlbumsLoaded(Err(e.message())));
                    }
                }
            });
        }
        SideEffect::PlayPlaylist(id) => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let app_state = Arc::clone(&backend.app_state);
                let queue_state = Arc::clone(&backend.queue_state);
                let audio_player = Arc::clone(&backend.audio_player);
                match firmium_backend::commands::subsonic::get_playlist_tracks(Arc::clone(&app_state), id).await {
                    Ok(tracks) => {
                        let _ = firmium_backend::commands::queue::set_queue(
                            queue_state, app_state, audio_player, tracks.tracks, 0,
                        )
                        .await;
                    }
                    Err(e) => {
                        let _ = result_tx.send(AppEvent::PlaylistsLoaded(Err(e.message())));
                    }
                }
            });
        }
        SideEffect::PlaySongs(songs) => {
            let Some(backend) = app.backend.clone() else { return };
            tokio::spawn(async move {
                let app_state = Arc::clone(&backend.app_state);
                let queue_state = Arc::clone(&backend.queue_state);
                let audio_player = Arc::clone(&backend.audio_player);
                let _ = firmium_backend::commands::queue::set_queue(
                    queue_state, app_state, audio_player, songs, 0,
                )
                .await;
            });
        }
    }
}
