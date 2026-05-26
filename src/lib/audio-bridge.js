import { tauriInvoke } from './tauri.js'
import { listen } from '@tauri-apps/api/event'

/**
 * Audio Bridge — Frontend interface to the native Rust audio backend.
 *
 * State changes and track completion are delivered via Tauri events emitted
 * by the Rust layer instead of JS polling:
 *   "playback-state-changed" { playerId, state }  — 'loading' | 'playing' | 'paused'
 *   "playback-finished"      { playerId }          — track played to completion
 *
 * Bridge events re-emitted to callers:
 *   'statechange' (state: string)  — 'loading' | 'playing' | 'paused' | 'stopped'
 *   'finished'    ()               — track played to completion
 *   'volumechange'(vol: number)    — volume was changed
 *   'error'       (msg: string)    — a playback error occurred
 */
export class AudioBridge {
  constructor() {
    this.currentPlayerId = null
    this.listeners = new Map()
    this.lastKnownState = null
    this._hasStartedPlaying = false
    // Gapless preload — stores the next track's player before it's needed.
    this.preloadedPlayerId = null
    this.preloadedTrackId = null
    // Tauri event unlisten functions — called in destroy().
    this._unlistenState = null
    this._unlistenFinished = null
    this._initListeners()
  }

  // ── Tauri event listeners ──────────────────────────────────────────────────

  // Set up persistent listeners for Rust-emitted playback events.
  // These replace the previous 750ms setInterval polling loop.
  async _initListeners() {
    this._unlistenState = await listen('playback-state-changed', ({ payload }) => {
      if (payload.playerId !== this.currentPlayerId) return
      const state = payload.state
      if (state === 'playing') this._hasStartedPlaying = true
      if (state !== this.lastKnownState) {
        this.lastKnownState = state
        this.emit('statechange', state)
      }
    })

    this._unlistenFinished = await listen('playback-finished', ({ payload }) => {
      if (payload.playerId !== this.currentPlayerId) return
      this.lastKnownState = 'finished'
      this.currentPlayerId = null
      this.emit('finished')
    })
  }

  // ── Event emitter ──────────────────────────────────────────────────────────

  on(event, callback) {
    if (!this.listeners.has(event)) this.listeners.set(event, [])
    this.listeners.get(event).push(callback)
  }

  off(event, callback) {
    if (this.listeners.has(event)) {
      const callbacks = this.listeners.get(event)
      const idx = callbacks.indexOf(callback)
      if (idx > -1) callbacks.splice(idx, 1)
    }
  }

  emit(event, data) {
    if (this.listeners.has(event)) {
      this.listeners.get(event).forEach(cb => cb(data))
    }
  }

  // ── Playback controls ──────────────────────────────────────────────────────

  async play(streamUrl, trackId, replayGainDb = null) {
    try {
      // If this track was preloaded, promote the preloaded session instead of
      // starting a fresh fetch+decode — this is what makes gapless work.
      if (this.preloadedPlayerId && this.preloadedTrackId === trackId) {
        const preloadedId = this.preloadedPlayerId
        this.preloadedPlayerId = null
        this.preloadedTrackId = null
        if (this.currentPlayerId) {
          const old = this.currentPlayerId
          this.currentPlayerId = null
          try { await tauriInvoke('stop_playback', { playerId: old }) } catch (_) {}
        }
        this.currentPlayerId = preloadedId
        this._hasStartedPlaying = false
        this.lastKnownState = 'loading'
        this.emit('statechange', 'loading')
        // resume_playback emits 'playing' from Rust once the sink is unpaused.
        await tauriInvoke('resume_playback', { playerId: preloadedId })
        return preloadedId
      }

      if (this.currentPlayerId) await this.stop()
      const playerId = await tauriInvoke('play_stream', { streamUrl, trackId, replayGainDb })
      this.currentPlayerId = playerId
      this._hasStartedPlaying = false
      this.lastKnownState = 'loading'
      // Rust emits 'loading' immediately on play_stream, and 'playing' after decode.
      this.emit('statechange', 'loading')
      return playerId
    } catch (err) {
      this.emit('error', `Playback failed: ${err}`)
      throw err
    }
  }

  // Pre-fetch and decode a track in the background without starting audio output.
  // Call play() with the same trackId to promote it instantly when needed.
  async preload(streamUrl, trackId, replayGainDb = null) {
    // Drop any existing preload for a different track.
    if (this.preloadedPlayerId && this.preloadedTrackId !== trackId) {
      const old = this.preloadedPlayerId
      this.preloadedPlayerId = null
      this.preloadedTrackId = null
      try { await tauriInvoke('stop_playback', { playerId: old }) } catch (_) {}
    }
    if (this.preloadedTrackId === trackId) return // already preloading this track
    try {
      const playerId = await tauriInvoke('preload_stream', { streamUrl, trackId, replayGainDb })
      this.preloadedPlayerId = playerId
      this.preloadedTrackId = trackId
    } catch (err) {
      console.error('Preload failed:', err)
    }
  }

