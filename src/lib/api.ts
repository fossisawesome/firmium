import { tauriFetch, tauriInvoke } from './tauri'
// map_albums / map_artists / map_songs are Tauri commands in main.rs
import { LrclibApi, type LyricsResult } from './lyrics'
import { SafeStorage } from './utils'
import { get } from 'svelte/store'
import { authServer, openSubsonicExtensions, getQueryParams } from './stores'
import type { Album, Artist, Song } from './types/tauri-commands'
import { getCover, addCover, getPending, setPending, clearPending } from './coverCache'

// ── Constants ─────────────────────────────────────────────────────────────────
export const API_PAGE_SIZE = 500
export const SEARCH_ALBUM_LIMIT = 40
export const SEARCH_SONG_LIMIT = 100
export const PLAY_ALL_CONCURRENCY = 5

// ── Keyring ───────────────────────────────────────────────────────────────────
export const Keyring = {
  save: (user: string, pass: string) => tauriInvoke('save_password', { user, pass }),
  load: (user: string) => tauriInvoke('get_password', { user }),
  remove: (user: string) => tauriInvoke('delete_password', { user }),
}

// ── URL builder ───────────────────────────────────────────────────────────────
export const OpenSubsonicRouter = {
  buildUrl: async (action: string, params: Record<string, unknown> = {}): Promise<string> => {
    const server = get(authServer)
    if (!server) return ''
    const url = new URL(`${server}/rest/${action}`)
    const combined = { ...await getQueryParams(), ...params }
    Object.entries(combined).forEach(([k, v]) => {
      if (v === null || v === undefined) return
      if (Array.isArray(v)) {
        v.forEach(item => url.searchParams.append(k, String(item)))
      } else {
        url.searchParams.append(k, String(v))
      }
    })
    return url.toString()
  }
}

export interface ServerPlaylist {
  id: string
  name: string
  comment?: string
  songCount?: number
  [key: string]: unknown
}

export interface Genre {
  name: string
  albumCount: number
  songCount: number
}

export interface ArtistInfo {
  image: string | null
  bio: string | null
}

interface UpdatePlaylistOptions {
  name?: string
  comment?: string
  songIdsToAdd?: string[]
  songIndicesToRemove?: number[]
}

// ── Raw OpenSubsonic response shapes (subset of fields we read) ────────────────
interface SubsonicArtistIndexGroup {
  artist?: unknown[]
}

interface SubsonicGenre {
  value?: string
  name?: string
  albumCount?: number
  songCount?: number
}

interface SubsonicStructuredLyricLine {
  start?: number
  value?: string
}

interface SubsonicStructuredLyrics {
  synced?: boolean
  offset?: number
  line?: SubsonicStructuredLyricLine[]
}

// ── Session expiry ───────────────────────────────────────────────────────────
// Broadcast a window event so App.svelte can clear auth and prompt for
// reconnect. Callers validating fresh credentials (e.g. the initial connect)
// pass `silent: true` to suppress the broadcast, since a rejected login isn't
// an expired session.
const sessionExpiredError = (silent = false): Error & { code: string } => {
  if (!silent) window.dispatchEvent(new CustomEvent('firmium:session-expired'))
  return Object.assign(new Error('Session Expired'), { code: 'SESSION_EXPIRED' })
}

// ── API layer ─────────────────────────────────────────────────────────────────
const _fetchAlbumList = async (type: string, size: number, signal?: AbortSignal | null): Promise<Album[]> => {
  const d = await Api.fetch('getAlbumList2', { type, size }, signal)
  return tauriInvoke<Album[]>('map_albums', { albums: d.albumList2?.album ?? [] })
}

