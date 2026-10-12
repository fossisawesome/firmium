import { onDestroy } from 'svelte'

export const formatDuration = (secs: number | string): string => {
  const s = Number(secs)
  if (isNaN(s) || s <= 0) return '0:00'
  const m = Math.floor(s / 60), r = Math.floor(s % 60)
  return `${m}:${r < 10 ? '0' : ''}${r}`
}

// Run async tasks with a concurrency limit.
export const pooledMap = async <T, R>(items: T[], limit: number, asyncFn: (item: T) => Promise<R>): Promise<R[]> => {
  const results = new Array<R>(items.length)
  let nextIdx = 0
  const worker = async () => {
    while (nextIdx < items.length) {
      const idx = nextIdx++
      results[idx] = await asyncFn(items[idx])
    }
  }
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker))
  return results
}

// Wraps localStorage with try/catch so silent data loss surfaces in the console.
export const SafeStorage = {
  getItem: (key: string): string | null => {
    try { return localStorage.getItem(key) } catch (e) {
      console.warn(`SafeStorage.getItem("${key}") failed:`, e)
      return null
    }
  },
  setItem: (key: string, value: string): void => {
    try { localStorage.setItem(key, value) } catch (e) {
      console.warn(`SafeStorage.setItem("${key}") failed — storage may be full or unavailable:`, e)
    }
  },
  removeItem: (key: string): void => {
    try { localStorage.removeItem(key) } catch (e) {
      console.warn(`SafeStorage.removeItem("${key}") failed:`, e)
    }
  }
}


// Manages a single AbortController for a component's in-flight requests:
// renew() aborts any previous request and returns a fresh signal; the
// controller is also aborted automatically when the component is destroyed.
export function createAbortController(): { renew: () => AbortSignal; readonly signal: AbortSignal | undefined } {
  let ctrl: AbortController | undefined
  onDestroy(() => ctrl?.abort())
  return {
    renew: (): AbortSignal => {
      ctrl?.abort()
      ctrl = new AbortController()
      return ctrl.signal
    },
    get signal() { return ctrl?.signal }
  }
}

export const safeText = (str: unknown): string =>
  String(str ?? '').replace(/[&<>"']/g, m => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;'
  } as Record<string, string>)[m])

export function extractGenres(raw: unknown): string[] {
  if (!Array.isArray(raw)) return []
  return raw
    .map((g: unknown) => (typeof g === 'object' && g !== null && 'name' in g) ? String((g as { name: unknown }).name) : null)
    .filter((n): n is string => n !== null && n.length > 0)
}

export function albumDecade(year: number | undefined): string | null {
  if (!year || year < 1900) return null
  const decade = Math.floor(year / 10) * 10
  return `${decade}s`
}
