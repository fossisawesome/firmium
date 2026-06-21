// File-save helpers for stats export (CSV/JSON) and recap image export.
// Uses the dialog plugin to pick a path, then writes via the Rust
// save_text_file / save_binary_file commands (commands/stats.rs).
import { save } from '@tauri-apps/plugin-dialog'
import { toBlob } from 'html-to-image'
import { tauriInvoke } from './tauri'

/** Opens a save dialog and writes UTF-8 text. Returns false if cancelled. */
export async function saveTextFile(defaultName: string, contents: string, ext: 'csv' | 'json'): Promise<boolean> {
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  })
  if (!path) return false
  await tauriInvoke('save_text_file', { path, contents })
  return true
}

/** Renders a DOM node to PNG and saves it via a save dialog. Returns false if cancelled. */
export async function saveNodeAsImage(node: HTMLElement, defaultName: string): Promise<boolean> {
  const bg = getComputedStyle(document.documentElement).getPropertyValue('--bg').trim() || '#0f0f0f'
  const blob = await toBlob(node, { pixelRatio: 2, cacheBust: true, backgroundColor: bg })
  if (!blob) return false
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: 'PNG', extensions: ['png'] }],
  })
  if (!path) return false
  const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()))
  await tauriInvoke('save_binary_file', { path, bytes })
  return true
}