export const Api = {
  fetch: async (action: string, params: Record<string, unknown> = {}, signal: AbortSignal | null = null, opts: { silentSessionExpiry?: boolean } = {}): Promise<any> => {
    const url = await OpenSubsonicRouter.buildUrl(action, params)
    const res = await tauriFetch(url, signal ? { signal } : {})
    if (res.status === 401) {
      throw sessionExpiredError(opts.silentSessionExpiry)
    }
    if (!res.ok) throw new Error(`HTTP Error ${res.status}`)
    const json = await res.json()
    const responseObj = json['subsonic-response']
    if (!responseObj) throw new Error('Malformed API response')
    if (responseObj.openSubsonicExtensions !== undefined) {
      openSubsonicExtensions.set(Array.isArray(responseObj.openSubsonicExtensions) ? responseObj.openSubsonicExtensions : null)
    }
    if (responseObj.status === 'failed') {
      // OpenSubsonic servers (e.g. Navidrome) return HTTP 200 with status:"failed"
      // and error code 40/41 for bad/expired credentials, not HTTP 401.
      const code = responseObj.error?.code
      if (code === 40 || code === 41) throw sessionExpiredError(opts.silentSessionExpiry)
      throw new Error(responseObj.error?.message ?? 'Engine error')
    }
    return responseObj
  },

  getAlbums: async (signal?: AbortSignal | null): Promise<Album[]> => {
    const d = await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: API_PAGE_SIZE }, signal)
    return tauriInvoke<Album[]>('map_albums', { albums: d.albumList2?.album ?? [] })
  },

  getArtists: async (signal?: AbortSignal | null): Promise<Artist[]> => {
    const d = await Api.fetch('getArtists', {}, signal)
    const raw: unknown[] = []
    if (d.artists?.index) d.artists.index.forEach((i: SubsonicArtistIndexGroup) => { if (Array.isArray(i.artist)) raw.push(...i.artist) })
    return tauriInvoke<Artist[]>('map_artists', { artists: raw })
  },

  getAlbumTracks: async (id: string, signal?: AbortSignal | null): Promise<{ tracks: Song[]; albumName: string; albumArtist: string; coverArtId?: string }> => {
    const d = await Api.fetch('getAlbum', { id }, signal)
    const a = d.album ?? {}
    const tracks = await tauriInvoke<Song[]>('map_songs', { songs: a.song ?? [] })
    return {
      tracks,
      albumName: a.name ?? a.title ?? 'Unknown Album',
      albumArtist: a.displayArtist ?? a.artist ?? 'Unknown Artist',
      coverArtId: a.coverArt
    }
  },

  getArtistDetails: async (id: string, signal?: AbortSignal | null): Promise<{ name: string; albums: Album[] }> => {
    const d = await Api.fetch('getArtist', { id }, signal)
    const albums = await tauriInvoke<Album[]>('map_albums', { albums: d.artist?.album ?? [] })
    return {
      name: d.artist?.name ?? 'Unknown Artist',
      albums
    }
  },

  // Returns artist info (bio + image) from Last.fm/MusicBrainz via the server's getArtistInfo2 endpoint.
  getArtistInfo: async (id: string, signal?: AbortSignal | null): Promise<ArtistInfo | null> => {
    try {
      const d = await Api.fetch('getArtistInfo2', { id }, signal)
      const info = d.artistInfo2 ?? {}
      return {
        image: info.largeImageUrl || info.mediumImageUrl || info.smallImageUrl || null,
        bio: info.biography || null
      }
    } catch { return null }
  },

  search: async (query: string, signal?: AbortSignal | null): Promise<{ songs: Song[]; albums: Album[] }> => {
    const d = await Api.fetch('search3', { query, albumCount: SEARCH_ALBUM_LIMIT, songCount: SEARCH_SONG_LIMIT }, signal)
    const [songs, albums] = await Promise.all([
      tauriInvoke<Song[]>('map_songs', { songs: d.searchResult3?.song ?? [] }),
      tauriInvoke<Album[]>('map_albums', { albums: d.searchResult3?.album ?? [] }),
    ])
    return { songs, albums }
  },

  getRecentAlbums: async (size = 12, signal?: AbortSignal | null): Promise<Album[]> => _fetchAlbumList('recent', size, signal),

  getRandomAlbums: async (size = 12, signal?: AbortSignal | null): Promise<Album[]> => _fetchAlbumList('random', size, signal),

  getNewestAlbums: async (size = 100, signal?: AbortSignal | null): Promise<Album[]> => _fetchAlbumList('newest', size, signal),

  getGenresList: async (signal?: AbortSignal | null): Promise<Genre[]> => {
    const d = await Api.fetch('getGenres', {}, signal)
    return (d.genres?.genre ?? [])
      .map((g: SubsonicGenre) => ({ name: g.value ?? g.name ?? '', albumCount: g.albumCount ?? 0, songCount: g.songCount ?? 0 }))
      .filter((g: Genre) => g.name)
      .sort((a: Genre, b: Genre) => b.albumCount - a.albumCount)
  },

  scrobble: (id: string, submission: boolean, time: number = Date.now()): void => {
    OpenSubsonicRouter.buildUrl('scrobble', { id, submission: String(submission), time: String(time) }).then(url => {
      if (!url) return
      tauriFetch(url)
        .then(async r => {
          const json = await r.json().catch(() => null)
          const resp = json?.['subsonic-response']
          if (!r.ok || resp?.status === 'failed') console.error(`Scrobble failed (HTTP ${r.status}):`, resp?.error ?? json)
        })
        .catch(e => console.error('Scrobble network error:', e))
    })
  },

  // ── Playlist API (OpenSubsonic) ───────────────────────────────────────────────

  // Returns all playlists visible to the current user from the server.
  getPlaylists: async (signal?: AbortSignal | null): Promise<ServerPlaylist[]> => {
    const d = await Api.fetch('getPlaylists', {}, signal)
    return d.playlists?.playlist ?? []
  },

  // Fetches a playlist's full track list from the server.
  getPlaylistTracks: async (id: string, signal?: AbortSignal | null): Promise<{ id: string; name: string; comment: string; songCount: number; tracks: Song[] }> => {
    const d = await Api.fetch('getPlaylist', { id }, signal)
    const pl = d.playlist ?? {}
    const tracks = await tauriInvoke<Song[]>('map_songs', { songs: pl.entry ?? [] })
    return { id: pl.id, name: pl.name, comment: pl.comment ?? '', songCount: pl.songCount, tracks }
  },

  // Creates a new playlist on the server and returns the created playlist object.
  createPlaylist: async (name: string): Promise<ServerPlaylist> => {
    const d = await Api.fetch('createPlaylist', { name })
    return d.playlist ?? {}
  },

  // Updates playlist metadata and/or adds/removes tracks by server-side ID.
  updatePlaylist: async (id: string, { name, comment, songIdsToAdd = [], songIndicesToRemove = [] }: UpdatePlaylistOptions = {}): Promise<void> => {
    const params: Record<string, unknown> = { playlistId: id }
    if (name !== undefined) params.name = name
    if (comment !== undefined) params.comment = comment
    if (songIdsToAdd.length) params.songIdToAdd = songIdsToAdd
    if (songIndicesToRemove.length) params.songIndexToRemove = songIndicesToRemove
    await Api.fetch('updatePlaylist', params)
  },

  // Deletes a playlist from the server.
  deletePlaylist: async (id: string): Promise<void> => {
    await Api.fetch('deletePlaylist', { id })
  },

  getLyrics: async (song: Song): Promise<LyricsResult | null> => {
    // 1. OpenSubsonic structured lyrics (synced preferred)
    try {
      const d = await Api.fetch('getLyricsBySongId', { id: song.id })
      const list: SubsonicStructuredLyrics[] = d.lyricsList?.structuredLyrics ?? []
      const best = list.find(l => l.synced) || list[0]
      if (best && best.line?.length) {
        const offset = best.offset ?? 0
        return {
          lines: best.line.map(l => ({ start: (l.start ?? 0) + offset, value: l.value ?? '' })),
          synced: best.synced ?? false
        }
      }
    } catch (_) {}
    // 2. Legacy getLyrics (plain text)
    try {
      const d = await Api.fetch('getLyrics', { artist: song.artist, title: song.title })
      const lyr = d.lyrics
      if (lyr?.value?.trim()) {
        return { lines: lyr.value.split('\n').map((v: string) => ({ start: 0, value: v })), synced: false }
      }
    } catch (_) {}
    // 3. LRCLIB external fallback
    if (SafeStorage.getItem('firmium_lrclib') !== 'false') {
      try {
        const result = await LrclibApi.getLyrics(song)
        if (result) return result
      } catch (_) {}
    }
    return null
  }
}

