import { get, writable } from 'svelte/store'
import {
  audioBridge, queue, queueIdx, currentTrack,
  volume, repeatOne, repeatAll, crossfadeEnabled, crossfadeDuration, gaplessEnabled,
  playbackState, currentPosition, trackDuration, isSeeking,
  lyricsOpen, lyricsTrackId, lyricsLines, lyricsSynced, lyricsStatus,
  bumpToken, getPlayToken, recentlyPlayedSongs, authServer, getQueryParams
} from './stores.js'
import { Api, OpenSubsonicRouter } from './api.js'
import { initNowPlaying, updateNowPlaying, updateNowPlayingState, clearNowPlaying } from './nowPlaying.js'
import { isMobile } from './platform.js'

// ── Play a track ──────────────────────────────────────────────────────────────

export async function playAt(idx) {
  const bridge = get(audioBridge)
  const $queue = get(queue)
  if (!bridge || idx < 0 || idx >= $queue.length) return

  if (isMobile) {
    // Scrobble the track we're leaving before overwriting the queue player.
    const outgoing = get(currentTrack)
    if (outgoing) Api.scrobble(outgoing.id, true)

    const currentToken = bumpToken()
    try {
      // Build all stream URLs in one auth round-trip so ExoPlayer gets the full
      // playlist and can advance tracks natively even while the WebView is frozen.
      const authParams = await getQueryParams()
      if (currentToken !== getPlayToken()) return
      const server = get(authServer)
      const tracks = $queue.map(t => {
        const url = new URL(`${server}/rest/stream`)
        Object.entries({ ...authParams, id: t.id }).forEach(([k, v]) => url.searchParams.append(k, String(v)))
        return {
          streamUrl: url.toString(),
          trackId: t.id,
          replayGainDb: t.replayGain?.trackGain ?? t.replayGain?.albumGain ?? null,
        }
      })
      await bridge.setQueue(tracks, idx)
      const track = $queue[idx]
      if (!track) return
      queueIdx.set(idx)
      Api.scrobble(track.id, false)
      recentlyPlayedSongs.push(track)
      await bridge.setVolume(get(volume))
      fetchAndShowLyrics(track)
      updateNowPlaying(track, true)
      document.title = `▶ ${track.title} - Firmium`
    } catch (e) {
      if (currentToken === getPlayToken()) console.error('Queue setup error:', e)
    }
    return
  }

  // ── Desktop path ────────────────────────────────────────────────────────────

  // If gapless is promoting a preloaded track, the finished event for the outgoing
  // track will be filtered out by the player ID guard in audio-bridge, so scrobble
  // the outgoing track here before the bridge swaps player IDs.
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
    updateNowPlaying(track, true)

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
    updateNowPlaying(nextTrack, true)
    document.title = `▶ ${nextTrack.title} - Firmium`
  } catch (e) {
    console.error('Crossfade error:', e)
  }
}

// ── Position tracking interval ────────────────────────────────────────────────

let _positionInterval = null
let _cachedDuration = null
let _crossfadeStarted = false
let _preloadStarted = false

