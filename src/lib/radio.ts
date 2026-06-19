// Smart Radio seeding — shared by auto-continue, Mood Mix, and Start Radio.
// The seeding cascade lives here so all three features behave identically:
//   1. Server similar tracks (sonicSimilarity, else genre/Last.fm-artist fallback)
//   2. Local library filtered by genre + BPM (±15) of the seed track
import { get } from 'svelte/store'
import { listen } from '@tauri-apps/api/event'
import { Api } from './api'
import { tauriInvoke } from './tauri'
import { hasSonicSimilarity, queue, currentTrack } from './stores'
import type { Song } from './types/tauri-commands'

const RADIO_BATCH = 10
const BPM_TOLERANCE = 15
const POOL_SIZE = 500

// Tracks played this session, excluded from auto-continue seeding so radio
// doesn't loop back over songs already heard.
const sessionPlayedIds = new Set<string>()

// Reads the first genre name from a Song's raw `genres` field (array of strings,
// array of {name}, or a single string depending on the server).
function genreOf(song: Song): string | undefined {
  const g = song.genres
  if (Array.isArray(g) && g.length) {
    const first = g[0]
    if (typeof first === 'string') return first
    if (first && typeof first === 'object' && 'name' in (first as object)) return String((first as { name: unknown }).name)
  }
  if (typeof g === 'string') return g
  return undefined
}

function shuffleInPlace<T>(arr: T[]): T[] {
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[arr[i], arr[j]] = [arr[j], arr[i]]
  }
  return arr
}

async function libraryPool(genre: string | undefined): Promise<Song[]> {
  return genre
    ? tauriInvoke<Song[]>('get_songs_by_genre', { genre, count: POOL_SIZE })
    : tauriInvoke<Song[]>('get_random_songs', { count: POOL_SIZE, genre: null })
}

// Seeding cascade. Returns up to `count` tracks similar to `seed`, excluding the
// seed itself and anything in `exclude`.
export async function seedFrom(seed: Song, exclude: Set<string> = new Set(), count = RADIO_BATCH): Promise<Song[]> {
  const skip = new Set<string>([seed.id, ...exclude])
  const out: Song[] = []
  const push = (s: Song) => { if (s && !skip.has(s.id)) { skip.add(s.id); out.push(s) } }

  // 1. Server similar tracks.
  try {
    const matches = get(hasSonicSimilarity)
      ? await Api.getSonicSimilarTracks(seed.id, count * 2)
      : await Api.getSimilarTracksFallback(seed.id, seed.artistId, genreOf(seed), count * 2)
    matches.forEach(m => push(m.song))
  } catch (_) { /* fall through to local filter */ }

  // 2. Local library by genre + BPM.
  if (out.length < count) {
    const seedBpm = typeof seed.bpm === 'number' ? seed.bpm : null
    try {
      const pool = await libraryPool(genreOf(seed))
      const filtered = pool.filter(s =>
        seedBpm == null || (typeof s.bpm === 'number' && Math.abs(s.bpm - seedBpm) <= BPM_TOLERANCE))
      shuffleInPlace(filtered).forEach(push)
    } catch (_) { /* leave whatever step 1 produced */ }
  }

  return out.slice(0, count)
}

// ── Mood Mix ──────────────────────────────────────────────────────────────────
export type Energy = 'chill' | 'mid' | 'high'

function inBand(bpm: number | undefined, energy: Energy): boolean {
  if (typeof bpm !== 'number') return false
  if (energy === 'chill') return bpm < 80
  if (energy === 'mid') return bpm >= 80 && bpm <= 120
  return bpm > 120
}

// Shuffled queue of library tracks matching an energy band (+ optional genre).
export async function buildMoodMix(energy: Energy, genre?: string): Promise<Song[]> {
  const pool = await libraryPool(genre)
  return shuffleInPlace(pool.filter(s => inBand(s.bpm, energy)))
}

// ── Start Radio (seed an item, play immediately) ───────────────────────────────
export async function startRadio(seed: Song): Promise<Song[]> {
  const seeded = await seedFrom(seed, new Set(), RADIO_BATCH)
  const tracks = [seed, ...seeded]
  if (tracks.length) await tauriInvoke('set_queue', { songs: tracks, startIdx: 0 })
  return tracks
}

// ── Auto-continue ───────────────────────────────────────────────────────────────
let _radioBusy = false

// Wires the session-played tracker and the 'queue-exhausted' listener that Rust
// emits when a queue ends and auto-continue is on. Call once at app startup.
export function startAutoContinue(): () => void {
  const unsubTrack = currentTrack.subscribe(t => { if (t?.id) sessionPlayedIds.add(t.id) })

  let unlistenEvent: (() => void) | undefined
  listen<Song>('queue-exhausted', async ({ payload: seed }) => {
    if (_radioBusy || !seed) return
    _radioBusy = true
    try {
      const existing = get(queue)
      const seeded = await seedFrom(seed, sessionPlayedIds, RADIO_BATCH)
      if (!seeded.length) return
      await tauriInvoke('set_queue', { songs: [...existing, ...seeded], startIdx: existing.length })
    } catch (e) {
      console.error('Auto-continue seeding failed:', e)
    } finally {
      _radioBusy = false
    }
  }).then(fn => { unlistenEvent = fn })

  return () => { unsubTrack(); unlistenEvent?.() }
}
