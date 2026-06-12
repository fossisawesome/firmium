// In-memory LRU cache for cover art blob URLs.
// Evicts oldest entries when total estimated byte size exceeds the budget.
const MAX_BYTES = 50 * 1024 * 1024 // 50 MB
let _totalBytes = 0

interface CoverEntry {
  url: string
  size: number
}

const _covers = new Map<string, CoverEntry>()
const _pending = new Map<string, Promise<unknown>>()

export function getCover(id: string): string | null {
  const entry = _covers.get(id) || null
  if (entry) { _covers.delete(id); _covers.set(id, entry) } // LRU touch
  return entry ? entry.url : null
}

// sizeBytes is the original blob.size — pass 0 if unknown (falls back to count guard).
export function addCover(id: string, url: string, sizeBytes = 0): void {
  if (!id || !url) return
  if (_covers.has(id)) {
    _totalBytes -= _covers.get(id)!.size
    _covers.delete(id)
  }
  _covers.set(id, { url, size: sizeBytes })
  _totalBytes += sizeBytes

  while (_totalBytes > MAX_BYTES && _covers.size > 0) {
    const oldest = _covers.keys().next().value as string
    const old = _covers.get(oldest)!
    if (old.url?.startsWith('blob:')) { try { URL.revokeObjectURL(old.url) } catch (_) {} }
    _totalBytes -= old.size
    _covers.delete(oldest)
  }
}

export const getPending = (id: string): Promise<unknown> | null => _pending.get(id) || null
export const setPending = (id: string, p: Promise<unknown>): void => { if (id && p) _pending.set(id, p) }
export const clearPending = (id: string): void => { _pending.delete(id) }

export function clearAll(): void {
  _covers.forEach(entry => {
    if (entry.url?.startsWith('blob:')) { try { URL.revokeObjectURL(entry.url) } catch (_) {} }
  })
  _covers.clear()
  _pending.clear()
  _totalBytes = 0
}
