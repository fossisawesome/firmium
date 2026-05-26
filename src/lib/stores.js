import { writable, derived, get } from 'svelte/store'
import { SafeStorage } from './utils.js'
import { tauriInvoke } from './tauri.js'

const DEFAULT_VOLUME = 0.8

// ── Auth ──────────────────────────────────────────────────────────────────────
export const authServer = writable(null)
export const authUsername = writable(null)
export const authPassword = writable(null)
export const isAuthed = derived(
  [authServer, authUsername, authPassword],
  ([$s, $u, $p]) => Boolean($s && $u && $p)
)

export function setAuth(s, u, p) {
  authServer.set(s ? String(s).trim().replace(/\/+$/, '') : null)
  authUsername.set(u)
  authPassword.set(p)
}

export function clearAuth() {
  authServer.set(null)
  authUsername.set(null)
  authPassword.set(null)
}

export async function getQueryParams() {
  const username = get(authUsername)
  const password = get(authPassword)
  if (!username || !password) return {}
  return tauriInvoke('generate_auth_params', { username, password })
}

// ── Server info ───────────────────────────────────────────────────────────────
export const openSubsonicExtensions = writable(null)
export const isOpenSubsonic = derived(openSubsonicExtensions, $ext => $ext !== null)

// ── View routing ──────────────────────────────────────────────────────────────
// type: 'albums' | 'artists' | 'search' | 'playlists' | 'settings'
//     | 'album'   (id, parentType)
//     | 'artist'  (id, parentType)
//     | 'playlist'(id)
export const activeView = writable({ type: 'albums' })

export function navToView(type) {
  activeView.set({ type })
}

export function navToAlbum(id) {
  const parent = get(activeView).type
  activeView.set({ type: 'album', id, parentType: ['albums', 'artists', 'search'].includes(parent) ? parent : 'albums' })
}

export function navToArtist(id) {
  const parent = get(activeView).type
  activeView.set({ type: 'artist', id, parentType: ['albums', 'artists'].includes(parent) ? parent : 'artists' })
}

export function navToPlaylist(id) {
  activeView.set({ type: 'playlist', id })
}

export function navBack() {
  const view = get(activeView)
  if (view.parentType) activeView.set({ type: view.parentType })
  else if (view.type === 'playlist') activeView.set({ type: 'playlists' })
  else activeView.set({ type: 'albums' })
}

// ── Playback ──────────────────────────────────────────────────────────────────
export const queue = writable([])
export const queueIdx = writable(-1)
export const currentTrack = derived([queue, queueIdx], ([$q, $i]) => $q[$i] || null)
export const playbackState = writable('stopped')
export const volume = writable(Number(SafeStorage.getItem('firmium_volume') ?? DEFAULT_VOLUME))
export const repeatOne = writable(false)
export const repeatAll = writable(false)
export const crossfadeEnabled = writable(SafeStorage.getItem('firmium_crossfade') !== 'false')
export const crossfadeDuration = writable(
  Math.max(1, Math.min(12, Number(SafeStorage.getItem('firmium_crossfade_duration') ?? 5)))
)
export const currentPosition = writable(0)
export const trackDuration = writable(null)
export const isSeeking = writable(false)

// Used to cancel stale play requests when a new one supersedes them.
let _playToken = 0
export const bumpToken = () => ++_playToken
export const getPlayToken = () => _playToken

export function setVolume(v) {
  const normalized = Math.max(0, Math.min(1, Number.isFinite(Number(v)) ? Number(v) : DEFAULT_VOLUME))
  volume.set(normalized)
  SafeStorage.setItem('firmium_volume', String(normalized))
  return normalized
}

export function setCrossfadeEnabled(v) {
  const val = Boolean(v)
  crossfadeEnabled.set(val)
  SafeStorage.setItem('firmium_crossfade', val ? 'true' : 'false')
}

export function setCrossfadeDuration(v) {
  const val = Math.max(1, Math.min(12, Number(v) || 5))
  crossfadeDuration.set(val)
  SafeStorage.setItem('firmium_crossfade_duration', String(val))
}

// Gapless playback — pre-buffers the next track so there's no pause between songs.
// Mutually exclusive with crossfade; gapless is skipped when crossfade is active.
export const gaplessEnabled = writable(SafeStorage.getItem('firmium_gapless') !== 'false')

