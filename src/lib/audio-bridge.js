import { tauriInvoke } from './tauri.js'

/**
 * Audio Bridge — Frontend interface to the native Rust audio backend.
 *
 * Events emitted:
 *   'statechange' (state: string)  — 'loading' | 'playing' | 'paused' | 'stopped'
 *   'finished'    ()               — track played to completion
 *   'volumechange'(vol: number)    — volume was changed
 *   'error'       (msg: string)    — a playback error occurred
 */
export class AudioBridge {
  constructor() {
    this.currentPlayerId = null
    this.listeners = new Map()
    this.statusCheckInterval = null
    this.lastKnownState = null
    this._hasStartedPlaying = false
    this.crossfadingPlayerId = null
    this.crossfadeInterval = null
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

  async play(streamUrl, trackId) {
    try {
      if (this.currentPlayerId) await this.stop()
      const playerId = await tauriInvoke('play_stream', { streamUrl, trackId })
      this.currentPlayerId = playerId
      this._hasStartedPlaying = false
      this.lastKnownState = 'loading'
      this.startStatusMonitoring()
      this.emit('statechange', 'loading')
      return playerId
    } catch (err) {
      this.emit('error', `Playback failed: ${err}`)
      throw err
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
    if (this.crossfadeInterval) {
      clearInterval(this.crossfadeInterval)
      this.crossfadeInterval = null
    }
    if (this.crossfadingPlayerId) {
      const oldId = this.crossfadingPlayerId
      this.crossfadingPlayerId = null
      try { await tauriInvoke('stop_playback', { playerId: oldId }) } catch (_) {}
    }
    this.stopStatusMonitoring()
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

  // Crossfade from the current session into a new stream over fadeDurationMs milliseconds.
  async startCrossfadeIn(streamUrl, trackId, targetVolume, fadeDurationMs) {
    const oldPlayerId = this.currentPlayerId
    try {
      const newPlayerId = await tauriInvoke('play_stream', { streamUrl, trackId })
      if (this.currentPlayerId !== oldPlayerId) {
        try { await tauriInvoke('stop_playback', { playerId: newPlayerId }) } catch (_) {}
        return
      }
      if (this.crossfadeInterval) {
        clearInterval(this.crossfadeInterval)
        this.crossfadeInterval = null
        if (this.crossfadingPlayerId) {
          const prev = this.crossfadingPlayerId
          this.crossfadingPlayerId = null
          try { await tauriInvoke('stop_playback', { playerId: prev }) } catch (_) {}
        }
      }
      try { await tauriInvoke('set_volume', { playerId: newPlayerId, volume: 0 }) } catch (_) {}
      this.crossfadingPlayerId = oldPlayerId
      this.currentPlayerId = newPlayerId
      this._hasStartedPlaying = false
      this.lastKnownState = 'loading'
      this.startStatusMonitoring()
      this.emit('statechange', 'loading')
      const steps = 25
      const stepMs = Math.max(50, fadeDurationMs / steps)
      let step = 0
      this.crossfadeInterval = setInterval(async () => {
        step++
        const progress = Math.min(step / steps, 1)
        if (oldPlayerId) {
          try { await tauriInvoke('set_volume', { playerId: oldPlayerId, volume: targetVolume * (1 - progress) }) } catch (_) {}
        }
        try { await tauriInvoke('set_volume', { playerId: newPlayerId, volume: targetVolume * progress }) } catch (_) {}
        if (step >= steps) {
          clearInterval(this.crossfadeInterval)
          this.crossfadeInterval = null
          if (oldPlayerId) {
            this.crossfadingPlayerId = null
            try { await tauriInvoke('stop_playback', { playerId: oldPlayerId }) } catch (_) {}
          }
        }
      }, stepMs)
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

  // ── State queries ──────────────────────────────────────────────────────────

  async isFinished() {
    if (!this.currentPlayerId) return true
    if (this.lastKnownState === 'loading') return false
    try { return await tauriInvoke('is_playback_finished', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Is-finished check failed:', err)
      return false
    }
  }

  async getDuration() {
    if (!this.currentPlayerId) return null
    try { return await tauriInvoke('get_track_duration', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get duration failed:', err)
      return null
    }
  }

  async getState() {
    if (!this.currentPlayerId) return 'stopped'
    try { return await tauriInvoke('get_playback_state', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get state failed:', err)
      return this.lastKnownState || 'stopped'
    }
  }

  // ── Status monitoring ──────────────────────────────────────────────────────

  startStatusMonitoring() {
    this.stopStatusMonitoring()
    this.statusCheckInterval = setInterval(async () => {
      if (!this.currentPlayerId) { this.stopStatusMonitoring(); return }
      try {
        const currentState = await this.getState()
        if (currentState === 'playing') this._hasStartedPlaying = true
        if (currentState !== this.lastKnownState) {
          this.lastKnownState = currentState
          this.emit('statechange', currentState)
        }
        if (this._hasStartedPlaying) {
          const finished = await this.isFinished()
          if (finished && this.lastKnownState !== 'finished') {
            this.lastKnownState = 'finished'
            this.stopStatusMonitoring()
            this.currentPlayerId = null
            this.emit('finished')
          }
        }
      } catch (err) {
        console.error('Status monitoring error:', err)
      }
    }, 750)
  }

  stopStatusMonitoring() {
    if (this.statusCheckInterval) {
      clearInterval(this.statusCheckInterval)
      this.statusCheckInterval = null
    }
  }

  destroy() {
    this.stopStatusMonitoring()
    if (this.crossfadeInterval) { clearInterval(this.crossfadeInterval); this.crossfadeInterval = null }
    if (this.currentPlayerId) this.stop().catch(() => {})
    this.listeners.clear()
  }
}
