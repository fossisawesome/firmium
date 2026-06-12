import { tauriInvoke } from './tauri'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

type BridgeEvent = 'statechange' | 'finished' | 'volumechange' | 'error' | 'position'
type BridgeCallback = (data?: any) => void

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
  currentPlayerId: string | null = null
  listeners = new Map<BridgeEvent, BridgeCallback[]>()
  lastKnownState: string | null = null
  _hasStartedPlaying = false
  preloadedPlayerId: string | null = null
  preloadedTrackId: string | null = null
  _unlistenState: UnlistenFn | null = null
  _unlistenFinished: UnlistenFn | null = null
  _unlistenPosition: UnlistenFn | null = null

  constructor() {
    this._initListeners()
  }

  // ── Event listeners ────────────────────────────────────────────────────────

  async _initListeners(): Promise<void> {
    // Rust emits global events via app_handle.emit() — listen() wraps data in { payload }.
    this._unlistenState = await listen<{ playerId: string; state: string }>('playback-state-changed', ({ payload }) => {
      if (payload.playerId !== this.currentPlayerId) return
      const state = payload.state
      if (state === 'playing') this._hasStartedPlaying = true
      if (state !== this.lastKnownState) {
        this.lastKnownState = state
        this.emit('statechange', state)
      }
    })

    this._unlistenFinished = await listen<{ playerId: string }>('playback-finished', ({ payload }) => {
      if (payload.playerId !== this.currentPlayerId) return
      this.lastKnownState = 'finished'
      this.currentPlayerId = null
      this.emit('finished')
    })

    // Position events from Rust's finish-watcher thread (~300ms cadence).
    this._unlistenPosition = await listen<{ playerId: string; position: number; duration: number }>('playback-position', ({ payload }) => {
      if (payload.playerId !== this.currentPlayerId) return
      this.emit('position', { position: payload.position, duration: payload.duration })
    })
  }

  // ── Event emitter ──────────────────────────────────────────────────────────

  on(event: BridgeEvent, callback: BridgeCallback): void {
    if (!this.listeners.has(event)) this.listeners.set(event, [])
    this.listeners.get(event)!.push(callback)
  }

  off(event: BridgeEvent, callback: BridgeCallback): void {
    if (this.listeners.has(event)) {
      const callbacks = this.listeners.get(event)!
      const idx = callbacks.indexOf(callback)
      if (idx > -1) callbacks.splice(idx, 1)
    }
  }

  emit(event: BridgeEvent, data?: any): void {
    if (this.listeners.has(event)) {
      this.listeners.get(event)!.forEach(cb => cb(data))
    }
  }

  // ── Playback controls ──────────────────────────────────────────────────────

  async play(streamUrl: string, trackId: string, replayGainDb: number | null = null): Promise<string> {
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
      const playerId = await tauriInvoke<string>('play_stream', { streamUrl, trackId, replayGainDb })
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

  async preload(streamUrl: string, trackId: string, replayGainDb: number | null = null): Promise<void> {
    if (this.preloadedPlayerId && this.preloadedTrackId !== trackId) {
      const old = this.preloadedPlayerId
      this.preloadedPlayerId = null
      this.preloadedTrackId = null
      try { await tauriInvoke('stop_playback', { playerId: old }) } catch (_) {}
    }
    if (this.preloadedTrackId === trackId) return
    try {
      const playerId = await tauriInvoke<string>('preload_stream', { streamUrl, trackId, replayGainDb })
      this.preloadedPlayerId = playerId
      this.preloadedTrackId = trackId
    } catch (err) {
      console.error('Preload failed:', err)
    }
  }

  async pause(): Promise<void> {
    if (!this.currentPlayerId) return
    try {
      await tauriInvoke('pause_playback', { playerId: this.currentPlayerId })
      this.lastKnownState = 'paused'
      this.emit('statechange', 'paused')
    } catch (err) {
      this.emit('error', `Pause failed: ${err}`)
    }
  }

  async resume(): Promise<void> {
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

  async stop(): Promise<void> {
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

  async startCrossfadeIn(streamUrl: string, trackId: string, targetVolume: number, fadeDurationMs: number, replayGainDb: number | null = null): Promise<void> {
    const oldPlayerId = this.currentPlayerId
    try {
      const newPlayerId = await tauriInvoke<string>('crossfade_to', {
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

  async setVolume(vol: number): Promise<void> {
    const normalized = Math.max(0, Math.min(1, Number(vol)))
    if (!this.currentPlayerId) return
    try {
      await tauriInvoke('set_volume', { playerId: this.currentPlayerId, volume: normalized })
      this.emit('volumechange', normalized)
    } catch (err) {
      this.emit('error', `Volume change failed: ${err}`)
    }
  }

  async getVolume(): Promise<number | null> {
    if (!this.currentPlayerId) return null
    try { return await tauriInvoke<number>('get_volume', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get volume failed:', err)
      return null
    }
  }

  async seek(position: number): Promise<void> {
    if (!this.currentPlayerId) return
    try { await tauriInvoke('seek_position', { playerId: this.currentPlayerId, position: Math.max(0, Number(position) || 0) }) } catch (err) {
      console.warn('Seek not supported for this stream:', err)
    }
  }

  async getCurrentPosition(): Promise<number> {
    if (!this.currentPlayerId) return 0
    try { return await tauriInvoke<number>('get_current_position', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get position failed:', err)
      return 0
    }
  }

  async getDuration(): Promise<number | null> {
    if (!this.currentPlayerId) return null
    try { return await tauriInvoke<number>('get_track_duration', { playerId: this.currentPlayerId }) } catch (err) {
      console.error('Get duration failed:', err)
      return null
    }
  }

  async isFinished(): Promise<boolean> {
    if (!this.currentPlayerId) return true
    try { return await tauriInvoke<boolean>('is_playback_finished', { playerId: this.currentPlayerId }) } catch (err) {
      return false
    }
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  destroy(): void {
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
