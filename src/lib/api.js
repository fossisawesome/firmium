import { tauriFetch, tauriInvoke } from './tauri.js'
// map_albums / map_artists / map_songs are Tauri commands in main.rs
import { LrclibApi } from './lyrics.js'
import { SafeStorage } from './utils.js'
import { get } from 'svelte/store'
import { authServer, openSubsonicExtensions, getQueryParams } from './stores.js'

// ── Constants ─────────────────────────────────────────────────────────────────
export const API_PAGE_SIZE = 500
export const SEARCH_ALBUM_LIMIT = 40
export const SEARCH_SONG_LIMIT = 100
export const PLAY_ALL_CONCURRENCY = 5
const KEYRING_SERVICE = 'firmium-desktop'

// ── Keyring ───────────────────────────────────────────────────────────────────
export const Keyring = {
  save: (user, pass) => tauriInvoke('save_password', { service: KEYRING_SERVICE, user, pass }),
  load: (user) => tauriInvoke('get_password', { service: KEYRING_SERVICE, user }),
  remove: (user) => tauriInvoke('delete_password', { service: KEYRING_SERVICE, user }),
}

// ── URL builder ───────────────────────────────────────────────────────────────
export const OpenSubsonicRouter = {
  buildUrl: async (action, params = {}) => {
    const server = get(authServer)
    if (!server) return ''
    const url = new URL(`${server}/rest/${action}`)
    const combined = { ...await getQueryParams(), ...params }
    Object.entries(combined).forEach(([k, v]) => {
      if (v !== null && v !== undefined) url.searchParams.append(k, String(v))
    })
    return url.toString()
  }
}


// ── API layer ─────────────────────────────────────────────────────────────────
const _fetchAlbumList = async (type, size, signal) => {
  const d = await Api.fetch('getAlbumList2', { type, size }, signal)
  return tauriInvoke('map_albums', { albums: d.albumList2?.album ?? [] })
}

export const Api = {
  fetch: async (action, params = {}, signal = null) => {
    const url = await OpenSubsonicRouter.buildUrl(action, params)
    const res = await tauriFetch(url, signal ? { signal } : {})
    if (res.status === 401) {
      // Circular import avoided — caller (App.svelte) handles teardown on re-throw
      throw Object.assign(new Error('Session Expired'), { code: 'SESSION_EXPIRED' })
    }
    if (!res.ok) throw new Error(`HTTP Error ${res.status}`)
    const json = await res.json()
    const responseObj = json['subsonic-response']
    if (!responseObj) throw new Error('Malformed API response')
    if (responseObj.openSubsonicExtensions !== undefined) {
      openSubsonicExtensions.set(Array.isArray(responseObj.openSubsonicExtensions) ? responseObj.openSubsonicExtensions : null)
    }
    if (responseObj.status === 'failed') throw new Error(responseObj.error?.message ?? 'Engine error')
    return responseObj
  },

  getAlbums: async (signal) => {
    const d = await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: API_PAGE_SIZE }, signal)
    return tauriInvoke('map_albums', { albums: d.albumList2?.album ?? [] })
  },

  getArtists: async (signal) => {
    const d = await Api.fetch('getArtists', {}, signal)
    const raw = []
    if (d.artists?.index) d.artists.index.forEach(i => { if (Array.isArray(i.artist)) raw.push(...i.artist) })
    return tauriInvoke('map_artists', { artists: raw })
  },

  getAlbumTracks: async (id, signal) => {
    const d = await Api.fetch('getAlbum', { id }, signal)
    const a = d.album ?? {}
    const tracks = await tauriInvoke('map_songs', { songs: a.song ?? [] })
    return {
      tracks,
      albumName: a.name ?? a.title ?? 'Unknown Album',
      albumArtist: a.displayArtist ?? a.artist ?? 'Unknown Artist',
      coverArtId: a.coverArt
    }
  },

  getArtistDetails: async (id, signal) => {
    const d = await Api.fetch('getArtist', { id }, signal)
    const albums = await tauriInvoke('map_albums', { albums: d.artist?.album ?? [] })
    return {
      name: d.artist?.name ?? 'Unknown Artist',
      albums
    }
  },

  // Returns artist info (bio + image) from Last.fm/MusicBrainz via the server's getArtistInfo2 endpoint.
  getArtistInfo: async (id, signal) => {
    try {
      const d = await Api.fetch('getArtistInfo2', { id }, signal)
      const info = d.artistInfo2 ?? {}
      return {
        image: info.largeImageUrl || info.mediumImageUrl || info.smallImageUrl || null,
        bio: info.biography || null
      }
    } catch { return null }
  },

  search: async (query, signal) => {
    const d = await Api.fetch('search3', { query, albumCount: SEARCH_ALBUM_LIMIT, songCount: SEARCH_SONG_LIMIT }, signal)
    const [songs, albums] = await Promise.all([
      tauriInvoke('map_songs', { songs: d.searchResult3?.song ?? [] }),
      tauriInvoke('map_albums', { albums: d.searchResult3?.album ?? [] }),
    ])
    return { songs, albums }
  },

  getRecentAlbums: async (size = 12, signal) => _fetchAlbumList('recent', size, signal),

  getRandomAlbums: async (size = 12, signal) => _fetchAlbumList('random', size, signal),

  getNewestAlbums: async (size = 100, signal) => _fetchAlbumList('newest', size, signal),

  getGenresList: async (signal) => {
    const d = await Api.fetch('getGenres', {}, signal)
    return (d.genres?.genre ?? [])
      .map(g => ({ name: g.value ?? g.name ?? '', albumCount: g.albumCount ?? 0, songCount: g.songCount ?? 0 }))
      .filter(g => g.name)
      .sort((a, b) => b.albumCount - a.albumCount)
  },

  scrobble: (id, submission, time = Date.now()) => {
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

  getLyrics: async (song) => {
    // 1. OpenSubsonic structured lyrics (synced preferred)
    try {
      const d = await Api.fetch('getLyricsBySongId', { id: song.id })
      const list = d.lyricsList?.structuredLyrics ?? []
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
        return { lines: lyr.value.split('\n').map(v => ({ start: 0, value: v })), synced: false }
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
import { getCover, addCover, getPending, setPending, clearPending } from './coverCache.js'

export async function loadImage(img, coverId, signal) {
  if (!img || !coverId) return
  const cached = getCover(coverId)
  if (cached) { img.src = cached; return }

  let promise = getPending(coverId)
  if (!promise) {
    promise = (async () => {
      const url = await OpenSubsonicRouter.buildUrl('getCoverArt', { id: coverId })
      const res = await tauriFetch(url, { signal })
      if (!res.ok) throw new Error('Cover art unavailable')
      const blob = await res.blob()
      const objUrl = URL.createObjectURL(blob)
      addCover(coverId, objUrl, blob.size)
      return objUrl
    })()
    // Attach a no-op catch so the shared promise never becomes an unhandled rejection
    promise.catch(() => {})
    setPending(coverId, promise)
  }

  try {
    const finalUrl = await promise
    if (finalUrl && !signal?.aborted) img.src = finalUrl
  } catch (e) {
    // Tauri HTTP plugin throws "resource id X is invalid" on abort instead of AbortError
    if (e.name !== 'AbortError' && !signal?.aborted) console.error('Cover art load error:', e)
  } finally {
    clearPending(coverId)
  }
}