export function setGaplessEnabled(v) {
  const val = Boolean(v)
  gaplessEnabled.set(val)
  SafeStorage.setItem('firmium_gapless', val ? 'true' : 'false')
}

// ── Lyrics ────────────────────────────────────────────────────────────────────
export const lyricsOpen = writable(false)
export const lyricsLines = writable([])
export const lyricsSynced = writable(false)
export const lyricsTrackId = writable(null)
export const lyricsStatus = writable('No track playing')

// ── Audio bridge ──────────────────────────────────────────────────────────────
export const audioBridge = writable(null)

// ── Active AbortController for list-panel requests ────────────────────────────
let _activeCtrl = null
export const abortActive = () => { if (_activeCtrl) { _activeCtrl.abort(); _activeCtrl = null } }
export const newAbortCtrl = () => { abortActive(); _activeCtrl = new AbortController(); return _activeCtrl }
export const getActiveSignal = () => _activeCtrl?.signal ?? null

let _searchCtrl = null
export const abortSearch = () => { if (_searchCtrl) { _searchCtrl.abort(); _searchCtrl = null } }
export const newSearchCtrl = () => { abortSearch(); _searchCtrl = new AbortController(); return _searchCtrl }

// ── Recently Played Songs (persisted to localStorage, max 20) ────────────────
const RECENT_SONGS_KEY = 'firmium_recent_songs'
const RECENT_SONGS_MAX = 20

function _loadFromStorage(key, fallback = []) {
  try { const raw = SafeStorage.getItem(key); return raw ? JSON.parse(raw) : fallback } catch (_) { return fallback }
}

function createRecentSongsStore() {
  const { subscribe, update } = writable(_loadFromStorage(RECENT_SONGS_KEY))

  return {
    subscribe,
    // Adds a track to the front, dedupes by id, trims to max length, persists.
    push(track) {
      update(songs => {
        const filtered = songs.filter(s => s.id !== track.id)
        const next = [track, ...filtered].slice(0, RECENT_SONGS_MAX)
        SafeStorage.setItem(RECENT_SONGS_KEY, JSON.stringify(next))
        return next
      })
    }
  }
}

export const recentlyPlayedSongs = createRecentSongsStore()

// ── Playlists (persisted to localStorage) ─────────────────────────────────────
const PLAYLISTS_KEY = 'firmium_playlists'
const _uuid = () => 'pl-' + Date.now() + '-' + Math.random().toString(36).slice(2, 7)

function _savePlaylists(pls) { SafeStorage.setItem(PLAYLISTS_KEY, JSON.stringify(pls)) }

function createPlaylistsStore() {
  const { subscribe, update } = writable(_loadFromStorage(PLAYLISTS_KEY))

  return {
    subscribe,
    create(name = 'New Playlist') {
      const pl = { id: _uuid(), name: String(name).trim() || 'New Playlist', description: '', coverArtId: null, coverDataUrl: null, tracks: [] }
      update(pls => { const next = [...pls, pl]; _savePlaylists(next); return next })
      return pl
    },
    updatePlaylist(id, changes) {
      update(pls => {
        const next = pls.map(p => {
          if (p.id !== id) return p
          const updated = { ...p };
          ['name', 'description', 'coverArtId', 'coverDataUrl'].forEach(k => { if (k in changes) updated[k] = changes[k] })
          return updated
        })
        _savePlaylists(next)
        return next
      })
    },
    delete(id) {
      update(pls => { const next = pls.filter(p => p.id !== id); _savePlaylists(next); return next })
    },
    addTracks(id, tracks) {
      let added = 0
      update(pls => {
        const next = pls.map(p => {
          if (p.id !== id) return p
          const existingIds = new Set(p.tracks.map(t => t.id))
          const newTracks = tracks.filter(t => !existingIds.has(t.id))
          added = newTracks.length
          const updatedTracks = [...p.tracks, ...newTracks]
          const coverArtId = (!p.coverArtId && !p.coverDataUrl)
            ? updatedTracks.find(t => t.coverArtId)?.coverArtId ?? null
            : p.coverArtId
          return { ...p, tracks: updatedTracks, coverArtId }
        })
        _savePlaylists(next)
        return next
      })
      return added
    },
    removeTrack(id, trackId) {
      update(pls => {
        const next = pls.map(p => p.id === id ? { ...p, tracks: p.tracks.filter(t => t.id !== trackId) } : p)
        _savePlaylists(next)
        return next
      })
    }
  }
}

export const playlists = createPlaylistsStore()
