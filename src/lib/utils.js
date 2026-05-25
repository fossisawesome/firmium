import { tauriInvoke } from './tauri.js'

export const formatDuration = (secs) => {
  const s = Number(secs)
  if (isNaN(s) || s <= 0) return '0:00'
  const m = Math.floor(s / 60), r = Math.floor(s % 60)
  return `${m}:${r < 10 ? '0' : ''}${r}`
}

// Run async tasks with a concurrency limit.
export const pooledMap = async (items, limit, asyncFn) => {
  const results = new Array(items.length)
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
  getItem: (key) => {
    try { return localStorage.getItem(key) } catch (e) {
      console.warn(`SafeStorage.getItem("${key}") failed:`, e)
      return null
    }
  },
  setItem: (key, value) => {
    try { localStorage.setItem(key, value) } catch (e) {
      console.warn(`SafeStorage.setItem("${key}") failed — storage may be full or unavailable:`, e)
    }
  },
  removeItem: (key) => {
    try { localStorage.removeItem(key) } catch (e) {
      console.warn(`SafeStorage.removeItem("${key}") failed:`, e)
    }
  }
}

const _writeLog = (level, ...args) => {
  const msg = args.map(a => {
    if (a instanceof Error) return `${a.name}: ${a.message}`
    if (typeof a === 'object' && a !== null) {
      try { return JSON.stringify(a) } catch (_) { return String(a) }
    }
    return String(a)
  }).join(' ')
  const ts = new Date().toISOString()
  try { tauriInvoke('write_log', { entry: `[${ts}] [${level}] ${msg}` }) } catch (_) {}
}

export const AppLogger = {
  info: (...args) => _writeLog('INFO', ...args),
  warn: (...args) => _writeLog('WARN', ...args),
  error: (...args) => _writeLog('ERROR', ...args),
}

// Patch console so existing log calls are also persisted to disk.
const _log = console.log.bind(console)
const _warn = console.warn.bind(console)
const _error = console.error.bind(console)
console.log = (...a) => { _log(...a); AppLogger.info(...a) }
console.warn = (...a) => { _warn(...a); AppLogger.warn(...a) }
console.error = (...a) => { _error(...a); AppLogger.error(...a) }

export const safeText = (str) =>
  String(str ?? '').replace(/[&<>"']/g, m => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;'
  }[m]))
