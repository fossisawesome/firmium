// Shapes returned by the Rust backend. All fields are camelCase unless noted.

export interface Album {
  id: string
  name: string
  albumArtist: string
  artistId?: string
  coverArtId?: string
  songCount?: number
  releaseType: string
  genres?: unknown
  year?: number
  isCompilation: boolean
  starred: boolean
}

export interface Artist {
  id: string
  name: string
  albumCount: number
}

export interface ReplayGain {
  trackGain?: number
  trackPeak?: number
  albumGain?: number
  albumPeak?: number
  baseGain?: number
  fallbackGain?: number
}

export interface Song {
  id: string
  title: string
  artist: string
  artistId?: string
  album: string
  albumId?: string
  duration: number
  trackNumber?: number
  coverArtId?: string
  replayGain?: ReplayGain
  bpm?: number
  comment?: string
  genres?: unknown
  bitRate?: number
  samplingRate?: number
  bitDepth?: number
  suffix?: string
  contentType?: string
  trackInfo?: string
  userRating?: number
  averageRating?: number
  starred: boolean
}

export interface SimilarMatch { song: Song; similarity: number }
export interface Starred { artists: Artist[]; albums: Album[]; songs: Song[] }
export interface Genre { name: string; albumCount: number; songCount: number }
export interface ArtistInfo { image: string | null; bio: string | null }

export interface AlbumTracks {
  tracks: Song[]
  albumName: string
  albumArtist: string
  coverArtId?: string
  starred: boolean
}

export interface ArtistDetails { name: string; albums: Album[] }
export interface SearchResult { songs: Song[]; albums: Album[] }

export interface PlaylistTracks {
  id: string
  name: string
  comment: string
  songCount?: number
  tracks: Song[]
}

/** Local-first playlist (synced to the server when connected). */
export interface Playlist {
  id: string
  name: string
  tracks: Song[]
  createdAt: number
  serverId?: string
  createPending: boolean
  createAttempts: number
}

/** Server playlist not linked to a local playlist. */
export interface ServerPlaylist { id: string; name: string; comment?: string; songCount?: number; coverArt?: string }
export interface PlaylistLists { local: Playlist[]; serverOnly: ServerPlaylist[] }

export interface RemotePlayQueue {
  entries: Song[]
  current: string | null
  positionMs: number | null
  changedBy: string | null
}

// ── Auth & settings ─────────────────────────────────────────────────────────
export interface SavedAccount { server: string; username: string }

export interface AuthState {
  authed: boolean
  server: string | null
  username: string | null
  accounts: SavedAccount[]
}

export interface Settings {
  themeId: string
  uiThemeId: string
  fontFamily: string
  scrollbarWidth: number
  downloadFormat: string
  lrclibEnabled: boolean
  lyricsWordFill: boolean
  windowDecorations: boolean
  volume: number
  crossfadeEnabled: boolean
  crossfadeDuration: number
  crossfadeCurve: 'linear' | 'logarithmic'
  gaplessEnabled: boolean
  replayGainEnabled: boolean
  autoContinue: boolean
  bitPerfectMode: 'off' | 'relaxed' | 'strict'
  lastfmKey: string
  lastfmSecret: string
  listenbrainzToken: string
}
export type SettingKey = keyof Settings

export interface ThemeColors {
  bg: string; surface: string; surface2: string; border: string
  text: string; muted: string; accent: string; accent_dim: string; error: string
  font?: string; timing?: string
}
export interface Theme { id: string; name: string; color_scheme?: string; colors: ThemeColors }

// ── Playback ────────────────────────────────────────────────────────────────
export type PlaybackState = 'loading' | 'playing' | 'paused' | 'stopped'

export interface QueueState {
  queue: Song[]
  queueIdx: number
  repeatOne: boolean
  repeatAll: boolean
  shuffleEnabled: boolean
  crossfadeEnabled: boolean
  crossfadeDuration: number
  crossfadeCurve: string
  gaplessEnabled: boolean
  replayGainEnabled: boolean
  volume: number
  playerId: string | null
}

export interface PlaybackStateEvent { playerId: string; state: PlaybackState }
export interface PlaybackPositionEvent { playerId: string; position: number; duration: number | null }

export interface AudioDevice { name: string; default: boolean }

// ── Lyrics / similar / colors ───────────────────────────────────────────────
export interface WordTiming { text: string; startMs: number; endMs: number }
export interface LyricLine { start: number; value: string; words: WordTiming[] }
export interface Lyrics { lines: LyricLine[]; synced: boolean }

export interface Rgb { r: number; g: number; b: number }
export interface CoverColors { orb: { primary: Rgb; secondary: Rgb; tertiary: Rgb }; dominant: Rgb | null }

// ── Equalizer ───────────────────────────────────────────────────────────────
export type EqMode = 'graphic' | 'parametric'
export interface EqBandSpec { freq: number; gain: number; q?: number }
export interface EqProfileInfo { name: string; kind: EqMode; bands: EqBandSpec[]; imported?: boolean }
export interface EqState {
  enabled: boolean
  profiles: EqProfileInfo[]
  defaultDevice: string | null
  activeProfile: string | null
  deviceProfiles: Record<string, string>
}

// ── Stats & recap ───────────────────────────────────────────────────────────
export type RecapRange = 'week' | 'month' | 'threeMonths' | 'year' | 'all' | 'custom'

export interface RecentPlay {
  trackId: string; trackTitle: string
  artistId?: string; artistName?: string
  albumId?: string; albumName?: string
  coverArtId?: string
}

export interface PlayHistorySummary {
  totalPlays: number; totalSeconds: number
  uniqueTracks: number; uniqueArtists: number; uniqueAlbums: number
  firstPlay?: number; lastPlay?: number
}

export interface RecapStats {
  from: number; to: number
  totalPlays: number; totalSeconds: number
  topTracks: { trackId: string; title: string; artist?: string; coverArtId?: string; count: number }[]
  topArtists: { artistId?: string; name: string; count: number }[]
  topAlbums: { albumId?: string; name: string; artist?: string; coverArtId?: string; count: number }[]
  topGenre?: { genre: string; count: number }
  byTimeOfDay: { morning: number; afternoon: number; evening: number; night: number }
  byDayOfWeek: number[]
  biggestDiscovery?: { trackId: string; title: string; artist?: string; coverArtId?: string; count: number; firstHeard: number }
  streak: { daysActive: number; longestStreak: number }
}

// ── Podcasts ────────────────────────────────────────────────────────────────
export interface PodcastChannel {
  id: string; feedUrl: string; title: string
  description?: string; imageUrl?: string; addedAt: number
}
export interface PodcastEpisode {
  id: string; channelId: string; guid: string; title: string
  description?: string; audioUrl: string
  durationSeconds?: number; publishedAt?: number; positionMs: number
}

export type Energy = 'chill' | 'mid' | 'high'
export type StarKind = 'song' | 'album' | 'artist'
