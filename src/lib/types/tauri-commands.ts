// Types matching the serde structs returned by Tauri commands in src-tauri/src/lib.rs.
// All fields are camelCase via #[serde(rename_all = "camelCase")].

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
}

export interface Artist {
  id: string
  name: string
  albumCount: number
}

export interface Song {
  id: string
  title: string
  artist: string
  album: string
  albumId?: string
  duration: number
  trackNumber?: number
  coverArtId?: string
  replayGain?: unknown
  bpm?: number
  comment?: string
  genres?: unknown
  bitRate?: number
  samplingRate?: number
  bitDepth?: number
  suffix?: string
  contentType?: string
  trackInfo?: string
}

export type PlaybackState = 'loading' | 'playing' | 'paused' | 'stopped'

export interface AudioDevice {
  name: string
  default: boolean
}

export interface ThemeColors {
  bg: string
  surface: string
  surface2: string
  border: string
  text: string
  muted: string
  accent: string
  accent_dim: string
  error: string
  font?: string
  timing?: string
}

export interface Theme {
  id: string
  name: string
  color_scheme?: string
  colors: ThemeColors
}

export interface LyricLine {
  start: number
  value: string
}

export interface LyricsResult {
  lines: LyricLine[]
  synced: boolean
}
