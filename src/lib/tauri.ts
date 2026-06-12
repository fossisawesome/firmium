import { invoke } from '@tauri-apps/api/core'
// Import fetch from the Tauri HTTP plugin directly so it works on both desktop
// and Android. Using window.__TAURI__?.http?.fetch was unreliable on Android —
// the global isn't always populated before the first fetch, causing a silent
// fallback to window.fetch which gets blocked by CORS on most Subsonic servers.
import { fetch as pluginFetch } from '@tauri-apps/plugin-http'

export { invoke as tauriInvoke }

// Routes HTTP requests through the Tauri http plugin (reqwest on the Rust side)
// rather than the WebView's native fetch. This bypasses CORS and the WebKit
// mixed-content blocker that silently aborts http:// requests from tauri://localhost.
// Never pass the signal to the Tauri HTTP plugin — it registers its own abort
// listener that throws "resource id X is invalid" on a promise we can't catch.
// Instead, race the response against the signal ourselves so callers get a clean
// AbortError and the plugin request just completes silently in the background.
export function tauriFetch(url: string | URL | Request, init?: RequestInit): Promise<Response> {
  const signal = init?.signal
  const pluginInit: RequestInit | undefined = signal ? { ...init, signal: undefined as unknown as AbortSignal } : init
  const p = pluginFetch(url, pluginInit)

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
