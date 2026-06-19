import { writable } from 'svelte/store'
import type { Song } from './types/tauri-commands'

export type PendingAction =
  | { type: 'tracks'; tracks: Song[] }
  | { type: 'album'; albumId: string; albumName: string }
  | { type: 'artist'; artistId: string; artistName: string }
  | null

interface PlaylistMenuState {
  visible: boolean
  rect: DOMRect | null
  pending: PendingAction
  // 'actions' = top-level item menu (Start Radio / Add to playlist),
  // 'list' = pick a playlist, 'create' = name a new playlist.
  mode: 'actions' | 'list' | 'create'
}

// State for the floating per-item action popup.
export const playlistMenuState = writable<PlaylistMenuState>({
  visible: false,
  rect: null,        // DOMRect of the anchor button
  pending: null,
  mode: 'actions'
})

export function showPlaylistMenu(anchorEl: HTMLElement, pending: PendingAction): void {
  const rect = anchorEl.getBoundingClientRect()
  playlistMenuState.set({ visible: true, rect, pending, mode: 'actions' })
}

export function hidePlaylistMenu(): void {
  playlistMenuState.update(s => ({ ...s, visible: false, pending: null, mode: 'actions' }))
}

export function switchToList(): void {
  playlistMenuState.update(s => ({ ...s, mode: 'list' }))
}

export function switchToCreate(): void {
  playlistMenuState.update(s => ({ ...s, mode: 'create' }))
}
