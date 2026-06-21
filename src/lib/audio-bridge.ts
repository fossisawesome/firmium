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
  _unlistenState: UnlistenFn | null = null
  _unlistenFinished: UnlistenFn | null = null
  _unlistenPosition: UnlistenFn | null = null

  constructor() {
    this._initListeners()
  }

  // ── Event listeners ────────────────────────────────────────────────────────

  async _initListeners(): Promise<void> {
    // Rust emits global events via app_handle.emit() — listen() wraps data in { payload }.
    this._unlistenState = await listen<{ playerId: string; state: string; audioInfo?: { sampleRate: number; channels: number } }>('playback-state-changed', ({ payload }) => {
      if (payload.playerId !== this.currentPlayerId) return
      const state = payload.state
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
      this.emit('statechange', 'playing')
    } catch (err) {
      this.emit('error', `Resume failed: ${err}`)
    }
  }

  async stop(): Promise<void> {
    if (!this.currentPlayerId) return
    const idToStop = this.currentPlayerId
    this.currentPlayerId = null
    this.lastKnownState = 'stopped'
    try {
      await tauriInvoke('stop_playback', { playerId: idToStop })
      this.emit('statechange', 'stopped')
    } catch (err) {
      this.emit('error', `Stop failed: ${err}`)
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
    if (this.currentPlayerId) this.stop().catch(() => {})
    this.listeners.clear()
  }
}
