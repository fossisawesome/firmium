import { writable, derived, get, type Writable } from 'svelte/store'
import { SafeStorage } from './utils'
import { tauriInvoke } from './tauri'
import type { Song, PlaybackState, LyricLine, SimilarMatch } from './types/tauri-commands'
import type { AudioBridge } from './audio-bridge'
import type { ServerPlaylist } from './api'

const DEFAULT_VOLUME = 0.8

// ── Auth ──────────────────────────────────────────────────────────────────────
export const authServer = writable<string | null>(null)
export const authUsername = writable<string | null>(null)
export const authPassword = writable<string | null>(null)
export const isAuthed = derived(
  [authServer, authUsername, authPassword],
  ([$s, $u, $p]) => Boolean($s && $u && $p)
)

export function setAuth(s: string | null, u: string | null, p: string | null): void {
  const server = s ? String(s).trim().replace(/\/+$/, '') : null
  authServer.set(server)
  authUsername.set(u)
  authPassword.set(p)
  openSubsonicExtensions.set(null)
  tauriInvoke('set_connection', { server, username: u, password: p })
}

export function clearAuth(): void {
  authServer.set(null)
  authUsername.set(null)
  authPassword.set(null)
  tauriInvoke('set_connection', { server: null, username: null, password: null })
}

export async function getQueryParams(): Promise<Record<string, unknown>> {
  const username = get(authUsername)
  const password = get(authPassword)
  if (!username || !password) return {}
  return tauriInvoke('generate_auth_params', { username, password })
}

// ── Server info ───────────────────────────────────────────────────────────────
export const openSubsonicExtensions = writable<string[] | null>(null)
export const isOpenSubsonic = derived(openSubsonicExtensions, $ext => $ext !== null)
export const hasSonicSimilarity = derived(openSubsonicExtensions, $ext => $ext?.includes('sonicSimilarity') ?? false)

// ── View routing ──────────────────────────────────────────────────────────────
export type ViewType = 'albums' | 'artists' | 'search' | 'playlists' | 'settings' | 'home'
  | 'album' | 'artist' | 'playlist'

export interface ActiveView {
  type: ViewType
  id?: string
  parentType?: ViewType
}

export const activeView = writable<ActiveView>({ type: 'albums' })

export function navToView(type: ViewType): void {
  activeView.set({ type })
}

export function navToAlbum(id: string): void {
  const parent = get(activeView).type
  activeView.set({ type: 'album', id, parentType: (['albums', 'artists', 'search', 'home'] as ViewType[]).includes(parent) ? parent : 'albums' })
}

export function navToArtist(id: string): void {
  const parent = get(activeView).type
  activeView.set({ type: 'artist', id, parentType: (['albums', 'artists', 'home'] as ViewType[]).includes(parent) ? parent : 'artists' })
}

export function navToPlaylist(id: string): void {
  activeView.set({ type: 'playlist', id })
}

export function navBack(): void {
  const view = get(activeView)
  if (view.parentType) activeView.set({ type: view.parentType })
  else if (view.type === 'playlist') activeView.set({ type: 'playlists' })
  else activeView.set({ type: 'albums' })
}

// ── Playback ──────────────────────────────────────────────────────────────────
export const queue = writable<Song[]>([])
export const queueIdx = writable(-1)
export const currentTrack = derived([queue, queueIdx], ([$q, $i]) => $q[$i] || null)
export const playbackState = writable<PlaybackState>('stopped')
export const volume = writable(Number(SafeStorage.getItem('firmium_volume') ?? DEFAULT_VOLUME))
export const repeatOne = writable(false)
export const repeatAll = writable(false)
export const crossfadeEnabled = writable(SafeStorage.getItem('firmium_crossfade') === 'true')
export const crossfadeDuration = writable(
  Math.max(1, Math.min(12, Number(SafeStorage.getItem('firmium_crossfade_duration') ?? 5)))
)
// Shuffle playback mode.
export const shuffleEnabled = writable(false)

export const currentPosition = writable(0)
export const trackDuration = writable<number | null>(null)
export const isSeeking = writable(false)

// Used to cancel stale play requests when a new one supersedes them.
let _playToken = 0
export const bumpToken = (): number => ++_playToken
export const getPlayToken = (): number => _playToken

export function setVolume(v: number): number {
  const normalized = Math.max(0, Math.min(1, Number.isFinite(Number(v)) ? Number(v) : DEFAULT_VOLUME))
  volume.set(normalized)
  SafeStorage.setItem('firmium_volume', String(normalized))
  return normalized
}

export function setCrossfadeEnabled(v: unknown): void {
  const val = Boolean(v)
  crossfadeEnabled.set(val)
  SafeStorage.setItem('firmium_crossfade', val ? 'true' : 'false')
}

export function setCrossfadeDuration(v: number): void {
  const val = Math.max(1, Math.min(12, Number(v) || 5))
  crossfadeDuration.set(val)
  SafeStorage.setItem('firmium_crossfade_duration', String(val))
}

// Gapless playback — pre-buffers the next track so there's no pause between songs.
// Mutually exclusive with crossfade; gapless is skipped when crossfade is active.
export const gaplessEnabled = writable(SafeStorage.getItem('firmium_gapless') !== 'false')

