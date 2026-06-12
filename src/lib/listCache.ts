// In-memory cache for list/page data (albums, artists, home sections), so
// navigating back to a view doesn't refetch — mirrors the Android ViewModel
// behavior of keeping state alive across navigation.
const _cache = new Map<string, unknown>()

export function getCached<T = unknown>(key: string): T | null {
  return _cache.has(key) ? (_cache.get(key) as T) : null
}

export function setCached(key: string, value: unknown): void {
  _cache.set(key, value)
}

export function clearAll(): void {
  _cache.clear()
}
