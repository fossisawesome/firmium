import { describe, it, expect, vi, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import {
  queue, queueIdx, audioBridge, trackDuration, currentPosition, isSeeking,
  lyricsOpen, lyricsSynced, lyricsLines,
} from './stores'
import type { Song, LyricLine } from './types/tauri-commands'

vi.mock('./tauri', () => ({
  tauriInvoke: vi.fn().mockResolvedValue(undefined),
  tauriFetch: vi.fn(),
}))

vi.mock('./api', () => ({
  Api: { scrobble: vi.fn(), reportPlayback: vi.fn(), getLyrics: vi.fn().mockResolvedValue(null) },
  OpenSubsonicRouter: { buildUrl: vi.fn().mockResolvedValue('http://example.com/stream') },
}))

vi.mock('./localApi', () => ({
  getLocalTrackPath: vi.fn().mockResolvedValue('/tmp/track'),
  findLocalMatch: vi.fn().mockResolvedValue(null),
}))

const { wireBridgeEvents, startPositionTracking, stopPositionTracking, activeLyricIdx } = await import('./playback')

const song = (id: string): Song => ({ id, title: id, artist: '', album: '', albumId: '', artistId: '', duration: 0 } as unknown as Song)

// Minimal AudioBridge stub: records position-event handlers.
function makeBridge() {
  const handlers: Record<string, ((data: unknown) => void)[]> = {}
  return {
    on: vi.fn((event: string, cb: (data: unknown) => void) => {
      ;(handlers[event] ??= []).push(cb)
    }),
    off: vi.fn((event: string, cb: (data: unknown) => void) => {
      handlers[event] = (handlers[event] ?? []).filter(h => h !== cb)
    }),
    emit: (event: string, data: unknown) => handlers[event]?.forEach(h => h(data)),
  }
}

beforeEach(() => {
  stopPositionTracking()
  queue.set([song('a'), song('b')])
  queueIdx.set(0)
  trackDuration.set(null)
  currentPosition.set(0)
  isSeeking.set(false)
  lyricsOpen.set(false)
  lyricsSynced.set(false)
  lyricsLines.set([])
  activeLyricIdx.set(-1)
})

describe('position tracking — store mirroring', () => {
  it('mirrors position and duration into the UI stores', () => {
    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)
    startPositionTracking()

    bridge.emit('position', { position: 42, duration: 100 })
    expect(get(currentPosition)).toBe(42)
    expect(get(trackDuration)).toBe(100)
  })

  it('does not overwrite position while seeking', () => {
    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)
    startPositionTracking()

    isSeeking.set(true)
    bridge.emit('position', { position: 42, duration: 100 })
    expect(get(currentPosition)).toBe(0)
  })

  it('advances the active synced-lyric index when lyrics are open', () => {
    const lines: LyricLine[] = [
      { start: 0, value: 'one' },
      { start: 5000, value: 'two' },
      { start: 10000, value: 'three' },
    ] as unknown as LyricLine[]
    lyricsLines.set(lines)
    lyricsSynced.set(true)
    lyricsOpen.set(true)

    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)
    startPositionTracking()

    bridge.emit('position', { position: 6, duration: 100 })
    expect(get(activeLyricIdx)).toBe(1)
  })
})

describe('startPositionTracking / stopPositionTracking', () => {
  it('detaches the handler so stale events no longer update stores', () => {
    const bridge = makeBridge()
    audioBridge.set(bridge as never)
    wireBridgeEvents(bridge as never)
    startPositionTracking()
    stopPositionTracking()

    bridge.emit('position', { position: 42, duration: 100 })
    expect(get(currentPosition)).toBe(0)
  })
})
