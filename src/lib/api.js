import { tauriFetch, tauriInvoke } from './tauri.js'
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

// ── Data mappers ──────────────────────────────────────────────────────────────
const mapAlbum = (a) => ({
  id: a.id,
  name: a.name ?? a.title ?? 'Unknown Album',
  albumArtist: a.displayArtist ?? a.artist ?? 'Unknown Artist',
  coverArtId: a.coverArt,
  songCount: a.songCount,
  releaseType: a.releaseTypes?.[0] ?? a.releaseType,
  genres: a.genres,
  year: a.year,
  isCompilation: a.isCompilation ?? false
})

const mapArtist = (a) => ({
  id: a.id,
  name: a.name ?? 'Unknown Artist',
  albumCount: a.albumCount ?? 0
})

const mapSong = (s) => ({
  id: s.id,
  title: s.title ?? 'Unknown Track',
  artist: s.displayArtist ?? s.artist ?? 'Unknown Artist',
  duration: s.duration ?? 0,
  trackNumber: s.track,
  coverArtId: s.coverArt,
  replayGain: s.replayGain,
  bpm: s.bpm,
  comment: s.comment,
  genres: s.genres
})

// ── API layer ─────────────────────────────────────────────────────────────────
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
    return (d.albumList2?.album ?? []).map(mapAlbum)
  },

  getArtists: async (signal) => {
    const d = await Api.fetch('getArtists', {}, signal)
    const container = []
    if (d.artists?.index) d.artists.index.forEach(i => { if (Array.isArray(i.artist)) container.push(...i.artist) })
    return container.map(mapArtist)
  },

  getAlbumTracks: async (id, signal) => {
    const d = await Api.fetch('getAlbum', { id }, signal)
    const a = d.album ?? {}
    return {
      tracks: (a.song ?? []).map(mapSong),
      albumName: a.name ?? a.title ?? 'Unknown Album',
      albumArtist: a.displayArtist ?? a.artist ?? 'Unknown Artist',
      coverArtId: a.coverArt
    }
  },

  getArtistDetails: async (id, signal) => {
    const d = await Api.fetch('getArtist', { id }, signal)
    return {
      name: d.artist?.name ?? 'Unknown Artist',
      albums: (d.artist?.album ?? []).map(mapAlbum)
    }
  },

  search: async (query, signal) => {
    const d = await Api.fetch('search3', { query, albumCount: SEARCH_ALBUM_LIMIT, songCount: SEARCH_SONG_LIMIT }, signal)
    return {
      songs: (d.searchResult3?.song ?? []).map(mapSong),
      albums: (d.searchResult3?.album ?? []).map(mapAlbum)
    }
  },

  getRecentAlbums: async (size = 12, signal) => {
    const d = await Api.fetch('getAlbumList2', { type: 'recent', size }, signal)
    return (d.albumList2?.album ?? []).map(mapAlbum)
  },

  getRandomAlbums: async (size = 12, signal) => {
    const d = await Api.fetch('getAlbumList2', { type: 'random', size }, signal)
    return (d.albumList2?.album ?? []).map(mapAlbum)
  },

  getNewestAlbums: async (size = 100, signal) => {
    const d = await Api.fetch('getAlbumList2', { type: 'newest', size }, signal)
    return (d.albumList2?.album ?? []).map(mapAlbum)
  },

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
      addCover(coverId, objUrl)
      return objUrl
    })()
    setPending(coverId, promise)
  }

  try {
    const finalUrl = await promise
    if (finalUrl && !signal?.aborted) img.src = finalUrl
  } catch (e) {
    if (e.name !== 'AbortError') console.error('Cover art load error:', e)
  } finally {
    clearPending(coverId)
  }
}
