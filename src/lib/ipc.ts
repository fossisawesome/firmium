// Thin bridge to the Rust backend. The UI never does I/O itself: every
// request is a Tauri command and every update arrives as a Tauri event.
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** Error shape the backend rejects with. */
export interface BackendError {
  kind: string
  message: string
}

export function call<T = void>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args)
}

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, e => handler(e.payload))
}

/** Human-readable message from a rejected command (BackendError, string or Error). */
export function errorMessage(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message)
  return String(e)
}

/** True when the backend rejected because the session is no longer valid. */
export const isSessionExpired = (e: unknown): boolean =>
  !!e && typeof e === 'object' && (e as BackendError).kind === 'SessionExpired'

/** Webview-safe URL for a file path returned by the backend (e.g. cover art). */
export const fileUrl = (path: string): string => convertFileSrc(path)
