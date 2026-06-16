// Local-library counterpart to `Api` (api.ts), backed by the `local_library.rs`
// Tauri commands. Used via `dataSource` (dataSource.ts) when the user isn't
// connected to a server, so the existing Album/Artist/Song UI works unchanged
// against `~/Music/Firmium`.
import { tauriInvoke } from './tauri'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { Album, Artist, Song, LyricsResult, SimilarMatch } from './types/tauri-commands'
import type { Genre, ArtistInfo, ServerPlaylist } from './api'

export const LocalApi = {
  getAlbums: async (_signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke<Album[]>('get_local_albums'),

  getArtists: async (_signal?: AbortSignal | null): Promise<Artist[]> => tauriInvoke<Artist[]>('get_local_artists'),

  getAlbumTracks: async (id: string, _signal?: AbortSignal | null): Promise<{ tracks: Song[]; albumName: string; albumArtist: string; coverArtId?: string }> =>
    tauriInvoke('get_local_album_tracks', { id }),

  getArtistDetails: async (id: string, _signal?: AbortSignal | null): Promise<{ name: string; albums: Album[] }> =>
    tauriInvoke('get_local_artist_details', { id }),

  // No bio/image lookups for local files — there's no server to ask.
  getArtistInfo: async (_id: string, _signal?: AbortSignal | null): Promise<ArtistInfo | null> => null,

  search: async (query: string, _signal?: AbortSignal | null): Promise<{ songs: Song[]; albums: Album[] }> =>
    tauriInvoke('search_local', { query }),

  getRecentAlbums: async (size = 12, _signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke('get_local_recent_albums', { size }),

  getRandomAlbums: async (size = 12, _signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke('get_local_random_albums', { size }),

  getNewestAlbums: async (size = 100, _signal?: AbortSignal | null): Promise<Album[]> => tauriInvoke('get_local_newest_albums', { size }),

  getGenresList: async (_signal?: AbortSignal | null): Promise<Genre[]> => tauriInvoke<Genre[]>('get_local_genres_list'),

  // No-ops — scrobbling/playback reporting only make sense with a server.
  scrobble: (_id: string, _submission: boolean, _time: number = Date.now()): void => {},
  reportPlayback: (_id: string, _positionMs: number, _playbackState: string): void => {},

  // sonicSimilarity is server-only; local mode has no "similar tracks".
  getSonicSimilarTracks: async (_id: string, _count?: number): Promise<SimilarMatch[]> => [],
  getSimilarTracksFallback: async (_songId: string, _artistId: string | undefined, _genre: string | undefined, _count?: number): Promise<SimilarMatch[]> => [],

  // Server-synced playlists don't exist offline; local playlists (stores.ts) work unchanged.
  getPlaylists: async (_signal?: AbortSignal | null): Promise<ServerPlaylist[]> => [],
  getPlaylistTracks: async (_id: string, _signal?: AbortSignal | null): Promise<{ id: string; name: string; comment: string; songCount: number; tracks: Song[] }> => {
    throw new Error('Server playlists are unavailable offline')
  },
  createPlaylist: async (_name: string): Promise<ServerPlaylist> => {
    throw new Error('Connect to a server to create synced playlists')
  },
  updatePlaylist: async (): Promise<void> => {},
  deletePlaylist: async (): Promise<void> => {},

  getLyrics: async (_song: Song): Promise<LyricsResult | null> => null,
}

// Copies dropped audio files/folders into `~/Music/Firmium`, returning the number imported.
export const importLocalFiles = (paths: string[]): Promise<number> => tauriInvoke<number>('import_local_files', { paths })

// Returns the absolute file path of a locally-downloaded track matching title + (album or artist),
// or null if not found. Used to prefer the local copy over streaming.
export const findLocalMatch = (title: string, artist: string, album: string): Promise<string | null> =>
  tauriInvoke<string | null>('find_local_match', { title, artist, album })

// Resolves a `local:<hash>` song id to its absolute file path, for playback.
export const getLocalTrackPath = (id: string): Promise<string> => tauriInvoke<string>('get_local_track_path', { id })

// Loads embedded cover art for a `local:<hash>` cover id into an <img>.
export async function loadLocalImage(img: HTMLImageElement | null | undefined, coverId: string | null | undefined, signal?: AbortSignal | null): Promise<void> {
  if (!img || !coverId) return
  try {
    const path = await tauriInvoke<string>('get_local_cover_art', { id: coverId })
    if (!signal?.aborted) img.src = convertFileSrc(path)
  } catch (e: unknown) {
    if (!signal?.aborted) console.error('Local cover art load error:', e)
  }
}
