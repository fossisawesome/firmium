// UI state. Anything that is real application state (queue, settings, auth)
// lives in Rust; these stores only mirror what the backend reports and hold
// purely visual state (current view, open panel, toasts).
import { writable, derived, get } from 'svelte/store'
import { Api } from './api'
import { on, errorMessage, isSessionExpired } from './ipc'
import type {
  AuthState, Settings, Theme, QueueState, PlaybackState, Song,
  PlaybackStateEvent, PlaybackPositionEvent,
} from './types'

// ── Backend mirrors ─────────────────────────────────────────────────────────
export const auth = writable<AuthState>({ authed: false, server: null, username: null, accounts: [] })
export const isAuthed = derived(auth, $a => $a.authed)
export const settings = writable<Settings | null>(null)
export const themes = writable<Theme[]>([])
export const serverExtensions = writable<string[]>([])

export const queueState = writable<QueueState>({
  queue: [], queueIdx: -1, repeatOne: false, repeatAll: false, shuffleEnabled: false,
  crossfadeEnabled: false, crossfadeDuration: 5, crossfadeCurve: 'linear', gaplessEnabled: true,
  replayGainEnabled: true, volume: 0.8, playerId: null,
})
export const queue = derived(queueState, $q => $q.queue)
export const queueIdx = derived(queueState, $q => $q.queueIdx)
export const currentTrack = derived(queueState, $q => $q.queue[$q.queueIdx] ?? null as Song | null)
export const volume = derived(queueState, $q => $q.volume)

export const playbackState = writable<PlaybackState>('stopped')
export const position = writable(0)
export const duration = writable(0)

// Bumped after imports/downloads/account switches so views refetch.
export const dataVersion = writable(0)
export const bumpData = () => dataVersion.update(n => n + 1)

// ── Navigation (view-only) ──────────────────────────────────────────────────
export type ViewType =
  | 'home' | 'albums' | 'artists' | 'search' | 'playlists' | 'favorites' | 'genres' | 'mix'
  | 'podcasts' | 'settings'
  | 'album' | 'artist' | 'playlist' | 'podcast' | 'genre'

export interface ActiveView { type: ViewType; id?: string }

export const activeView = writable<ActiveView>({ type: 'home' })
const history: ActiveView[] = []

export function navTo(type: ViewType, id?: string): void {
  history.push(get(activeView))
  if (history.length > 50) history.shift()
  activeView.set({ type, id })
}
export function navBack(): void {
  activeView.set(history.pop() ?? { type: 'home' })
}

// ── Right-hand panels (one at a time) ───────────────────────────────────────
export type PanelType = 'queue' | 'lyrics' | 'similar' | 'eq' | 'stats'
export const openPanel = writable<PanelType | null>(null)
export const togglePanel = (p: PanelType) => openPanel.update(cur => (cur === p ? null : p))

export const showAccountModal = writable(false)

// ── Toasts ──────────────────────────────────────────────────────────────────
export interface Toast { id: number; text: string }
export const toasts = writable<Toast[]>([])
let toastId = 0
export function toast(text: string, ms = 4000): void {
  const id = ++toastId
  toasts.update(t => [...t, { id, text }])
  setTimeout(() => toasts.update(t => t.filter(x => x.id !== id)), ms)
}
/** Shows a rejected command's message unless it is a session expiry (handled by the event). */
export function toastError(e: unknown): void {
  if (!isSessionExpired(e)) toast(errorMessage(e))
}

// ── Settings & theme ────────────────────────────────────────────────────────
export async function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]): Promise<void> {
  try {
    const s = await Api.setSetting(key, value)
    settings.set(s)
    if (key === 'windowDecorations') Api.setWindowDecorations(value as boolean).catch(() => {})
  } catch (e) { toastError(e) }
}

export async function resetSettings(): Promise<void> {
  settings.set(await Api.resetSettings())
}

/** Writes the active color theme, UI theme, font and scrollbar width onto :root. */
export function applyAppearance(s: Settings, all: Theme[]): void {
  const root = document.documentElement
  const theme = all.find(t => t.id === s.themeId)
  if (theme) {
    const c = theme.colors
    root.style.colorScheme = theme.color_scheme || 'dark'
    const vars: Record<string, string> = {
      '--bg': c.bg, '--surface': c.surface, '--surface2': c.surface2, '--border': c.border,
      '--text': c.text, '--muted': c.muted, '--accent': c.accent, '--accent-dim': c.accent_dim,
      '--error': c.error, '--timing': c.timing || '0.15s',
    }
    for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v)
  }
  root.style.setProperty('--font', fontStack(s.fontFamily, theme?.colors.font))
  root.style.setProperty('--scrollbar-width', `${s.scrollbarWidth}px`)
  root.dataset.uiTheme = s.uiThemeId
}

export const FONT_OPTIONS = [
  'Inter', 'Liberation Mono', 'Monospace', 'System', 'Comic Sans', 'Sans Serif',
  'BigBlue Terminal', 'Cousine', 'FiraCode', 'Hack',
]

function fontStack(name: string, themeFont?: string): string {
  const stacks: Record<string, string> = {
    'Inter': "'Inter', sans-serif",
    'Liberation Mono': "'Liberation Mono', monospace",
    'Monospace': 'monospace',
    'System': 'system-ui, sans-serif',
    'Comic Sans': "'Comic Sans MS', 'Comic Neue', cursive",
    'Sans Serif': 'sans-serif',
    'BigBlue Terminal': "'BigBlue TerminalPlus', monospace",
    'Cousine': "'Cousine', monospace",
    'FiraCode': "'Fira Code', monospace",
    'Hack': "'Hack', monospace",
  }
  return stacks[name] ?? themeFont ?? "'Courier New', monospace"
}

// Re-apply whenever settings or themes change.
let appearanceReady = false
function watchAppearance() {
  if (appearanceReady) return
  appearanceReady = true
  derived([settings, themes], x => x).subscribe(([s, t]) => { if (s && t.length) applyAppearance(s, t) })
}

// ── Startup ─────────────────────────────────────────────────────────────────
/** Hydrates from the backend and subscribes to its events. Returns an unsubscribe fn. */
export async function initBackend(): Promise<() => void> {
  watchAppearance()
  const unlisten = await Promise.all([
    on<PlaybackStateEvent>('playback-state-changed', e => {
      if (e.playerId === get(queueState).playerId) playbackState.set(e.state)
    }),
    on<PlaybackPositionEvent>('playback-position', e => {
      if (e.playerId !== get(queueState).playerId) return
      position.set(e.position)
      if (e.duration != null) duration.set(e.duration)
    }),
    on<{ playerId: string }>('playback-finished', () => {}),
    on<QueueState>('queue-state-changed', q => {
      const prev = get(queueState)
      queueState.set(q)
      if (q.playerId !== prev.playerId || q.queueIdx !== prev.queueIdx) {
        position.set(0)
        duration.set(get(currentTrack)?.duration ?? 0)
      }
    }),
    on('session-expired', async () => {
      auth.set(await Api.logout())
      showAccountModal.set(true)
      toast('Session expired. Please log in again.')
    }),
  ])

  const [s, t] = await Promise.all([Api.settings(), Api.themes()])
  themes.set(t)
  settings.set(s)
  queueState.set(await Api.queueState())
  auth.set(await Api.authState())
  return () => unlisten.forEach(u => u())
}

/** Refreshes auth and clears cached views after login / account switch / logout. */
export async function setAuthState(a: AuthState): Promise<void> {
  auth.set(a)
  if (a.authed) serverExtensions.set(await Api.serverExtensions())
  else serverExtensions.set([])
  bumpData()
}
