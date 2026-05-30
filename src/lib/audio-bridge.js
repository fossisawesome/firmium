import { tauriInvoke } from './tauri.js'
import { listen } from '@tauri-apps/api/event'

/**
 * Audio Bridge — Frontend interface to the Rust rodio audio backend.
 *
 * Events:
 *   'statechange' (state: string)  — 'loading' | 'playing' | 'paused' | 'stopped'
 *   'finished'    ()               — track played to completion
 *   'volumechange'(vol: number)    — volume changed
 *   'error'       (msg: string)    — playback error
 */
export class AudioBridge {
  constructor() {
    this.currentPlayerId = null
    this.listeners = new Map()
    this.lastKnownState = null
    this._hasStartedPlaying = false
    this.preloadedPlayerId = null
    this.preloadedTrackId = null
    this._unlistenState = null
    this._unlistenFinished = null
    this._unlistenPosition = null
    this._initListeners()
  }

  // ── Event listeners ────────────────────────────────────────────────────────

  async _initListeners() {
    // Rust emits global events via app_handle.emit() — listen() wraps data in { payload }.
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

    // Position events from Rust's finish-watcher thread (~300ms cadence).
    this._unlistenPosition = await listen('playback-position', ({ payload }) => {
      if (payload.playerId !== this.currentPlayerId) return
      this.emit('position', { position: payload.position, duration: payload.duration })
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
      // Promote a preloaded session instead of starting a fresh fetch — gapless.
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
        await tauriInvoke('resume_playback', { playerId: preloadedId })
        return preloadedId
      }

      if (this.currentPlayerId) await this.stop()
      const playerId = await tauriInvoke('play_stream', { streamUrl, trackId, replayGainDb })
      this.currentPlayerId = playerId
      this._hasStartedPlaying = false
      this.lastKnownState = 'loading'
      this.emit('statechange', 'loading')
      return playerId
    } catch (err) {
      this.emit('error', `Playback failed: ${err}`)
      throw err
    }
  }

  async preload(streamUrl, trackId, replayGainDb = null) {
    if (this.preloadedPlayerId && this.preloadedTrackId !== trackId) {
      const old = this.preloadedPlayerId
      this.preloadedPlayerId = null
      this.preloadedTrackId = null
      try { await tauriInvoke('stop_playback', { playerId: old }) } catch (_) {}
    }
    if (this.preloadedTrackId === trackId) return
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

  async setVolume(vol) {
    const normalized = Math.max(0, Math.min(1, Number(vol)))
    if (!this.currentPlayerId) return
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

  async getDuration() {
    if (!this.currentPlayerId) return null
    try { return await tauriInvoke('get_track_duration', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get duration failed:', err)
      return null
    }
  }

  async isFinished() {
    if (!this.currentPlayerId) return true
    try { return await tauriInvoke('is_playback_finished', { playerId: this.currentPlayerId }) } catch (err) {
      return false
    }
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  destroy() {
    if (this._unlistenState) { this._unlistenState(); this._unlistenState = null }
    if (this._unlistenFinished) { this._unlistenFinished(); this._unlistenFinished = null }
    if (this._unlistenPosition) { this._unlistenPosition(); this._unlistenPosition = null }
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
