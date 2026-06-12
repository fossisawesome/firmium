import { describe, it, expect, vi, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import {
  queue, queueIdx, audioBridge, crossfadeEnabled, crossfadeDuration,
  gaplessEnabled, repeatOne, repeatAll, trackDuration, currentPosition,
} from './stores'
import type { Song } from './types/tauri-commands'

vi.mock('./tauri', () => ({
  tauriInvoke: vi.fn().mockResolvedValue(undefined),
  tauriFetch: vi.fn(),
}))

vi.mock('./api', () => ({
  Api: { scrobble: vi.fn(), getLyrics: vi.fn().mockResolvedValue(null) },
  OpenSubsonicRouter: { buildUrl: vi.fn().mockResolvedValue('http://example.com/stream') },
}))

const { wireBridgeEvents, startPositionTracking, stopPositionTracking } = await import('./playback')

const song = (id: string): Song => ({ id, title: id, artist: '', album: '', albumId: '', artistId: '', duration: 0 } as unknown as Song)

// Minimal AudioBridge stub: records position-event handlers and crossfade/preload calls.
function makeBridge() {
  const handlers: Record<string, ((data: unknown) => void)[]> = {}
  return {
    preloadedTrackId: null as string | null,
    on: vi.fn((event: string, cb: (data: unknown) => void) => {
      ;(handlers[event] ??= []).push(cb)
    }),
    off: vi.fn((event: string, cb: (data: unknown) => void) => {
      handlers[event] = (handlers[event] ?? []).filter(h => h !== cb)
    }),
    emit: (event: string, data: unknown) => handlers[event]?.forEach(h => h(data)),
    startCrossfadeIn: vi.fn().mockResolvedValue(undefined),
    preload: vi.fn().mockResolvedValue(undefined),
    setVolume: vi.fn().mockResolvedValue(undefined),
    play: vi.fn().mockResolvedValue(undefined),
  }
}

beforeEach(() => {
  stopPositionTracking()
  queue.set([song('a'), song('b')])
  queueIdx.set(0)
  crossfadeEnabled.set(false)
  gaplessEnabled.set(false)
  repeatOne.set(false)
  repeatAll.set(false)
  crossfadeDuration.set(5)
  trackDuration.set(null)
  currentPosition.set(0)
})

describe('position tracking — crossfade trigger', () => {
  it('starts crossfade when within crossfadeDuration of the track end', async () => {
    crossfadeEnabled.set(true)
    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)
    startPositionTracking()

    // 100s track, 5s crossfade window — not yet at the trigger point.
    bridge.emit('position', { position: 90, duration: 100 })
    expect(bridge.startCrossfadeIn).not.toHaveBeenCalled()

    // Now within the last 5 seconds — should trigger exactly once.
    bridge.emit('position', { position: 96, duration: 100 })
    await Promise.resolve()
    expect(bridge.startCrossfadeIn).toHaveBeenCalledTimes(1)

    // Further updates must not re-trigger.
    bridge.emit('position', { position: 98, duration: 100 })
    await Promise.resolve()
    expect(bridge.startCrossfadeIn).toHaveBeenCalledTimes(1)
  })
})

describe('position tracking — gapless preload trigger', () => {
  it('preloads the next track 30s before the end when gapless is enabled', async () => {
    gaplessEnabled.set(true)
    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)
    startPositionTracking()

    // 100s track — preload window starts at 70s.
    bridge.emit('position', { position: 60, duration: 100 })
    await Promise.resolve()
    expect(bridge.preload).not.toHaveBeenCalled()

    bridge.emit('position', { position: 71, duration: 100 })
    await Promise.resolve()
    expect(bridge.preload).toHaveBeenCalledTimes(1)

    bridge.emit('position', { position: 80, duration: 100 })
    await Promise.resolve()
    expect(bridge.preload).toHaveBeenCalledTimes(1)
  })

  it('does not preload when gapless is disabled', async () => {
    gaplessEnabled.set(false)
    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)
    startPositionTracking()

    bridge.emit('position', { position: 71, duration: 100 })
    await Promise.resolve()
    expect(bridge.preload).not.toHaveBeenCalled()
  })
})

describe('startPositionTracking / stopPositionTracking', () => {
  it('resets per-track flags so a new track can re-trigger crossfade and preload', async () => {
    crossfadeEnabled.set(true)
    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)

    startPositionTracking()
    bridge.emit('position', { position: 96, duration: 100 })
    await Promise.resolve()
    expect(bridge.startCrossfadeIn).toHaveBeenCalledTimes(1)

    // Simulate moving to a new track: queue index advances, tracking restarts, flags reset.
    queueIdx.set(0)
    stopPositionTracking()
    startPositionTracking()
    bridge.emit('position', { position: 96, duration: 100 })
    await Promise.resolve()
    expect(bridge.startCrossfadeIn).toHaveBeenCalledTimes(2)
  })
})
