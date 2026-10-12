// Playlist list shared by the Playlists view and the add-to-playlist popup.
import { writable } from 'svelte/store'
import { Api } from './api'
import type { Playlist, ServerPlaylist } from './types'

export const localPlaylists = writable<Playlist[]>([])
export const serverOnlyPlaylists = writable<ServerPlaylist[]>([])

/** Loads local playlists; when `sync` is set, also reconciles with the server. */
export async function loadPlaylists(sync: boolean): Promise<void> {
  localPlaylists.set(await Api.localPlaylists())
  if (!sync) return
  const lists = await Api.syncServerPlaylists()
  localPlaylists.set(lists.local)
  serverOnlyPlaylists.set(lists.serverOnly)
}