export function startPositionTracking() {
  stopPositionTracking()
  _cachedDuration = null
  _crossfadeStarted = false
  _preloadStarted = false

  _positionInterval = setInterval(async () => {
    const bridge = get(audioBridge)
    if (!bridge || !get(currentTrack)) { stopPositionTracking(); return }

    try {
      const position = await bridge.getCurrentPosition()
      if (!_cachedDuration) {
        _cachedDuration = await bridge.getDuration()
        if (_cachedDuration) trackDuration.set(_cachedDuration)
      }

      if (!get(isSeeking)) {
        currentPosition.set(position)
      }

      if (get(lyricsOpen)) syncLyricsToPosition(position)

      // On mobile, plugin finish events may not arrive — poll is_playback_finished
      // when we're near the end as a reliable fallback.
      if (isMobile && _cachedDuration && position >= _cachedDuration - 2) {
        const finished = await bridge.isFinished()
        if (finished) { bridge.emit('finished'); stopPositionTracking(); return }
      }

      // Trigger crossfade when approaching end of track (desktop only).
      if (!isMobile && !_crossfadeStarted && _cachedDuration && get(crossfadeEnabled) && !get(repeatOne)) {
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

      // Preload next track for gapless playback — desktop only, crossfade off.
      // Trigger 30 seconds before the end (or at track start for short tracks).
      if (!isMobile && !_preloadStarted && _cachedDuration && get(gaplessEnabled) && !get(crossfadeEnabled) && !get(repeatOne)) {
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
    } catch (err) {
      console.error('Position update failed:', err)
    }
  }, 250)
}

export function stopPositionTracking() {
  if (_positionInterval) { clearInterval(_positionInterval); _positionInterval = null }
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

// ── Background/foreground recovery (mobile only) ──────────────────────────────
// On Android the WebView is throttled when backgrounded — setInterval stops,
// so position tracking and finished-event processing stall. When the app
// returns to the foreground we check whether playback ended while we were gone
// and either emit 'finished' to advance the queue or restart tracking.

let _visibilityHandler = null

function _teardownVisibilityHandler() {
  if (_visibilityHandler) {
    document.removeEventListener('visibilitychange', _visibilityHandler)
    _visibilityHandler = null
  }
}

function _setupVisibilityHandler(bridge) {
  _teardownVisibilityHandler()
  if (!isMobile) return
  _visibilityHandler = async () => {
    if (document.visibilityState !== 'visible') return
    // Only act when we believe a track is active — avoids spurious 'finished' emits.
    if (!bridge.currentPlayerId) return
    if (bridge.lastKnownState !== 'playing' && bridge.lastKnownState !== 'loading') return
    try {
      const finished = await bridge.isFinished()
      if (finished) {
        bridge.emit('finished')
        return
      }
      // Still playing — restart the interval which was frozen while backgrounded.
      const state = bridge.lastKnownState
      if (state === 'playing') startPositionTracking()
    } catch (e) {
      console.error('Foreground recovery check failed:', e)
    }
  }
  document.addEventListener('visibilitychange', _visibilityHandler)
}

export function wireBridgeEvents(bridge) {
  _setupVisibilityHandler(bridge)

  // Wire notification button events from the lock screen / shade
  initNowPlaying((action) => {
    if (action === 'prev') {
      // On mobile ExoPlayer owns the queue; skip natively instead of calling playAt.
      if (isMobile) { bridge.skipToPrevious().catch(console.error); return }
      const idx = get(queueIdx); if (idx > 0) playAt(idx - 1)
    } else if (action === 'next') {
      if (isMobile) { bridge.skipToNext().catch(console.error); return }
      const idx = get(queueIdx); const len = get(queue).length
      if (idx < len - 1) playAt(idx + 1)
      else if (get(repeatAll)) playAt(0)
    } else if (action === 'togglePlayPause') {
      const state = bridge.lastKnownState
      if (state === 'playing') bridge.pause().catch(console.error)
      else if (state === 'paused') bridge.resume().catch(console.error)
    }
  })

  bridge.on('statechange', (state) => {
    playbackState.set(state)
    if (state === 'playing') {
      startPositionTracking()
      updateNowPlayingState(true)
    } else if (state === 'paused') {
      stopPositionTracking()
      updateNowPlayingState(false)
    } else {
      stopPositionTracking()
    }
  })

  // Fires on mobile when ExoPlayer advances to the next item in the playlist.
  // Handles metadata, scrobbling, and lyrics without any JS timer involvement.
  bridge.on('track-changed', ({ trackId, index }) => {
    const outgoing = get(currentTrack)
    if (outgoing) Api.scrobble(outgoing.id, true)

    queueIdx.set(index)
    const newTrack = get(currentTrack)
    if (!newTrack) return

    Api.scrobble(newTrack.id, false)
    recentlyPlayedSongs.push(newTrack)
    fetchAndShowLyrics(newTrack)
    updateNowPlaying(newTrack, true)
    document.title = `▶ ${newTrack.title} - Firmium`

    // Reset per-track position state so the tracking interval picks up fresh duration.
    _cachedDuration = null
    currentPosition.set(0)
    trackDuration.set(0)
  })

  bridge.on('finished', () => {
    const track = get(currentTrack)
    if (track) Api.scrobble(track.id, true)

    if (isMobile) {
      // On mobile the queue is exhausted — ExoPlayer already advanced as far as it can.
      if (get(repeatOne)) {
        repeatOne.set(false)
        bridge.skipToQueueIndex(get(queueIdx)).catch(console.error)
      } else if (get(repeatAll)) {
        bridge.skipToQueueIndex(0).catch(console.error)
      } else {
        stopPositionTracking()
        playbackState.set('stopped')
        clearNowPlaying()
        document.title = 'Firmium'
        currentPosition.set(0)
      }
      return
    }

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
      clearNowPlaying()
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
    if (get(lyricsTrackId) !== song.id) return
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
