import { get, writable } from 'svelte/store'
import {
  audioBridge, lyricsOpen, lyricsTrackId, lyricsLines, lyricsSynced, lyricsStatus,
  lyricsWordTimings, lyricsGlowColor,
  playbackState, currentPosition, trackDuration, isSeeking, queue, queueIdx,
  crossfadeEnabled, crossfadeDuration, gaplessEnabled, repeatOne, volume, replayGainEnabled,
} from './stores'
import { Api, OpenSubsonicRouter } from './api'
import { tauriInvoke } from './tauri'
import type { Song, PlaybackState, LyricLine, WordTiming, CoverColorsResult } from './types/tauri-commands'
import type { AudioBridge } from './audio-bridge'

// ── Position tracking ─────────────────────────────────────────────────────────
// Driven by Rust "playback-position" events (~300ms cadence) via AudioBridge.

let _positionHandler: ((data: { position: number; duration: number }) => void) | null = null
let _crossfadeStarted = false
let _preloadStarted = false
let _lastQueueIdx = -1

function _handlePositionUpdate(position: number, duration: number | null): void {
  if (duration != null) trackDuration.set(duration)
  if (!get(isSeeking)) currentPosition.set(position)
  if (get(lyricsOpen)) syncLyricsToPosition(position)

  // Check for track changes (reset per-track flags)
  const currentQueueIdx = get(queueIdx)
  if (currentQueueIdx !== _lastQueueIdx) {
    _lastQueueIdx = currentQueueIdx
    _crossfadeStarted = false
    _preloadStarted = false
  }

  if (duration == null || duration <= 0) return

  const bridge = get(audioBridge)
  if (!bridge) return

  // Crossfade trigger: position >= duration - crossfadeDuration
  if (!_crossfadeStarted && get(crossfadeEnabled) && !get(repeatOne)) {
    const crossfadeSecs = get(crossfadeDuration)
    if (position >= duration - crossfadeSecs) {
      _crossfadeStarted = true
      const $queue = get(queue)
      const $idx = get(queueIdx)
      const nextSong = $queue[$idx + 1] ?? null
      if (nextSong) {
        const targetVolume = get(volume)
        const fadeDurationMs = crossfadeSecs * 1000
        const rg = nextSong.replayGain as { albumGain?: number; trackGain?: number } | null | undefined
        const replayGainDb = get(replayGainEnabled) ? (rg?.albumGain ?? rg?.trackGain ?? null) : null
        OpenSubsonicRouter.buildUrl('stream', { id: nextSong.id })
          .then(url => bridge.startCrossfadeIn(url, nextSong.id, targetVolume, fadeDurationMs, replayGainDb))
          .catch(() => {})
      }
    }
  }

  // Preload trigger: position >= duration - 30 (gapless preload window)
  if (!_preloadStarted && get(gaplessEnabled) && !get(repeatOne)) {
    if (position >= duration - 30) {
      _preloadStarted = true
      const $queue = get(queue)
      const $idx = get(queueIdx)
      const nextSong = $queue[$idx + 1] ?? null
      if (nextSong) {
        const rg = nextSong.replayGain as { albumGain?: number; trackGain?: number } | null | undefined
        const replayGainDb = get(replayGainEnabled) ? (rg?.albumGain ?? rg?.trackGain ?? null) : null
        OpenSubsonicRouter.buildUrl('stream', { id: nextSong.id })
          .then(url => bridge.preload(url, nextSong.id, replayGainDb))
          .catch(() => {})
      }
    }
  }
}

export function startPositionTracking(): void {
  stopPositionTracking()
  _lastQueueIdx = get(queueIdx)
  _crossfadeStarted = false
  _preloadStarted = false
  const bridge = get(audioBridge)
  if (!bridge) return
  _positionHandler = ({ position, duration }) => _handlePositionUpdate(position, duration)
  bridge.on('position', _positionHandler)
}

export function stopPositionTracking(): void {
  if (_positionHandler) {
    const bridge = get(audioBridge)
    if (bridge) bridge.off('position', _positionHandler)
    _positionHandler = null
  }
}

