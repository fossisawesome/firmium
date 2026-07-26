use iced::Task;


use super::super::message::Message;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn update_playlists(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PlaylistTracksScrolled(y) => {
                self.playlist_tracks_scroll = y;
                Task::none()
            }
            Message::PlaylistsLoaded(Ok(p)) => {
                self.server_playlists = p;
                // Adopt same-named server playlists for unsynced locals (avoid dups);
                // retry creating the rest that are still under the attempt cap.
                let mut to_retry: Vec<(String, String, Vec<String>)> = Vec::new();
                for local in self.playlists.iter_mut() {
                    if local.server_id.is_some() || !local.create_pending {
                        continue;
                    }
                    if local.create_attempts >= crate::playlists::CREATE_ATTEMPT_CAP {
                        continue;
                    }
                    let same = self.server_playlists.iter().find(|sp| {
                        sp.get("name").and_then(|v| v.as_str()) == Some(local.name.as_str())
                    });
                    if let Some(sp) = same {
                        local.server_id = sp.get("id").and_then(|v| v.as_str()).map(String::from);
                        local.create_pending = false;
                    } else {
                        let ids = local.tracks.iter().map(|s| s.id.clone()).collect();
                        to_retry.push((local.id.clone(), local.name.clone(), ids));
                    }
                }
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                let mut cover_ids: Vec<String> = self
                    .playlists
                    .iter()
                    .flat_map(|p| p.tracks.iter().filter_map(|s| s.cover_art_id.clone()))
                    .collect();
                cover_ids.extend(
                    self.server_playlists
                        .iter()
                        .filter_map(|sp| sp.get("coverArt").and_then(|v| v.as_str()).map(String::from)),
                );
                let tasks = to_retry.into_iter().map(|(local_id, name, ids)| {
                    Task::perform(
                        firmium_backend::commands::playlists::sync_create(
                            self.backend.app_state.clone(),
                            name,
                            ids,
                        ),
                        move |res| Message::PlaylistCreateSynced(local_id.clone(), res),
                    )
                });
                Task::batch(std::iter::once(self.load_cover_ids(cover_ids)).chain(tasks))
            }
            Message::PlaylistsLoaded(Err(e)) => {
                eprintln!("get_playlists failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::PlaylistTracksLoaded(Ok(pt)) => {
                let ids: Vec<String> = pt.tracks.iter().filter_map(|s| s.cover_art_id.clone()).collect();
                self.playlist_detail = Some(pt);
                self.load_cover_ids(ids)
            }
            Message::PlaylistTracksLoaded(Err(e)) => {
                eprintln!("get_playlist_tracks failed: {e:?}");
                self.show_toast(e);
                Task::none()
            }
            Message::PlayPlaylistAt(idx) => {
                if let Some(pt) = &self.playlist_detail {
                    let songs = pt.tracks.clone();
                    Task::perform(
                        firmium_backend::commands::queue::set_queue(
                            self.backend.queue_state.clone(),
                            self.backend.app_state.clone(),
                            self.backend.audio_player.clone(),
                            songs,
                            idx,
                        ),
                        Message::PlaybackDone,
                    )
                } else {
                    Task::none()
                }
            }
            Message::OpenAddToPlaylist(song) => {
                self.add_to_playlist_song = Some(song);
                self.new_playlist_name.clear();
                // Lazily load the playlist list the first time the overlay opens.
                if self.server_playlists.is_empty() {
                    Task::perform(
                        firmium_backend::commands::subsonic::get_playlists(self.backend.app_state.clone()),
                        Message::PlaylistsLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::CloseAddToPlaylist => {
                self.add_to_playlist_song = None;
                Task::none()
            }
            Message::NewPlaylistNameInput(s) => {
                self.new_playlist_name = s;
                Task::none()
            }
            Message::AddToPlaylist(local_id) => {
                if let Some(song) = self.add_to_playlist_song.take() {
                    let added = crate::playlists::add_tracks(&mut self.playlists, &local_id, vec![song]);
                    crate::playlists::save_playlists(&self.playlists);
                    self.rebuild_playlist_items();
                    self.refresh_local_detail(&local_id);
                    let server_id = self
                        .playlists
                        .iter()
                        .find(|p| p.id == local_id)
                        .and_then(|p| p.server_id.clone());
                    match server_id {
                        Some(sid) => Task::perform(
                            firmium_backend::commands::playlists::push_add(self.backend.app_state.clone(), sid, added),
                            |_| Message::PlaylistSyncNoop,
                        ),
                        None => Task::none(),
                    }
                } else {
                    Task::none()
                }
            }
            Message::CreatePlaylistAndAdd => {
                let name = self.new_playlist_name.trim().to_string();
                match self.add_to_playlist_song.take() {
                    Some(song) if !name.is_empty() => {
                        let mut p = crate::playlists::new_local(name.clone());
                        let local_id = p.id.clone();
                        p.tracks.push(song);
                        let track_ids: Vec<String> = p.tracks.iter().map(|s| s.id.clone()).collect();
                        self.playlists.insert(0, p);
                        crate::playlists::save_playlists(&self.playlists);
                        self.rebuild_playlist_items();
                        self.new_playlist_name.clear();
                        Task::perform(
                            firmium_backend::commands::playlists::sync_create(
                                self.backend.app_state.clone(),
                                name,
                                track_ids,
                            ),
                            move |res| Message::PlaylistCreateSynced(local_id.clone(), res),
                        )
                    }
                    // Nothing to do if the name is blank; keep the overlay open.
                    other => {
                        self.add_to_playlist_song = other;
                        Task::none()
                    }
                }
            }
            Message::PlaylistSyncNoop => Task::none(),

            // ── Local-first playlist management ──────────────────────────────────
            Message::OpenCreatePlaylist => {
                self.create_playlist_name.clear();
                self.show_create_playlist = true;
                Task::none()
            }
            Message::CloseCreatePlaylist => {
                self.show_create_playlist = false;
                Task::none()
            }
            Message::CreatePlaylistNameInput(s) => {
                self.create_playlist_name = s;
                Task::none()
            }
            Message::CreatePlaylist(name) => {
                let name = name.trim().to_string();
                self.show_create_playlist = false;
                if name.is_empty() {
                    return Task::none();
                }
                let p = crate::playlists::new_local(name.clone());
                let local_id = p.id.clone();
                self.playlists.insert(0, p);
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                Task::perform(
                    firmium_backend::commands::playlists::sync_create(
                        self.backend.app_state.clone(),
                        name,
                        Vec::new(),
                    ),
                    move |res| Message::PlaylistCreateSynced(local_id.clone(), res),
                )
            }
            Message::PlaylistCreateSynced(local_id, Ok(server_pl)) => {
                let server_id = server_pl.get("id").and_then(|v| v.as_str()).map(String::from);
                if let Some(p) = self.playlists.iter_mut().find(|p| p.id == local_id) {
                    p.server_id = server_id;
                    p.create_pending = false;
                }
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                Task::none()
            }
            Message::PlaylistCreateSynced(local_id, Err(e)) => {
                eprintln!("playlist create sync failed: {e:?}");
                if let Some(p) = self.playlists.iter_mut().find(|p| p.id == local_id) {
                    p.create_attempts += 1;
                    p.create_pending = p.create_attempts < crate::playlists::CREATE_ATTEMPT_CAP;
                }
                crate::playlists::save_playlists(&self.playlists);
                Task::none()
            }
            Message::SyncPlaylistNow(local_id) => {
                let Some(p) = self.playlists.iter().find(|p| p.id == local_id) else {
                    return Task::none();
                };
                if p.server_id.is_some() {
                    return Task::none();
                }
                let name = p.name.clone();
                let track_ids: Vec<String> = p.tracks.iter().map(|s| s.id.clone()).collect();
                let lid = local_id.clone();
                Task::perform(
                    firmium_backend::commands::playlists::sync_create(
                        self.backend.app_state.clone(),
                        name,
                        track_ids,
                    ),
                    move |res| Message::PlaylistCreateSynced(lid.clone(), res),
                )
            }
            Message::DeleteLocalPlaylist(local_id) => {
                let server_id = self
                    .playlists
                    .iter()
                    .find(|p| p.id == local_id)
                    .and_then(|p| p.server_id.clone());
                self.playlists.retain(|p| p.id != local_id);
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                // If the open detail belonged to this playlist, go back to the list.
                if self.playlist_detail_id.as_deref() == Some(local_id.as_str()) {
                    self.view = View::Playlists;
                    self.playlist_detail = None;
                    self.playlist_detail_id = None;
                }
                self.nav_stack.retain(|v| *v != View::PlaylistDetail(local_id.clone()));
                self.forward_stack.retain(|v| *v != View::PlaylistDetail(local_id.clone()));
                match server_id {
                    Some(sid) => Task::perform(
                        firmium_backend::commands::playlists::push_delete(self.backend.app_state.clone(), sid),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    None => Task::none(),
                }
            }
            Message::RenamePlaylist(local_id, name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }
                let mut server_id = None;
                if let Some(p) = self.playlists.iter_mut().find(|p| p.id == local_id) {
                    p.name = name.clone();
                    server_id = p.server_id.clone();
                }
                crate::playlists::save_playlists(&self.playlists);
                self.rebuild_playlist_items();
                self.refresh_local_detail(&local_id);
                match server_id {
                    Some(sid) => Task::perform(
                        firmium_backend::commands::playlists::push_rename(self.backend.app_state.clone(), sid, name),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    None => Task::none(),
                }
            }
            Message::StartRenamePlaylist(id) => {
                self.create_playlist_name = self
                    .playlists
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                self.renaming_playlist = Some(id);
                Task::none()
            }
            Message::CommitRenamePlaylist => {
                match self.renaming_playlist.take() {
                    Some(id) => {
                        let name = self.create_playlist_name.clone();
                        self.update(Message::RenamePlaylist(id, name))
                    }
                    None => Task::none(),
                }
            }
            Message::MovePlaylistTrack(local_id, from, to) => {
                let ordered = crate::playlists::move_track(&mut self.playlists, &local_id, from, to);
                if ordered.is_none() {
                    return Task::none();
                }
                crate::playlists::save_playlists(&self.playlists);
                self.refresh_local_detail(&local_id);
                let server_id = self
                    .playlists
                    .iter()
                    .find(|p| p.id == local_id)
                    .and_then(|p| p.server_id.clone());
                match (server_id, ordered) {
                    (Some(sid), Some(ids)) => Task::perform(
                        firmium_backend::commands::playlists::push_reorder(self.backend.app_state.clone(), sid, ids),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    _ => Task::none(),
                }
            }
            Message::RemovePlaylistTrack(local_id, track_id) => {
                let idx = crate::playlists::remove_track(&mut self.playlists, &local_id, &track_id);
                if idx.is_none() {
                    return Task::none();
                }
                crate::playlists::save_playlists(&self.playlists);
                self.refresh_local_detail(&local_id);
                self.rebuild_playlist_items();
                let server_id = self
                    .playlists
                    .iter()
                    .find(|p| p.id == local_id)
                    .and_then(|p| p.server_id.clone());
                match (server_id, idx) {
                    (Some(sid), Some(i)) => Task::perform(
                        firmium_backend::commands::playlists::push_remove(self.backend.app_state.clone(), sid, i as u32),
                        |_| Message::PlaylistSyncNoop,
                    ),
                    _ => Task::none(),
                }
            }
            Message::MoveServerTrack(server_id, from, to) => {
                let Some(pt) = &mut self.playlist_detail else {
                    return Task::none();
                };
                let n = pt.tracks.len();
                if from >= n || to >= n || from == to {
                    return Task::none();
                }
                let moved = pt.tracks.remove(from);
                pt.tracks.insert(to, moved);
                let ids: Vec<String> = pt.tracks.iter().map(|s| s.id.clone()).collect();
                Task::perform(
                    firmium_backend::commands::playlists::push_reorder(self.backend.app_state.clone(), server_id, ids),
                    |_| Message::PlaylistSyncNoop,
                )
            }
            Message::RemoveServerTrack(server_id, index) => {
                let Some(pt) = &mut self.playlist_detail else {
                    return Task::none();
                };
                if index >= pt.tracks.len() {
                    return Task::none();
                }
                pt.tracks.remove(index);
                Task::perform(
                    firmium_backend::commands::playlists::push_remove(self.backend.app_state.clone(), server_id, index as u32),
                    |_| Message::PlaylistSyncNoop,
                )
            }

            // ── Search ──────────────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}
