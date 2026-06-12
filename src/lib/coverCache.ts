// Thin wrapper around the Rust disk-based cover art cache (commands/cover_cache.rs).
// Dedupes concurrent requests for the same cover id on the frontend, since
// multiple <img> elements may request the same cover during a single render.
import { tauriInvoke } from './tauri'
import { convertFileSrc } from '@tauri-apps/api/core'

const _pending = new Map<string, Promise<string>>()

export function getCoverArt(coverId: string, url: string): Promise<string> {
  let promise = _pending.get(coverId)
  if (!promise) {
    promise = tauriInvoke<string>('get_cover_art', { coverId, url }).then(convertFileSrc)
    promise.finally(() => _pending.delete(coverId))
    _pending.set(coverId, promise)
  }
  return promise
}

export function clearAll(): Promise<void> {
  return tauriInvoke('clear_cover_cache')
}
