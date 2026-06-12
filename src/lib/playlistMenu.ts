import { writable } from 'svelte/store'
import type { Song } from './types/tauri-commands'

type PendingAction =
  | { type: 'tracks'; tracks: Song[] }
  | { type: 'album'; albumId: string; albumName: string }
  | null

interface PlaylistMenuState {
  visible: boolean
  rect: DOMRect | null
  pending: PendingAction
  mode: 'list' | 'create'
}

// State for the floating playlist add popup.
export const playlistMenuState = writable<PlaylistMenuState>({
  visible: false,
  rect: null,     // DOMRect of the anchor button
  pending: null,  // { type: 'tracks', tracks: [...] } | { type: 'album', albumId, albumName }
  mode: 'list'    // 'list' | 'create'
})

export function showPlaylistMenu(anchorEl: HTMLElement, pending: PendingAction): void {
  const rect = anchorEl.getBoundingClientRect()
  playlistMenuState.set({ visible: true, rect, pending, mode: 'list' })
}

export function hidePlaylistMenu(): void {
  playlistMenuState.update(s => ({ ...s, visible: false, pending: null, mode: 'list' }))
}

export function switchToCreate(): void {
  playlistMenuState.update(s => ({ ...s, mode: 'create' }))
}
