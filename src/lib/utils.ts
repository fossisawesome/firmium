import { onDestroy } from 'svelte'
import { tauriInvoke } from './tauri'

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

// Buffer log lines and flush them as a single `write_log` IPC call, so a burst
// of console output (e.g. framework warnings) doesn't trigger one IPC
// round-trip per line.
const LOG_FLUSH_INTERVAL_MS = 1000
const LOG_FLUSH_SIZE = 20
let _logBuffer: string[] = []
let _logFlushTimer: ReturnType<typeof setTimeout> | null = null

const _flushLogBuffer = (): void => {
  if (_logFlushTimer !== null) { clearTimeout(_logFlushTimer); _logFlushTimer = null }
  if (_logBuffer.length === 0) return
  const entry = _logBuffer.join('\n')
  _logBuffer = []
  try { tauriInvoke('write_log', { entry }) } catch (_) {}
}

// Strip the OpenSubsonic auth token (t=) and salt (s=) from any URLs before
// they're written to app-logs.txt, so the log file can be shared for support
// without leaking credentials.
const _redactAuthParams = (s: string): string =>
  s.replace(/([?&](?:t|s)=)[^&\s"]+/g, '$1[redacted]')

const _writeLog = (level: string, ...args: unknown[]): void => {
  const msg = args.map(a => {
    if (a instanceof Error) return `${a.name}: ${a.message}`
    if (typeof a === 'object' && a !== null) {
      try { return JSON.stringify(a) } catch (_) { return String(a) }
    }
    return String(a)
  }).join(' ')
  const ts = new Date().toISOString()
  _logBuffer.push(`[${ts}] [${level}] ${_redactAuthParams(msg)}`)
  if (_logBuffer.length >= LOG_FLUSH_SIZE) {
    _flushLogBuffer()
  } else if (_logFlushTimer === null) {
    _logFlushTimer = setTimeout(_flushLogBuffer, LOG_FLUSH_INTERVAL_MS)
  }
}

export const AppLogger = {
  info: (...args: unknown[]) => _writeLog('INFO', ...args),
  warn: (...args: unknown[]) => _writeLog('WARN', ...args),
  error: (...args: unknown[]) => _writeLog('ERROR', ...args),
}

// Patch console so existing log calls are also persisted to disk.
const _log = console.log.bind(console)
const _warn = console.warn.bind(console)
const _error = console.error.bind(console)
console.log = (...a: unknown[]) => { _log(...a); AppLogger.info(...a) }
console.warn = (...a: unknown[]) => { _warn(...a); AppLogger.warn(...a) }
console.error = (...a: unknown[]) => { _error(...a); AppLogger.error(...a) }

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
