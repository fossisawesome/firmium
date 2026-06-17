import { writable, derived, get, type Writable } from 'svelte/store'
import { listen } from '@tauri-apps/api/event'
import { SafeStorage } from './utils'
import { tauriInvoke } from './tauri'
import type { Song, PlaybackState, LyricLine, SimilarMatch, WordTiming, QueueStatePayload } from './types/tauri-commands'
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

// Bumped after local-library imports/downloads so local-data views refetch even
// though `dataSource` itself didn't change (still the same LocalApi instance).
export const dataSourceVersion = writable(0)
export function bumpDataSourceVersion(): void {
  dataSourceVersion.update(n => n + 1)
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

export function setVolume(v: number): number {
  const normalized = Math.max(0, Math.min(1, Number.isFinite(Number(v)) ? Number(v) : DEFAULT_VOLUME))
  volume.set(normalized)
  SafeStorage.setItem('firmium_volume', String(normalized))
  tauriInvoke('set_queue_volume', { volume: normalized }).catch(() => {})
  return normalized
}

export function setCrossfadeEnabled(v: unknown): void {
  const val = Boolean(v)
  crossfadeEnabled.set(val)
  SafeStorage.setItem('firmium_crossfade', val ? 'true' : 'false')
  if (val && get(bitPerfectMode) === 'strict') {
    bitPerfectMode.set('relaxed')
    SafeStorage.setItem('firmium_bit_perfect_mode', 'relaxed')
    tauriInvoke('set_bit_perfect_mode', { mode: 'relaxed' }).catch(() => {})
  }
  tauriInvoke('set_crossfade_settings', { enabled: val, durationSecs: get(crossfadeDuration) }).catch(() => {})
}

export function setCrossfadeDuration(v: number): void {
  const val = Math.max(1, Math.min(12, Number(v) || 5))
  crossfadeDuration.set(val)
  SafeStorage.setItem('firmium_crossfade_duration', String(val))
  tauriInvoke('set_crossfade_settings', { enabled: get(crossfadeEnabled), durationSecs: val }).catch(() => {})
}

// Gapless playback — pre-buffers the next track so there's no pause between songs.
// Mutually exclusive with crossfade; gapless is skipped when crossfade is active.
export const gaplessEnabled = writable(SafeStorage.getItem('firmium_gapless') !== 'false')

export const replayGainEnabled = writable(SafeStorage.getItem('firmium_replaygain') !== 'false')

export function setReplayGainEnabled(v: unknown): void {
  const val = Boolean(v)
  replayGainEnabled.set(val)
  SafeStorage.setItem('firmium_replaygain', val ? 'true' : 'false')
  tauriInvoke('set_replay_gain_enabled', { enabled: val }).catch(() => {})
}

export function setGaplessEnabled(v: unknown): void {
  const val = Boolean(v)
  gaplessEnabled.set(val)
  SafeStorage.setItem('firmium_gapless', val ? 'true' : 'false')
  tauriInvoke('set_gapless_enabled', { enabled: val }).catch(() => {})
}

// Subscribes to queue-state-changed events from Rust, keeping all queue/settings
// stores in sync and forwarding the active player ID to AudioBridge for event filtering.
export function listenToQueueState(): () => void {
  let unlisten: (() => void) | undefined
  listen<QueueStatePayload>('queue-state-changed', ({ payload }) => {
    queue.set(payload.queue)
    queueIdx.set(payload.queueIdx)
    repeatOne.set(payload.repeatOne)
    repeatAll.set(payload.repeatAll)
    shuffleEnabled.set(payload.shuffleEnabled)
    crossfadeEnabled.set(payload.crossfadeEnabled)
    crossfadeDuration.set(payload.crossfadeDuration)
    gaplessEnabled.set(payload.gaplessEnabled)
    replayGainEnabled.set(payload.replayGainEnabled)
    if (payload.volume !== get(volume)) {
      volume.set(payload.volume)
      SafeStorage.setItem('firmium_volume', String(payload.volume))
    }
    const bridge = get(audioBridge)
    if (bridge) bridge.currentPlayerId = payload.playerId ?? null
  }).then(fn => { unlisten = fn })
  return () => unlisten?.()
}

// Bit-perfect mode — controls whether the output stream is reopened to match each
// track's native sample rate. "strict" also disables crossfade.
export const bitPerfectMode = writable<string>(SafeStorage.getItem('firmium_bit_perfect_mode') ?? 'relaxed')

export function setBitPerfectMode(mode: string): void {
  bitPerfectMode.set(mode)
  SafeStorage.setItem('firmium_bit_perfect_mode', mode)
  tauriInvoke('set_bit_perfect_mode', { mode }).catch(() => {})
  if (mode === 'strict') setCrossfadeEnabled(false)
}


// ── Downloads ────────────────────────────────────────────────────────────────

// Download format: 'original' (server's source file, format=raw) or a transcode target.
export const downloadFormat = writable(SafeStorage.getItem('firmium_download_format') ?? 'original')

export function setDownloadFormat(v: string): void {
  downloadFormat.set(v)
  SafeStorage.setItem('firmium_download_format', v)
}

// ── Visualizer ───────────────────────────────────────────────────────────────
export const visualizerOpen = writable(false)
export const visualizerMode = writable<'orb' | 'bars'>(
  (SafeStorage.getItem('firmium_visualizer_mode') as 'orb' | 'bars') ?? 'orb'
)

export function setVisualizerMode(mode: 'orb' | 'bars'): void {
  visualizerMode.set(mode)
  SafeStorage.setItem('firmium_visualizer_mode', mode)
}

// ── Lyrics ────────────────────────────────────────────────────────────────────
export const lyricsOpen = writable(false)
export const lyricsLines = writable<LyricLine[]>([])
export const lyricsSynced = writable(false)
export const lyricsTrackId = writable<string | null>(null)
export const lyricsStatus = writable('No track playing')

// Per-word timing estimates for the active synced lyrics (karaoke fill animation).
export const lyricsWordTimings = writable<WordTiming[][]>([])

// Dominant color extracted from the current track's cover art, used for the
// lyrics panel's glow background. CSS color string (e.g. "rgb(120, 80, 60)").
export const lyricsGlowColor = writable('transparent')

// Word-by-word karaoke fill animation toggle (estimated timing, so optional).
export const lyricsWordFillEnabled = writable(SafeStorage.getItem('firmium_lyrics_word_fill') !== 'false')
export function setLyricsWordFillEnabled(enabled: boolean): void {
  lyricsWordFillEnabled.set(enabled)
  SafeStorage.setItem('firmium_lyrics_word_fill', enabled ? 'true' : 'false')
}

// ── Account modal ─────────────────────────────────────────────────────────────
export const showAccountModal = writable(false)
export function openAccountModal(): void { showAccountModal.set(true) }
export function closeAccountModal(): void { showAccountModal.set(false) }

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
  // True until the playlist is first created on the server, or until createAttempts hits the retry cap.
  createPending?: boolean
  createAttempts?: number
}

