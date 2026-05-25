import { writable } from 'svelte/store'

// State for the floating playlist add popup.
export const playlistMenuState = writable({
  visible: false,
  rect: null,     // DOMRect of the anchor button
  pending: null,  // { type: 'tracks', tracks: [...] } | { type: 'album', albumId, albumName }
  mode: 'list'    // 'list' | 'create'
})

export function showPlaylistMenu(anchorEl, pending) {
  const rect = anchorEl.getBoundingClientRect()
  playlistMenuState.set({ visible: true, rect, pending, mode: 'list' })
}

export function hidePlaylistMenu() {
  playlistMenuState.update(s => ({ ...s, visible: false, pending: null, mode: 'list' }))
}

export function switchToCreate() {
  playlistMenuState.update(s => ({ ...s, mode: 'create' }))
}