// ── Lyrics sync ───────────────────────────────────────────────────────────────

export const activeLyricIdx = writable(-1)

function syncLyricsToPosition(positionSec: number): void {
  if (!get(lyricsSynced)) return
  const $lyricsLines = get(lyricsLines)
  if (!$lyricsLines.length) return

  const posMs = positionSec * 1000
  let newIdx = -1
  for (let i = 0; i < $lyricsLines.length; i++) {
    if ($lyricsLines[i].start <= posMs) newIdx = i
    else break
  }
  const current = get(activeLyricIdx)
  if (newIdx !== current) activeLyricIdx.set(newIdx)
}

// ── Bridge event wiring ───────────────────────────────────────────────────────

export function wireBridgeEvents(bridge: AudioBridge): void {
  bridge.on('statechange', (state: PlaybackState) => {
    playbackState.set(state)
    if (state === 'playing') {
      startPositionTracking()
    } else {
      stopPositionTracking()
    }
  })

  bridge.on('finished', () => {
    stopPositionTracking()
    currentPosition.set(0)
  })

  bridge.on('volumechange', (vol: number) => {
    // Volume changes from the device layer are reflected back; no localStorage update needed here.
  })
}

// ── Lyrics fetching ───────────────────────────────────────────────────────────

// Estimates per-word timing from line-level LRC timestamps, distributing each
// line's duration (to the next line's start, or track end for the last line)
// proportionally across word character lengths.
export function computeWordTimings(lines: LyricLine[], trackDurationMs: number): WordTiming[][] {
  return lines.map((line, i) => {
    const lineEnd = i < lines.length - 1
      ? lines[i + 1].start
      : Math.max(trackDurationMs, line.start)
    const words = line.value.split(/\s+/).filter(w => w.length > 0)
    if (words.length === 0) return []

    const weights = words.map((w, idx) => w.length + (idx < words.length - 1 ? 1 : 0))
    const totalWeight = weights.reduce((a, b) => a + b, 0) || 1
    const span = Math.max(0, lineEnd - line.start)

    let cursor = line.start
    return words.map((text, idx) => {
      const startMs = cursor
      const endMs = startMs + (weights[idx] / totalWeight) * span
      cursor = endMs
      return { text, startMs, endMs }
    })
  })
}

async function updateLyricsGlow(song: Song): Promise<void> {
  if (!song.coverArtId) { lyricsGlowColor.set('transparent'); return }
  try {
    const url = await OpenSubsonicRouter.buildUrl('getCoverArt', { id: song.coverArtId })
    const result = await tauriInvoke<CoverColorsResult>('extract_cover_colors', { coverId: song.coverArtId, url })
    if (get(lyricsTrackId) !== song.id) return
    const c = result?.dominant
    lyricsGlowColor.set(c ? `rgb(${c.r}, ${c.g}, ${c.b})` : 'transparent')
  } catch (e) {
    console.warn('Lyrics glow color extraction failed:', e)
  }
}

export async function fetchAndShowLyrics(song: Song): Promise<void> {
  if (!song) return
  lyricsTrackId.set(song.id)
  if (!get(lyricsOpen)) return
  lyricsStatus.set('Loading lyrics…')
  lyricsLines.set([])
  lyricsWordTimings.set([])
  updateLyricsGlow(song)
  try {
    const result = await Api.getLyrics(song)
    if (get(lyricsTrackId) !== song.id) { activeLyricIdx.set(-1); return }
    if (result) {
      lyricsLines.set(result.lines)
      lyricsSynced.set(result.synced)
      lyricsWordTimings.set(result.synced ? computeWordTimings(result.lines, (song.duration ?? 0) * 1000) : [])
      activeLyricIdx.set(-1)
    } else {
      lyricsStatus.set('No lyrics available for this track')
    }
  } catch (e) {
    if (get(lyricsTrackId) === song.id) {
      lyricsStatus.set('Failed to load lyrics')
      console.error('Lyrics fetch error:', e)
    }
  }
}