export function setGaplessEnabled(v: unknown): void {
  const val = Boolean(v)
  gaplessEnabled.set(val)
  SafeStorage.setItem('firmium_gapless', val ? 'true' : 'false')
}

// Bit-perfect audio — reopens the output device at each track's native sample rate.
export const bitPerfectEnabled = writable(SafeStorage.getItem('firmium_bit_perfect') !== 'false')

export function setBitPerfectEnabled(v: unknown): void {
  const val = Boolean(v)
  bitPerfectEnabled.set(val)
  SafeStorage.setItem('firmium_bit_perfect', val ? 'true' : 'false')
}

// Info about the currently-playing stream's actual output format, emitted by the Rust backend.
export const activeStreamInfo = writable<{ sampleRate: number; channels: number; bitPerfect: boolean } | null>(null)

// ── Lyrics ────────────────────────────────────────────────────────────────────
export const lyricsOpen = writable(false)
export const lyricsLines = writable<LyricLine[]>([])
export const lyricsSynced = writable(false)
export const lyricsTrackId = writable<string | null>(null)
export const lyricsStatus = writable('No track playing')

// ── Similar tracks ───────────────────────────────────────────────────────────
export const similarTracksOpen = writable(false)
export const similarTracksTrackId = writable<string | null>(null)
export const similarTracksResults = writable<SimilarMatch[]>([])
export const similarTracksStatus = writable('')

// ── Audio bridge ──────────────────────────────────────────────────────────────
export const audioBridge: Writable<AudioBridge | null> = writable(null)

// ── Recently Played Songs (persisted to localStorage, max 20) ────────────────
const RECENT_SONGS_KEY = 'firmium_recent_songs'
const RECENT_SONGS_MAX = 20

function _loadFromStorage<T>(key: string, fallback: T): T {
  try { const raw = SafeStorage.getItem(key); return raw ? JSON.parse(raw) : fallback } catch (_) { return fallback }
}

function createRecentSongsStore() {
  const { subscribe, update } = writable<Song[]>(_loadFromStorage(RECENT_SONGS_KEY, []))

  return {
    subscribe,
    // Adds a track to the front, dedupes by id, trims to max length, persists.
    push(track: Song) {
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
export interface Playlist {
  id: string
  name: string
  description: string
  coverArtId: string | null
  coverDataUrl: string | null
  tracks: Song[]
  serverId: string | null
}

type PlaylistChanges = Partial<Pick<Playlist, 'name' | 'description' | 'coverArtId' | 'coverDataUrl' | 'serverId'>>

const PLAYLISTS_KEY = 'firmium_playlists'
const _uuid = (): string => 'pl-' + crypto.randomUUID()

function _savePlaylists(pls: Playlist[]): void { SafeStorage.setItem(PLAYLISTS_KEY, JSON.stringify(pls)) }

function createPlaylistsStore() {
  const { subscribe, update } = writable<Playlist[]>(_loadFromStorage(PLAYLISTS_KEY, []))

  return {
    subscribe,
    create(name = 'New Playlist'): Playlist {
      // serverId is set later once the playlist is created on the server.
      const pl: Playlist = { id: _uuid(), name: String(name).trim() || 'New Playlist', description: '', coverArtId: null, coverDataUrl: null, tracks: [], serverId: null }
      update(pls => { const next = [...pls, pl]; _savePlaylists(next); return next })
      return pl
    },
    updatePlaylist(id: string, changes: PlaylistChanges): void {
      update(pls => {
        const next = pls.map(p => {
          if (p.id !== id) return p
          const updated = { ...p };
          (['name', 'description', 'coverArtId', 'coverDataUrl', 'serverId'] as const).forEach(k => { if (k in changes) (updated as any)[k] = (changes as any)[k] })
          return updated
        })
        _savePlaylists(next)
        return next
      })
    },
    // Records the server-side ID returned after creating/syncing a playlist.
    setServerId(id: string, serverId: string): void {
      update(pls => {
        const next = pls.map(p => p.id === id ? { ...p, serverId } : p)
        _savePlaylists(next)
        return next
      })
    },
    delete(id: string): void {
      update(pls => { const next = pls.filter(p => p.id !== id); _savePlaylists(next); return next })
    },
    addTracks(id: string, tracks: Song[]): { added: number; newTracks: Song[] } {
      let added = 0
      let newTracks: Song[] = []
      update(pls => {
        const next = pls.map(p => {
          if (p.id !== id) return p
          const existingIds = new Set(p.tracks.map(t => t.id))
          newTracks = tracks.filter(t => !existingIds.has(t.id))
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
      return { added, newTracks }
    },
    removeTrack(id: string, trackId: string): number {
      let removedIndex = -1
      update(pls => {
        const next = pls.map(p => {
          if (p.id !== id) return p
          removedIndex = p.tracks.findIndex(t => t.id === trackId)
          return { ...p, tracks: p.tracks.filter(t => t.id !== trackId) }
        })
        _savePlaylists(next)
        return next
      })
      return removedIndex
    }
  }
}

export const playlists = createPlaylistsStore()

// Server-fetched playlists (in-memory, populated when the Playlists view is opened).
export const serverPlaylists = writable<ServerPlaylist[]>([])
