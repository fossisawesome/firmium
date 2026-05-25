// In-memory LRU cache for cover art blob URLs.
// Oldest entries are evicted when the limit is exceeded and their blob URLs are revoked.
const MAX_COVER_CACHE_SIZE = 150
const _covers = new Map()
const _pending = new Map()

export function getCover(id) {
  const url = _covers.get(id) || null
  if (url) { _covers.delete(id); _covers.set(id, url) } // LRU touch
  return url
}

export function addCover(id, url) {
  if (!id || !url) return
  if (_covers.has(id)) _covers.delete(id)
  _covers.set(id, url)
  while (_covers.size > MAX_COVER_CACHE_SIZE) {
    const oldest = _covers.keys().next().value
    const oldUrl = _covers.get(oldest)
    if (oldUrl?.startsWith('blob:')) { try { URL.revokeObjectURL(oldUrl) } catch (_) {} }
    _covers.delete(oldest)
  }
}

export const getPending = (id) => _pending.get(id) || null
export const setPending = (id, p) => { if (id && p) _pending.set(id, p) }
export const clearPending = (id) => { _pending.delete(id) }

export function clearAll() {
  _covers.forEach(url => {
    if (url?.startsWith('blob:')) { try { URL.revokeObjectURL(url) } catch (_) {} }
  })
  _covers.clear()
  _pending.clear()
}
