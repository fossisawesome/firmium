import { get } from 'svelte/store'
import {
  audioBridge, queue, queueIdx, currentTrack,
  volume, repeatOne, repeatAll, crossfadeEnabled, crossfadeDuration,
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

  queueIdx.set(idx)
  const track = $queue[idx]
  if (!track) return

  const currentToken = bumpToken()

  try {
    const streamUrl = await OpenSubsonicRouter.buildUrl('stream', { id: track.id })
    if (currentToken !== getPlayToken()) return

    await bridge.play(streamUrl, track.id)
    Api.scrobble(track.id, false)
    recentlyPlayedSongs.push(track)
    await bridge.setVolume(get(volume))

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
    await bridge.startCrossfadeIn(streamUrl, nextTrack.id, get(volume), fadeDurationMs)

    Api.scrobble(nextTrack.id, false)
    recentlyPlayedSongs.push(nextTrack)
    document.title = `▶ ${nextTrack.title} - Firmium`
  } catch (e) {
    console.error('Crossfade error:', e)
  }
}

// ── Position tracking interval ────────────────────────────────────────────────

let _positionInterval = null
let _cachedDuration = null
let _crossfadeStarted = false

export function startPositionTracking() {
  stopPositionTracking()
  _cachedDuration = null
  _crossfadeStarted = false

  _positionInterval = setInterval(async () => {
    const bridge = get(audioBridge)
    if (!bridge || !get(currentTrack)) { stopPositionTracking(); return }

    try {
      const position = await bridge.getCurrentPosition()
      if (!_cachedDuration) _cachedDuration = await bridge.getDuration()

      if (!get(isSeeking)) {
        currentPosition.set(position)
        if (_cachedDuration) trackDuration.set(_cachedDuration)
      }

      // Sync lyrics to current position.
      syncLyricsToPosition(position)

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
    } catch (err) {
      console.error('Position update failed:', err)
    }
  }, 250)
}

export function stopPositionTracking() {
  if (_positionInterval) { clearInterval(_positionInterval); _positionInterval = null }
}

// ── Lyrics sync ───────────────────────────────────────────────────────────────

// Called every 250ms from position tracking. Updates lyricsActiveIdx store.
export const lyricsActiveIdx = { _val: -1 }
let _lyricsActiveIdxStore = null

import { writable } from 'svelte/store'
export const activeLyricIdx = writable(-1)

function syncLyricsToPosition(positionSec) {
  const $lyricsOpen = get(lyricsOpen)
  const $lyricsSynced = get(lyricsSynced)
  const $lyricsLines = get(lyricsLines)
  if (!$lyricsOpen || !$lyricsSynced || !$lyricsLines.length) return

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
    if (state === 'playing') startPositionTracking()
    else stopPositionTracking()
  })

  bridge.on('finished', () => {
    const track = get(currentTrack)
    if (track) Api.scrobble(track.id, true)

    if (get(repeatOne)) {
      playAt(get(queueIdx))
    } else if (get(queueIdx) < get(queue).length - 1) {
      playAt(get(queueIdx) + 1)
    } else if (get(repeatAll)) {
      playAt(0)
    } else {
      stopPositionTracking()
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
