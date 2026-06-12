import { get } from 'svelte/store'
import { audioBridge, currentTrack, currentPosition, queue, queueIdx, repeatOne, repeatAll, shuffleEnabled } from './stores'
import { playAt } from './playback'

export async function togglePlay(): Promise<void> {
  const bridge = get(audioBridge)
  if (!get(currentTrack) || !bridge) return
  const state = bridge.lastKnownState
  if (state === 'paused') await bridge.resume()
  else if (state === 'playing') await bridge.pause()
  else if (!state || state === 'stopped') playAt(get(queueIdx))
}

export function prevTrack(): void {
  const idx = get(queueIdx)
  if (get(currentPosition) > 3) {
    get(audioBridge)?.seek(0)
  } else if (idx > 0) {
    playAt(idx - 1)
  }
}

export function nextTrack(): void {
  const idx = get(queueIdx)
  const len = get(queue).length
  if (get(shuffleEnabled) && len > 1) {
    let next
    do { next = Math.floor(Math.random() * len) } while (next === idx)
    playAt(next)
  } else if (idx < len - 1) {
    playAt(idx + 1)
  } else if (get(repeatAll)) {
    playAt(0)
  }
}

export function toggleShuffle(): void {
  shuffleEnabled.update(v => !v)
}

export function cycleRepeat(): void {
  if (!get(repeatOne) && !get(repeatAll)) {
    repeatOne.set(true)
  } else if (get(repeatOne)) {
    repeatOne.set(false)
    repeatAll.set(true)
  } else {
    repeatAll.set(false)
  }
}
