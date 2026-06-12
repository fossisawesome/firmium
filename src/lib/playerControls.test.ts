import { describe, it, expect, vi, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import { audioBridge, currentPosition, queue, queueIdx } from './stores'
import type { Song } from './types/tauri-commands'

vi.mock('./tauri', () => ({
  tauriInvoke: vi.fn().mockResolvedValue(undefined),
  tauriFetch: vi.fn(),
}))

vi.mock('./api', () => ({
  Api: { scrobble: vi.fn(), getLyrics: vi.fn().mockResolvedValue(null) },
  OpenSubsonicRouter: { buildUrl: vi.fn().mockResolvedValue('http://example.com/stream') },
}))

vi.mock('./playback', () => ({ playAt: vi.fn() }))

const { prevTrack } = await import('./playerControls')
const { playAt } = await import('./playback')

const song = (id: string): Song => ({ id, title: id, artist: '', album: '', albumId: '', artistId: '', duration: 0 } as unknown as Song)

beforeEach(() => {
  vi.clearAllMocks()
  queue.set([song('a'), song('b'), song('c')])
  queueIdx.set(1)
  currentPosition.set(0)
  audioBridge.set(null)
})

describe('prevTrack', () => {
  it('restarts the current track when more than 3s in', () => {
    const seek = vi.fn().mockResolvedValue(undefined)
    audioBridge.set({ seek } as never)
    currentPosition.set(5)

    prevTrack()

    expect(seek).toHaveBeenCalledWith(0)
    expect(playAt).not.toHaveBeenCalled()
  })

  it('jumps to the previous track when 3s or less in', () => {
    const seek = vi.fn().mockResolvedValue(undefined)
    audioBridge.set({ seek } as never)
    currentPosition.set(3)

    prevTrack()

    expect(seek).not.toHaveBeenCalled()
    expect(playAt).toHaveBeenCalledWith(0)
  })

  it('does nothing at the start of the queue when near the beginning of the track', () => {
    const seek = vi.fn().mockResolvedValue(undefined)
    audioBridge.set({ seek } as never)
    queueIdx.set(0)
    currentPosition.set(1)

    prevTrack()

    expect(seek).not.toHaveBeenCalled()
    expect(playAt).not.toHaveBeenCalled()
  })

  it('restarts the current track even at the start of the queue when more than 3s in', () => {
    const seek = vi.fn().mockResolvedValue(undefined)
    audioBridge.set({ seek } as never)
    queueIdx.set(0)
    currentPosition.set(10)

    prevTrack()

    expect(seek).toHaveBeenCalledWith(0)
    expect(playAt).not.toHaveBeenCalled()
  })
})
