import { get } from 'svelte/store'
import { repeatOne, repeatAll } from './stores'
import { tauriInvoke } from './tauri'

export async function togglePlay(): Promise<void> {
  await tauriInvoke('toggle_play').catch(console.error)
}

export function prevTrack(): void {
  tauriInvoke('queue_prev').catch(console.error)
}

export function nextTrack(): void {
  tauriInvoke('queue_next').catch(console.error)
}

export function toggleShuffle(): void {
  tauriInvoke('toggle_shuffle').catch(console.error)
}

export function cycleRepeat(): void {
  const $repeatOne = get(repeatOne)
  const $repeatAll = get(repeatAll)
  // Cycle: off → repeat all (forever) → repeat one (once) → off
  if (!$repeatOne && !$repeatAll) {
    tauriInvoke('set_repeat_mode', { repeatOne: false, repeatAll: true }).catch(console.error)
  } else if ($repeatAll) {
    tauriInvoke('set_repeat_mode', { repeatOne: true, repeatAll: false }).catch(console.error)
  } else {
    tauriInvoke('set_repeat_mode', { repeatOne: false, repeatAll: false }).catch(console.error)
  }
}
