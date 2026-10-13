// State for the floating per-item action popup (Start Radio / Add to playlist).
import type { Song } from './types'

export type PendingAction =
  | { type: 'tracks'; tracks: Song[] }
  | { type: 'album'; albumId: string; albumName: string }
  | { type: 'artist'; artistId: string; artistName: string }
  | null

class MenuState {
  visible = $state(false)
  rect = $state.raw<DOMRect | null>(null)
  pending = $state.raw<PendingAction>(null)
  /** 'actions' = top-level menu, 'list' = pick a playlist, 'create' = name a new one. */
  mode = $state<'actions' | 'list' | 'create'>('actions')
}

export const menu = new MenuState()

export function showMenu(anchor: HTMLElement, pending: PendingAction): void {
  menu.visible = true
  menu.rect = anchor.getBoundingClientRect()
  menu.pending = pending
  menu.mode = 'actions'
}
export function hideMenu(): void {
  menu.visible = false
  menu.pending = null
  menu.mode = 'actions'
}
export const menuList = () => { menu.mode = 'list' }
export const menuCreate = () => { menu.mode = 'create' }
