// UI state. Anything that is real application state (queue, settings, auth)
// lives in Rust; this only mirrors what the backend reports and holds
// purely visual state (current view, open panel, toasts).
import { Api } from './api'
import { on, errorMessage, isSessionExpired } from './ipc'
import type {
  AuthState, Settings, Theme, QueueState, PlaybackState, Song,
  PlaybackStateEvent, PlaybackPositionEvent,
} from './types'

export type ViewType =
  | 'home' | 'albums' | 'artists' | 'search' | 'playlists' | 'favorites' | 'genres' | 'mix'
  | 'podcasts' | 'settings'
  | 'album' | 'artist' | 'playlist' | 'podcast' | 'genre'

export interface ActiveView { type: ViewType; id?: string }
export type PanelType = 'queue' | 'lyrics' | 'similar' | 'stats'
export interface Toast { id: number; text: string }

// Backend payloads are replaced wholesale, so everything is $state.raw (no deep proxies).
class AppState {
  // ── Backend mirrors
  auth = $state.raw<AuthState>({ authed: false, server: null, username: null, accounts: [] })
  settings = $state.raw<Settings | null>(null)
  themes = $state.raw<Theme[]>([])
  serverExtensions = $state.raw<string[]>([])
  queueState = $state.raw<QueueState>({
    queue: [], queueIdx: -1, repeatOne: false, repeatAll: false, shuffleEnabled: false,
    crossfadeEnabled: false, crossfadeDuration: 5, crossfadeCurve: 'linear', gaplessEnabled: true,
    replayGainEnabled: true, volume: 0.8, playerId: null,
  })
  playbackState = $state.raw<PlaybackState>('stopped')
  position = $state(0)
  duration = $state(0)

  isAuthed = $derived(this.auth.authed)
  queue = $derived(this.queueState.queue)
  queueIdx = $derived(this.queueState.queueIdx)
  currentTrack = $derived<Song | null>(this.queueState.queue[this.queueState.queueIdx] ?? null)
  volume = $derived(this.queueState.volume)

  /** Bumped after imports/downloads/account switches so views refetch. */
  dataVersion = $state(0)

  // ── View-only state
  activeView = $state.raw<ActiveView>({ type: 'home' })
  /** Right-hand panels, one at a time. */
  openPanel = $state<PanelType | null>(null)
  showAccountModal = $state(false)
  toasts = $state.raw<Toast[]>([])
}

export const app = new AppState()

export const bumpData = () => { app.dataVersion++ }

// ── Navigation (view-only) ──────────────────────────────────────────────────
interface NavEntry { view: ActiveView; scroll: number }
const backStack: NavEntry[] = []
const forwardStack: NavEntry[] = []
let scrollHost: HTMLElement | null = null

/** The one scrollable pane every main view renders into, so its offset can be saved and restored. */
export function setScrollHost(el: HTMLElement | null): void { scrollHost = el }

const snapshot = (): NavEntry => ({ view: app.activeView, scroll: scrollHost?.scrollTop ?? 0 })

// Views fetch asynchronously, so the pane may be too short at first; retry until it can reach `top`.
function restoreScroll(top: number): void {
  const el = scrollHost
  if (!el) return
  let frames = 0
  const step = () => {
    el.scrollTop = top
    if (top > 0 && Math.abs(el.scrollTop - top) > 1 && ++frames < 60) requestAnimationFrame(step)
  }
  requestAnimationFrame(step)
}

function show(entry: NavEntry): void {
  app.activeView = entry.view
  restoreScroll(entry.scroll)
}

export function navTo(type: ViewType, id?: string): void {
  backStack.push(snapshot())
  if (backStack.length > 50) backStack.shift()
  forwardStack.length = 0
  app.activeView = { type, id }
  restoreScroll(0)
}
export function navBack(): void {
  const prev = backStack.pop()
  if (!prev) return
  forwardStack.push(snapshot())
  show(prev)
}
export function navForward(): void {
  const next = forwardStack.pop()
  if (!next) return
  backStack.push(snapshot())
  show(next)
}

// ── Right-hand panels (one at a time) ───────────────────────────────────────
export const togglePanel = (p: PanelType) => { app.openPanel = app.openPanel === p ? null : p }

// ── Toasts ──────────────────────────────────────────────────────────────────
let toastId = 0
export function toast(text: string, ms = 4000): void {
  const id = ++toastId
  app.toasts = [...app.toasts, { id, text }]
  setTimeout(() => { app.toasts = app.toasts.filter(x => x.id !== id) }, ms)
}
/** Shows a rejected command's message unless it is a session expiry (handled by the event). */
export function toastError(e: unknown): void {
  if (!isSessionExpired(e)) toast(errorMessage(e))
}

// ── Settings & theme ────────────────────────────────────────────────────────
export async function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]): Promise<void> {
  try {
    const s = await Api.setSetting(key, value)
    app.settings = s
    if (key === 'windowDecorations') Api.setWindowDecorations(value as boolean).catch(() => {})
  } catch (e) { toastError(e) }
}

export async function resetSettings(): Promise<void> {
  app.settings = await Api.resetSettings()
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
  $effect.root(() => {
    $effect(() => { if (app.settings && app.themes.length) applyAppearance(app.settings, app.themes) })
  })
}

// ── Startup ─────────────────────────────────────────────────────────────────
/** Hydrates from the backend and subscribes to its events. Returns an unsubscribe fn. */
export async function initBackend(): Promise<() => void> {
  watchAppearance()
  const unlisten = await Promise.all([
    on<PlaybackStateEvent>('playback-state-changed', e => {
      if (e.playerId === app.queueState.playerId) app.playbackState = e.state
    }),
    on<PlaybackPositionEvent>('playback-position', e => {
      if (e.playerId !== app.queueState.playerId) return
      app.position = e.position
      if (e.duration != null) app.duration = e.duration
    }),
    on<{ playerId: string }>('playback-finished', () => {}),
    on<QueueState>('queue-state-changed', q => {
      const prev = app.queueState
      app.queueState = q
      if (q.playerId !== prev.playerId || q.queueIdx !== prev.queueIdx) {
        app.position = 0
        app.duration = app.currentTrack?.duration ?? 0
      }
    }),
    on('session-expired', async () => {
      app.auth = await Api.logout()
      app.showAccountModal = true
      toast('Session expired. Please log in again.')
    }),
  ])

  const [s, t] = await Promise.all([Api.settings(), Api.themes()])
  app.themes = t
  app.settings = s
  app.queueState = await Api.queueState()
  app.auth = await Api.authState()
  return () => unlisten.forEach(u => u())
}

/** Refreshes auth and clears cached views after login / account switch / logout. */
export async function setAuthState(a: AuthState): Promise<void> {
  app.auth = a
  app.serverExtensions = a.authed ? await Api.serverExtensions() : []
  bumpData()
}
