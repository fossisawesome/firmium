import { get, writable } from 'svelte/store'
import {
  audioBridge, queue, queueIdx, currentTrack,
  volume, repeatOne, repeatAll, crossfadeEnabled, crossfadeDuration, gaplessEnabled,
  playbackState, currentPosition, trackDuration, isSeeking,
  lyricsOpen, lyricsTrackId, lyricsLines, lyricsSynced, lyricsStatus,
  bumpToken, getPlayToken, recentlyPlayedSongs
} from './stores'
import { Api, OpenSubsonicRouter } from './api'
import type { Song, PlaybackState } from './types/tauri-commands'
import type { AudioBridge } from './audio-bridge'

const replayGainDb = (track: Song): number | null => {
  const rg = track.replayGain as { trackGain?: number; albumGain?: number } | undefined
  return rg?.trackGain ?? rg?.albumGain ?? null
}

// ── Play a track ──────────────────────────────────────────────────────────────

export async function playAt(idx: number): Promise<void> {
  const bridge = get(audioBridge)
  const $queue = get(queue)
  if (!bridge || idx < 0 || idx >= $queue.length) return

  // If gapless is promoting a preloaded track, scrobble the outgoing track before
  // the bridge swaps player IDs (the finished event won't fire for the old session).
  const outgoing = get(currentTrack)
  const isGaplessPromotion = get(gaplessEnabled) && !get(crossfadeEnabled) && bridge.preloadedTrackId === $queue[idx]?.id

  queueIdx.set(idx)
  const track = $queue[idx]
  if (!track) return

  const currentToken = bumpToken()

  try {
    const streamUrl = await OpenSubsonicRouter.buildUrl('stream', { id: track.id })
    if (currentToken !== getPlayToken()) return

    if (isGaplessPromotion && outgoing) Api.scrobble(outgoing.id, true)

    await bridge.play(streamUrl, track.id, replayGainDb(track))
    Api.scrobble(track.id, false)
    recentlyPlayedSongs.push(track)
    await bridge.setVolume(get(volume))
    fetchAndShowLyrics(track)

    document.title = `▶ ${track.title} - Firmium`
  } catch (e) {
    if (currentToken === getPlayToken()) {
      console.error('Playback error:', e)
    }
  }
}

// ── Crossfade to next track ───────────────────────────────────────────────────

export async function crossfadeToNext(nextIdx: number): Promise<void> {
  const bridge = get(audioBridge)
  if (!bridge) return

  const currentTrackVal = get(currentTrack)
  if (currentTrackVal) Api.scrobble(currentTrackVal.id, true)

  queueIdx.set(nextIdx)
  const nextTrack = get(currentTrack)
  if (!nextTrack) return

  const currentToken = bumpToken()

  try {
    const streamUrl = await OpenSubsonicRouter.buildUrl('stream', { id: nextTrack.id })
    if (currentToken !== getPlayToken()) return

    const fadeDurationMs = get(crossfadeDuration) * 1000
    await bridge.startCrossfadeIn(streamUrl, nextTrack.id, get(volume), fadeDurationMs, replayGainDb(nextTrack))

    Api.scrobble(nextTrack.id, false)
    recentlyPlayedSongs.push(nextTrack)
    fetchAndShowLyrics(nextTrack)
    document.title = `▶ ${nextTrack.title} - Firmium`
  } catch (e) {
    console.error('Crossfade error:', e)
  }
}

// ── Position tracking ─────────────────────────────────────────────────────────
// Driven by Rust "playback-position" events (~300ms cadence) via AudioBridge.

let _positionHandler: ((data: { position: number; duration: number }) => void) | null = null

// Per-track playback tracking state, reset at the start of each track via startPositionTracking().
interface TrackProgressState {
  cachedDuration: number | null
  crossfadeStarted: boolean
  preloadStarted: boolean
}

let _trackProgress: TrackProgressState = { cachedDuration: null, crossfadeStarted: false, preloadStarted: false }

function _handlePositionUpdate(position: number, duration: number | null): void {
  if (!_trackProgress.cachedDuration && duration != null) {
    _trackProgress.cachedDuration = duration
    trackDuration.set(duration)
  }

  if (!get(isSeeking)) currentPosition.set(position)

  if (get(lyricsOpen)) syncLyricsToPosition(position)

  const cachedDuration = _trackProgress.cachedDuration

  // Trigger crossfade when approaching end of track.
  if (!_trackProgress.crossfadeStarted && cachedDuration && get(crossfadeEnabled) && !get(repeatOne)) {
    const fadeSec = get(crossfadeDuration)
    if (position >= cachedDuration - fadeSec) {
      const $queue = get(queue)
      let nextIdx = get(queueIdx) + 1
      if (nextIdx >= $queue.length && get(repeatAll)) nextIdx = 0
      if (nextIdx < $queue.length) {
        _trackProgress.crossfadeStarted = true
        crossfadeToNext(nextIdx)
      }
    }
  }

  // Preload next track for gapless playback — crossfade off, 30s before end.
  if (!_trackProgress.preloadStarted && cachedDuration && get(gaplessEnabled) && !get(crossfadeEnabled) && !get(repeatOne)) {
    const preloadAt = Math.max(0, cachedDuration - 30)
    if (position >= preloadAt) {
      const $queue = get(queue)
      let nextIdx = get(queueIdx) + 1
      if (nextIdx >= $queue.length && get(repeatAll)) nextIdx = 0
      if (nextIdx < $queue.length) {
        _trackProgress.preloadStarted = true
        const nextTrack = $queue[nextIdx]
        if (nextTrack) {
          const rgDb = replayGainDb(nextTrack)
          OpenSubsonicRouter.buildUrl('stream', { id: nextTrack.id })
            .then(url => get(audioBridge)?.preload(url, nextTrack.id, rgDb))
            .catch(e => console.error('Preload URL error:', e))
        }
      }
    }
  }
}

export function startPositionTracking(): void {
  stopPositionTracking()
  _trackProgress = { cachedDuration: null, crossfadeStarted: false, preloadStarted: false }

  // Subscribe to Rust-emitted position events — no IPC polling overhead.
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
    const track = get(currentTrack)
    if (track) Api.scrobble(track.id, true)

    if (get(repeatOne)) {
      repeatOne.set(false)
      playAt(get(queueIdx))
    } else if (get(queueIdx) < get(queue).length - 1) {
      playAt(get(queueIdx) + 1)
    } else if (get(repeatAll)) {
      playAt(0)
    } else {
      stopPositionTracking()
      playbackState.set('stopped')
      document.title = 'Firmium'
      currentPosition.set(0)
    }
  })

  bridge.on('volumechange', (vol: number) => {
    volume.set(vol)
  })
}

// ── Lyrics fetching ───────────────────────────────────────────────────────────

export async function fetchAndShowLyrics(song: Song): Promise<void> {
  if (!song) return
  lyricsTrackId.set(song.id)
  if (!get(lyricsOpen)) return
  lyricsStatus.set('Loading lyrics…')
  lyricsLines.set([])
  try {
    const result = await Api.getLyrics(song)
    if (get(lyricsTrackId) !== song.id) { activeLyricIdx.set(-1); return }
    if (result) {
      lyricsLines.set(result.lines)
      lyricsSynced.set(result.synced)
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
