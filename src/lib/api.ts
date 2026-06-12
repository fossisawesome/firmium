import { tauriInvoke } from './tauri'
import { SafeStorage } from './utils'
import { get } from 'svelte/store'
import { authServer, getQueryParams } from './stores'
import type { Album, Artist, Song, LyricsResult } from './types/tauri-commands'
import { getCoverArt } from './coverCache'

// ── Constants ─────────────────────────────────────────────────────────────────
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

// ── API layer ─────────────────────────────────────────────────────────────────
export const Api = {
  getAlbums: async (_signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke<Album[]>('get_albums'),

  getArtists: async (_signal?: AbortSignal | null): Promise<Artist[]> => tauriInvoke<Artist[]>('get_artists'),

  getAlbumTracks: async (id: string, _signal?: AbortSignal | null): Promise<{ tracks: Song[]; albumName: string; albumArtist: string; coverArtId?: string }> =>
    tauriInvoke('get_album_tracks', { id }),

  getArtistDetails: async (id: string, _signal?: AbortSignal | null): Promise<{ name: string; albums: Album[] }> =>
    tauriInvoke('get_artist_details', { id }),

  // Returns artist info (bio + image) from Last.fm/MusicBrainz via the server's getArtistInfo2 endpoint.
  getArtistInfo: async (id: string, _signal?: AbortSignal | null): Promise<ArtistInfo | null> =>
    tauriInvoke('get_artist_info', { id }),

  search: async (query: string, _signal?: AbortSignal | null): Promise<{ songs: Song[]; albums: Album[] }> =>
    tauriInvoke('search', { query }),

  getRecentAlbums: async (size = 12, _signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke('get_recent_albums', { size }),

  getRandomAlbums: async (size = 12, _signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke('get_random_albums', { size }),

  getNewestAlbums: async (size = 100, _signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke('get_newest_albums', { size }),

  getGenresList: async (_signal?: AbortSignal | null): Promise<Genre[]> => tauriInvoke<Genre[]>('get_genres_list'),

  scrobble: (id: string, submission: boolean, time: number = Date.now()): void => {
    tauriInvoke('scrobble', { id, submission, time }).catch(() => {})
  },

  // ── Playlist API (OpenSubsonic) ───────────────────────────────────────────────

  // Returns all playlists visible to the current user from the server.
  getPlaylists: async (_signal?: AbortSignal | null): Promise<ServerPlaylist[]> => tauriInvoke('get_playlists'),

  // Fetches a playlist's full track list from the server.
  getPlaylistTracks: async (id: string, _signal?: AbortSignal | null): Promise<{ id: string; name: string; comment: string; songCount: number; tracks: Song[] }> =>
    tauriInvoke('get_playlist_tracks', { id }),

  // Creates a new playlist on the server and returns the created playlist object.
  createPlaylist: async (name: string): Promise<ServerPlaylist> => tauriInvoke('create_playlist', { name }),

  // Updates playlist metadata and/or adds/removes tracks by server-side ID.
  updatePlaylist: async (id: string, { name, comment, songIdsToAdd = [], songIndicesToRemove = [] }: UpdatePlaylistOptions = {}): Promise<void> =>
    tauriInvoke('update_playlist', { id, name: name ?? null, comment: comment ?? null, songIdsToAdd, songIndicesToRemove }),

  // Deletes a playlist from the server.
  deletePlaylist: async (id: string): Promise<void> => tauriInvoke('delete_playlist', { id }),

  getLyrics: async (song: Song): Promise<LyricsResult | null> => tauriInvoke('get_song_lyrics', {
    songId: song.id,
    artist: song.artist,
    title: song.title,
    duration: song.duration ?? 0,
    useLrclibFallback: SafeStorage.getItem('firmium_lrclib') !== 'false',
  })
}

// ── Cover art loader ──────────────────────────────────────────────────────────
export async function loadImage(img: HTMLImageElement | null | undefined, coverId: string | null | undefined, signal?: AbortSignal | null): Promise<void> {
  if (!img || !coverId) return
  try {
    const url = await OpenSubsonicRouter.buildUrl('getCoverArt', { id: coverId })
    const assetUrl = await getCoverArt(coverId, url)
    if (!signal?.aborted) img.src = assetUrl
  } catch (e: unknown) {
    if (!signal?.aborted) console.error('Cover art load error:', e)
  }
}