export type PlaylistSource = 'local' | 'synced' | 'server-only'

export interface UnifiedPlaylist {
  // For 'local'/'synced': the local playlist's id. For 'server-only': `server-<serverId>`.
  id: string
  name: string
  description: string
  coverArtId: string | null
  coverDataUrl: string | null
  trackCount: number
  serverId: string | null
  source: PlaylistSource
  local?: Playlist
  serverMeta?: ServerPlaylist
}

// Merges local playlists with the server's playlist list into one display list,
// matching local entries with `serverId` to their server counterpart.
export function mergePlaylists(local: Playlist[], server: ServerPlaylist[]): UnifiedPlaylist[] {
  const serverById = new Map(server.map(sp => [sp.id, sp]))
  const matchedServerIds = new Set<string>()
  const result: UnifiedPlaylist[] = local.map(p => {
    const sm = p.serverId ? serverById.get(p.serverId) : undefined
    if (sm) matchedServerIds.add(sm.id)
    return {
      id: p.id, name: p.name, description: p.description,
      coverArtId: p.coverArtId, coverDataUrl: p.coverDataUrl,
      trackCount: p.tracks.length, serverId: p.serverId,
      source: sm ? 'synced' : 'local', local: p, serverMeta: sm,
    }
  })
  for (const sp of server) {
    if (matchedServerIds.has(sp.id)) continue
    result.push({
      id: 'server-' + sp.id, name: sp.name, description: sp.comment ?? '',
      coverArtId: (sp.coverArt as string | undefined) ?? null, coverDataUrl: null,
      trackCount: sp.songCount ?? 0, serverId: sp.id, source: 'server-only', serverMeta: sp,
    })
  }
  return result
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
      const pl: Playlist = { id: _uuid(), name: String(name).trim() || 'New Playlist', description: '', coverArtId: null, coverDataUrl: null, tracks: [], serverId: null, createPending: true, createAttempts: 0 }
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
    // Records the outcome of a (re)attempt to create this playlist on the server.
    // On success, sets serverId and stops retrying. On failure, increments the
    // attempt count and stops retrying once the cap is reached.
    markCreateAttempt(id: string, success: boolean, serverId?: string): void {
      update(pls => {
        const next = pls.map(p => {
          if (p.id !== id) return p
          if (success) return { ...p, serverId: serverId ?? p.serverId, createPending: false }
          const createAttempts = (p.createAttempts ?? 0) + 1
          return { ...p, createAttempts, createPending: createAttempts < 3 }
        })
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
    moveTrack(id: string, from: number, to: number): Song[] | null {
      let newTracks: Song[] | null = null
      update(pls => {
        const next = pls.map(p => {
          if (p.id !== id) return p
          if (from < 0 || from >= p.tracks.length || to < 0 || to >= p.tracks.length || from === to) {
            newTracks = p.tracks
            return p
          }
          const tracks = [...p.tracks]
          const [moved] = tracks.splice(from, 1)
          tracks.splice(to, 0, moved)
          newTracks = tracks
          return { ...p, tracks }
        })
        _savePlaylists(next)
        return next
      })
      return newTracks
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