// ── Cover art loader ──────────────────────────────────────────────────────────
export async function loadImage(img: HTMLImageElement | null | undefined, coverId: string | null | undefined, signal?: AbortSignal | null): Promise<void> {
  if (!img || !coverId) return
  const cached = getCover(coverId)
  if (cached) { img.src = cached; return }

  let promise = getPending(coverId) as Promise<string> | null
  if (!promise) {
    promise = (async () => {
      const url = await OpenSubsonicRouter.buildUrl('getCoverArt', { id: coverId })
      // Bound the request so a hung server response can't hold the dedup slot forever.
      const timeoutCtrl = new AbortController()
      const timeoutId = setTimeout(() => timeoutCtrl.abort(), 15000)
      const fetchSignal = signal ? AbortSignal.any([signal, timeoutCtrl.signal]) : timeoutCtrl.signal
      try {
        const res = await tauriFetch(url, { signal: fetchSignal })
        if (!res.ok) throw new Error('Cover art unavailable')
        const blob = await res.blob()
        const objUrl = URL.createObjectURL(blob)
        addCover(coverId, objUrl, blob.size)
        return objUrl
      } finally {
        clearTimeout(timeoutId)
      }
    })()
    // Attach a no-op catch so the shared promise never becomes an unhandled rejection
    promise.catch(() => {})
    setPending(coverId, promise)
  }

  try {
    const finalUrl = await promise
    if (finalUrl && !signal?.aborted) img.src = finalUrl
  } catch (e: unknown) {
    // Tauri HTTP plugin throws "resource id X is invalid" on abort instead of AbortError
    if ((e as { name?: string })?.name !== 'AbortError' && !signal?.aborted) console.error('Cover art load error:', e)
  } finally {
    clearPending(coverId)
  }
}
