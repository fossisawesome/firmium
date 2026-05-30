import { get, writable } from 'svelte/store'
import {
  audioBridge, queue, queueIdx, currentTrack,
  volume, repeatOne, repeatAll, crossfadeEnabled, crossfadeDuration, gaplessEnabled,
  playbackState, currentPosition, trackDuration, isSeeking,
  lyricsOpen, lyricsTrackId, lyricsLines, lyricsSynced, lyricsStatus,
  bumpToken, getPlayToken, recentlyPlayedSongs
} from './stores.js'
import { Api, OpenSubsonicRouter } from './api.js'

// ── Play a track ──────────────────────────────────────────────────────────────

export async function playAt(idx) {
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

    const replayGainDb = track.replayGain?.trackGain ?? track.replayGain?.albumGain ?? null
    await bridge.play(streamUrl, track.id, replayGainDb)
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

export async function crossfadeToNext(nextIdx) {
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
    const replayGainDb = nextTrack.replayGain?.trackGain ?? nextTrack.replayGain?.albumGain ?? null
    await bridge.startCrossfadeIn(streamUrl, nextTrack.id, get(volume), fadeDurationMs, replayGainDb)

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

let _positionHandler = null
let _cachedDuration = null
let _crossfadeStarted = false
let _preloadStarted = false

function _handlePositionUpdate(position, duration) {
  if (!_cachedDuration && duration != null) {
    _cachedDuration = duration
    trackDuration.set(duration)
  }

  if (!get(isSeeking)) currentPosition.set(position)

  if (get(lyricsOpen)) syncLyricsToPosition(position)

  // Trigger crossfade when approaching end of track.
  if (!_crossfadeStarted && _cachedDuration && get(crossfadeEnabled) && !get(repeatOne)) {
    const fadeSec = get(crossfadeDuration)
    if (position >= _cachedDuration - fadeSec) {
      const $queue = get(queue)
      let nextIdx = get(queueIdx) + 1
      if (nextIdx >= $queue.length && get(repeatAll)) nextIdx = 0
      if (nextIdx < $queue.length) {
        _crossfadeStarted = true
        crossfadeToNext(nextIdx)
      }
    }
  }

  // Preload next track for gapless playback — crossfade off, 30s before end.
  if (!_preloadStarted && _cachedDuration && get(gaplessEnabled) && !get(crossfadeEnabled) && !get(repeatOne)) {
    const preloadAt = Math.max(0, _cachedDuration - 30)
    if (position >= preloadAt) {
      const $queue = get(queue)
      let nextIdx = get(queueIdx) + 1
      if (nextIdx >= $queue.length && get(repeatAll)) nextIdx = 0
      if (nextIdx < $queue.length) {
        _preloadStarted = true
        const nextTrack = $queue[nextIdx]
        if (nextTrack) {
          const rgDb = nextTrack.replayGain?.trackGain ?? nextTrack.replayGain?.albumGain ?? null
          OpenSubsonicRouter.buildUrl('stream', { id: nextTrack.id })
            .then(url => get(audioBridge)?.preload(url, nextTrack.id, rgDb))
            .catch(e => console.error('Preload URL error:', e))
        }
      }
    }
  }
}

export function startPositionTracking() {
  stopPositionTracking()
  _cachedDuration = null
  _crossfadeStarted = false
  _preloadStarted = false

  // Subscribe to Rust-emitted position events — no IPC polling overhead.
  const bridge = get(audioBridge)
  if (!bridge) return
  _positionHandler = ({ position, duration }) => _handlePositionUpdate(position, duration)
  bridge.on('position', _positionHandler)
}

export function stopPositionTracking() {
  if (_positionHandler) {
    const bridge = get(audioBridge)
    if (bridge) bridge.off('position', _positionHandler)
    _positionHandler = null
  }
}

// ── Lyrics sync ───────────────────────────────────────────────────────────────

export const activeLyricIdx = writable(-1)

function syncLyricsToPosition(positionSec) {
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

export function wireBridgeEvents(bridge) {
  bridge.on('statechange', (state) => {
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

  bridge.on('volumechange', (vol) => {
    volume.set(vol)
  })
}

// ── Lyrics fetching ───────────────────────────────────────────────────────────

export async function fetchAndShowLyrics(song) {
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
