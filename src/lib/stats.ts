// Frontend access to the local play-history store (src-tauri/src/db.rs +
// commands/stats.rs). All data is local — no server calls. Used by the Stats
// Export settings panel and the Recap overlay.
import { tauriInvoke } from './tauri'

export interface TrackStat {
  trackId: string
  title: string
  artist: string | null
  coverArtId: string | null
  count: number
}
export interface ArtistStat {
  artistId: string | null
  name: string
  count: number
}
export interface AlbumStat {
  albumId: string | null
  name: string
  artist: string | null
  coverArtId: string | null
  count: number
}
export interface GenreStat {
  genre: string
  count: number
}
export interface TimeOfDay {
  morning: number
  afternoon: number
  evening: number
  night: number
}
export interface DiscoveryStat {
  trackId: string
  title: string
  artist: string | null
  coverArtId: string | null
  count: number
  firstHeard: number
}
export interface Streak {
  daysActive: number
  longestStreak: number
}
export interface RecapStats {
  from: number
  to: number
  totalPlays: number
  totalSeconds: number
  topTracks: TrackStat[]
  topArtists: ArtistStat[]
  topAlbums: AlbumStat[]
  topGenre: GenreStat | null
  byTimeOfDay: TimeOfDay
  byDayOfWeek: number[]
  biggestDiscovery: DiscoveryStat | null
  streak: Streak
}
export interface PlayHistorySummary {
  totalPlays: number
  totalSeconds: number
  uniqueTracks: number
  uniqueArtists: number
  uniqueAlbums: number
  firstPlay: number | null
  lastPlay: number | null
}

export type RangeId = '7d' | '30d' | '3mo' | '1y' | 'all' | 'custom'

export const RANGE_OPTIONS: { id: RangeId; label: string }[] = [
  { id: '7d', label: '7 days' },
  { id: '30d', label: '30 days' },
  { id: '3mo', label: '3 months' },
  { id: '1y', label: '1 year' },
  { id: 'all', label: 'All time' },
  { id: 'custom', label: 'Custom' },
]

/** Resolves a range id to `[fromTs, toTs]` in unix seconds. */
export function rangeToBounds(id: RangeId, customFrom?: number, customTo?: number): [number, number] {
  const now = Math.floor(Date.now() / 1000)
  const day = 86400
  switch (id) {
    case '7d': return [now - 7 * day, now]
    case '30d': return [now - 30 * day, now]
    case '3mo': return [now - 90 * day, now]
    case '1y': return [now - 365 * day, now]
    case 'all': return [0, now]
    case 'custom': return [customFrom ?? 0, customTo ?? now]
  }
}

export function getRecapStats(fromTs: number, toTs: number): Promise<RecapStats> {
  return tauriInvoke<RecapStats>('get_recap_stats', { fromTs, toTs })
}

export function getPlayHistorySummary(): Promise<PlayHistorySummary> {
  return tauriInvoke<PlayHistorySummary>('get_play_history_summary')
}

export function exportPlayHistory(format: 'csv' | 'json'): Promise<string> {
  return tauriInvoke<string>('export_play_history', { format })
}

/** Formats a duration in seconds as a human "1h 23m" / "12m" / "45s" string. */
export function formatDuration(totalSeconds: number): string {
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m`
  return `${s}s`
}

const DOW_LABELS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
export function dayOfWeekLabel(i: number): string {
  return DOW_LABELS[i] ?? ''
}
