import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('./tauri', () => ({
  tauriInvoke: vi.fn().mockResolvedValue(undefined),
  tauriFetch: vi.fn(),
}))

const { prevTrack, nextTrack, togglePlay } = await import('./playerControls')
const { tauriInvoke } = await import('./tauri')

beforeEach(() => {
  vi.clearAllMocks()
})

describe('prevTrack', () => {
  it('calls queue_prev Rust command', () => {
    prevTrack()
    expect(tauriInvoke).toHaveBeenCalledWith('queue_prev')
  })
})

describe('nextTrack', () => {
  it('calls queue_next Rust command', () => {
    nextTrack()
    expect(tauriInvoke).toHaveBeenCalledWith('queue_next')
  })
})

describe('togglePlay', () => {
  it('calls toggle_play Rust command', async () => {
    await togglePlay()
    expect(tauriInvoke).toHaveBeenCalledWith('toggle_play')
  })
})