  async pause() {
    if (!this.currentPlayerId) return
    try {
      // Rust emits 'paused' state-change event after the sink is paused.
      await tauriInvoke('pause_playback', { playerId: this.currentPlayerId })
      this.lastKnownState = 'paused'
      this.emit('statechange', 'paused')
    } catch (err) {
      this.emit('error', `Pause failed: ${err}`)
    }
  }

  async resume() {
    if (!this.currentPlayerId) return
    try {
      // Rust emits 'playing' state-change event after the sink resumes.
      await tauriInvoke('resume_playback', { playerId: this.currentPlayerId })
      this.lastKnownState = 'playing'
      this._hasStartedPlaying = true
      this.emit('statechange', 'playing')
    } catch (err) {
      this.emit('error', `Resume failed: ${err}`)
    }
  }

  async stop() {
    if (!this.currentPlayerId) return
    // Discard any preloaded session — user is explicitly stopping playback.
    if (this.preloadedPlayerId) {
      const preId = this.preloadedPlayerId
      this.preloadedPlayerId = null
      this.preloadedTrackId = null
      try { await tauriInvoke('stop_playback', { playerId: preId }) } catch (_) {}
    }
    const idToStop = this.currentPlayerId
    this.currentPlayerId = null
    this.lastKnownState = 'stopped'
    this._hasStartedPlaying = false
    try {
      await tauriInvoke('stop_playback', { playerId: idToStop })
      this.emit('statechange', 'stopped')
    } catch (err) {
      this.emit('error', `Stop failed: ${err}`)
    }
  }

  // Cross-fade from the current session into a new stream over fadeDurationMs milliseconds.
  // Volume ramping runs natively in a Rust async task — no per-step IPC calls.
  async startCrossfadeIn(streamUrl, trackId, targetVolume, fadeDurationMs, replayGainDb = null) {
    const oldPlayerId = this.currentPlayerId
    try {
      const newPlayerId = await tauriInvoke('crossfade_to', {
        oldPlayerId: oldPlayerId ?? '',
        streamUrl,
        trackId,
        fadeDurationMs: Math.round(fadeDurationMs),
        targetVolume,
        replayGainDb,
      })
      // Guard against a concurrent play() call that changed currentPlayerId while awaiting.
      if (this.currentPlayerId !== oldPlayerId) {
        tauriInvoke('stop_playback', { playerId: newPlayerId }).catch(() => {})
        return
      }
      this.currentPlayerId = newPlayerId
      this._hasStartedPlaying = false
      this.lastKnownState = 'loading'
      this.emit('statechange', 'loading')
    } catch (err) {
      this.emit('error', `Crossfade failed: ${err}`)
      throw err
    }
  }

  // ── Volume ─────────────────────────────────────────────────────────────────

  async setVolume(volume) {
    if (!this.currentPlayerId) return
    const normalized = Math.max(0, Math.min(1, Number(volume)))
    try {
      await tauriInvoke('set_volume', { playerId: this.currentPlayerId, volume: normalized })
      this.emit('volumechange', normalized)
    } catch (err) {
      this.emit('error', `Volume change failed: ${err}`)
    }
  }

  async getVolume() {
    if (!this.currentPlayerId) return null
    try { return await tauriInvoke('get_volume', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get volume failed:', err)
      return null
    }
  }

  async seek(position) {
    if (!this.currentPlayerId) return
    try { await tauriInvoke('seek_position', { playerId: this.currentPlayerId, position: Math.max(0, Number(position) || 0) }) } catch (err) {
      console.warn('Seek not supported for this stream:', err)
    }
  }

  async getCurrentPosition() {
    if (!this.currentPlayerId) return 0
    try { return await tauriInvoke('get_current_position', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get position failed:', err)
      return 0
    }
  }

  // ── State queries (kept for diagnostics; state is now driven by events) ────

  async getDuration() {
    if (!this.currentPlayerId) return null
    try { return await tauriInvoke('get_track_duration', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get duration failed:', err)
      return null
    }
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  destroy() {
    if (this._unlistenState) { this._unlistenState(); this._unlistenState = null }
    if (this._unlistenFinished) { this._unlistenFinished(); this._unlistenFinished = null }
    if (this.preloadedPlayerId) {
      const preId = this.preloadedPlayerId
      this.preloadedPlayerId = null
      this.preloadedTrackId = null
      tauriInvoke('stop_playback', { playerId: preId }).catch(() => {})
    }
    if (this.currentPlayerId) this.stop().catch(() => {})
    this.listeners.clear()
  }
}
