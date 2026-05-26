import { invoke } from '@tauri-apps/api/core'

export { invoke as tauriInvoke }

// Routes HTTP requests through the Tauri http plugin (reqwest on the Rust side)
// rather than the WebView's native fetch. This bypasses WebKit's mixed-content
// blocker, which silently aborts http:// requests from the secure tauri://localhost
// origin. Falls back to window.fetch if the plugin is not available (e.g. tests).
// Never pass the signal to the Tauri HTTP plugin — it registers its own abort
// listener that throws "resource id X is invalid" on a promise we can't catch.
// Instead, race the response against the signal ourselves so callers get a clean
// AbortError and the plugin request just completes silently in the background.
export function tauriFetch(url, init) {
  // Resolved at call time (not module load) so Tauri is guaranteed to be initialized.
  const pluginFetch = window.__TAURI__?.http?.fetch
  if (!pluginFetch) {
    console.warn('tauri-plugin-http not available — falling back to window.fetch. http:// targets may fail.')
  }
  const signal = init?.signal
  const pluginInit = signal ? { ...init, signal: undefined } : init
  const p = pluginFetch ? pluginFetch(url, pluginInit) : fetch(url, init)

  if (!signal) return p

  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(new DOMException('Aborted', 'AbortError')); return }
    const onAbort = () => reject(new DOMException('Aborted', 'AbortError'))
    signal.addEventListener('abort', onAbort, { once: true })
    p.then(
      val => { signal.removeEventListener('abort', onAbort); resolve(val) },
      err => { signal.removeEventListener('abort', onAbort); reject(err) }
    )
  })
}
