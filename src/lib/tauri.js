import { invoke } from '@tauri-apps/api/core'

export { invoke as tauriInvoke }

// Routes HTTP requests through the Tauri http plugin (reqwest on the Rust side)
// rather than the WebView's native fetch. This bypasses WebKit's mixed-content
// blocker, which silently aborts http:// requests from the secure tauri://localhost
// origin. Falls back to window.fetch if the plugin is not available (e.g. tests).
export const tauriFetch = (url, init) => {
  const pluginFetch = window.__TAURI__?.http?.fetch
  if (!pluginFetch) {
    console.warn('tauri-plugin-http not available — falling back to window.fetch. http:// targets may fail.')
    return fetch(url, init)
  }
  return pluginFetch(url, init)
}
