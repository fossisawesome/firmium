// State for the floating per-item action popup (Start Radio / Add to playlist).
import { writable } from 'svelte/store'
import type { Song } from './types'

export type PendingAction =
  | { type: 'tracks'; tracks: Song[] }
  | { type: 'album'; albumId: string; albumName: string }
  | { type: 'artist'; artistId: string; artistName: string }
  | null

interface MenuState {
  visible: boolean
  rect: DOMRect | null
  pending: PendingAction
  /** 'actions' = top-level menu, 'list' = pick a playlist, 'create' = name a new one. */
  mode: 'actions' | 'list' | 'create'
}

export const menuState = writable<MenuState>({ visible: false, rect: null, pending: null, mode: 'actions' })

export function showMenu(anchor: HTMLElement, pending: PendingAction): void {
  menuState.set({ visible: true, rect: anchor.getBoundingClientRect(), pending, mode: 'actions' })
}
export const hideMenu = () => menuState.update(s => ({ ...s, visible: false, pending: null, mode: 'actions' }))
export const menuList = () => menuState.update(s => ({ ...s, mode: 'list' }))
export const menuCreate = () => menuState.update(s => ({ ...s, mode: 'create' }))
