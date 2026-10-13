// Playlist list shared by the Playlists view and the add-to-playlist popup.
import { Api } from './api'
import type { Playlist, ServerPlaylist } from './types'

class PlaylistState {
  local = $state.raw<Playlist[]>([])
  serverOnly = $state.raw<ServerPlaylist[]>([])
}

export const playlistState = new PlaylistState()

/** Loads local playlists; when `sync` is set, also reconciles with the server. */
export async function loadPlaylists(sync: boolean): Promise<void> {
  playlistState.local = await Api.localPlaylists()
  if (!sync) return
  const lists = await Api.syncServerPlaylists()
  playlistState.local = lists.local
  playlistState.serverOnly = lists.serverOnly
}
