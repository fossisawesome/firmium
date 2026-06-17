<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import {
    isAuthed, setAuth, clearAuth, navToView,
    authServer, activeView, lyricsOpen, currentTrack,
    openSubsonicExtensions, showAccountModal, bumpDataSourceVersion,
    listenToQueueState, recentlyPlayedSongs,
    crossfadeEnabled, crossfadeDuration, gaplessEnabled, volume,
  } from './lib/stores'
  import { SafeStorage } from './lib/utils'
  import { Keyring, Api } from './lib/api'
  import type { RemotePlayQueue } from './lib/types/tauri-commands'
  import { importLocalFiles } from './lib/localApi'
  import { tauriInvoke } from './lib/tauri'
  import { listen } from '@tauri-apps/api/event'
  import { fetchAndShowLyrics } from './lib/playback'
  import { playlistMenuState, hidePlaylistMenu } from './lib/playlistMenu'
  import type { Theme } from './lib/types/tauri-commands'

  import AccountModal from './components/AccountModal.svelte'
  import ResumeQueuePrompt from './components/ResumeQueuePrompt.svelte'
  import Sidebar from './components/Sidebar.svelte'
  import PlayerBar from './components/PlayerBar.svelte'
  import LyricsPanel from './components/LyricsPanel.svelte'
  import SimilarTracksPanel from './components/SimilarTracksPanel.svelte'
  import VisualizerPanel from './components/VisualizerPanel.svelte'
  import PlaylistMenu from './components/PlaylistMenu.svelte'
  import AlbumList from './views/AlbumList.svelte'
  import AlbumDetail from './views/AlbumDetail.svelte'
  import ArtistList from './views/ArtistList.svelte'
  import ArtistDetail from './views/ArtistDetail.svelte'
  import SearchView from './views/SearchView.svelte'
  import PlaylistsView from './views/PlaylistsView.svelte'
  import PlaylistDetail from './views/PlaylistDetail.svelte'
  import Settings from './views/Settings.svelte'
  import HomeView from './views/HomeView.svelte'

  let setupError = $state('')
  let loadedThemes = $state<Theme[]>([])
  let dragActive = $state(false)
  let remoteQueue = $state<RemotePlayQueue | null>(null)

  async function checkRemotePlayQueue() {
    try {
      const result = await Api.getPlayQueue()
      if (result && result.entries.length > 0) remoteQueue = result
    } catch (_) {}
  }

  // Apply a theme by setting CSS custom properties directly on :root.
  // Falls back gracefully if the theme ID isn't in the loaded list.
  function applyThemeById(id: string) {
    const theme = loadedThemes.find(t => t.id === id)
    if (theme) applyThemeData(theme)
  }

  function applyThemeData(theme: Theme) {
    const root = document.documentElement
    root.style.colorScheme = theme.color_scheme || 'dark'
    const c = theme.colors
    root.style.setProperty('--bg', c.bg)
    root.style.setProperty('--surface', c.surface)
    root.style.setProperty('--surface2', c.surface2)
    root.style.setProperty('--border', c.border)
    root.style.setProperty('--text', c.text)
    root.style.setProperty('--muted', c.muted)
    root.style.setProperty('--accent', c.accent)
    root.style.setProperty('--accent-dim', c.accent_dim)
    root.style.setProperty('--error', c.error)
    root.style.setProperty('--font', c.font || "'Courier New', monospace")
    root.style.setProperty('--timing', c.timing || '0.15s')
  }

  async function applyDecorations() {
    const show = SafeStorage.getItem('firmium_decorations') !== 'false'
    try {
      const tauriGlobal = (window as any).__TAURI__
      if (tauriGlobal) {
        const tauriWindow = tauriGlobal.window || tauriGlobal
        if (tauriWindow && typeof tauriWindow.getCurrentWindow === 'function') {
          tauriWindow.getCurrentWindow().setDecorations(show); return
        }
        if (tauriWindow && typeof tauriWindow.getCurrent === 'function') {
          tauriWindow.getCurrent().setDecorations(show); return
        }
      }
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().setDecorations(show)
    } catch (_) {}
  }

  // Keep has-player in sync with whether a track is loaded.
  $effect(() => {
    document.documentElement.classList.toggle('has-player', !!$currentTrack)
  })

  // When the playing track changes, update title, recently played, and lyrics.
  let _prevTrackId: string | null = null
  $effect(() => {
    const track = $currentTrack
    if (!track || track.id === _prevTrackId) return
    _prevTrackId = track.id
    document.title = `▶ ${track.title} - Firmium`
    recentlyPlayedSongs.push(track)
    fetchAndShowLyrics(track)
  })

  onMount(async () => {
    try {
      loadedThemes = await tauriInvoke<Theme[]>('list_themes')
    } catch (_) {}
    applyThemeById(SafeStorage.getItem('firmium_theme') || 'firmium')
    applyDecorations()

    // Bootstrap Rust queue state with values from localStorage.
    tauriInvoke('init_playback_settings', {
      volume: get(volume),
      crossfadeEnabled: get(crossfadeEnabled),
      crossfadeDuration: get(crossfadeDuration),
      gaplessEnabled: get(gaplessEnabled),
    }).catch(() => {})

    const unlistenQueue = listenToQueueState()
    document.addEventListener('contextmenu', e => e.preventDefault())

    // Block devtools shortcuts.
    document.addEventListener('keydown', e => {
      const devtoolsKey = e.key === 'F12' ||
        ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'I' || e.key === 'i' || e.key === 'J' || e.key === 'j' || e.key === 'C' || e.key === 'c'))
      if (devtoolsKey) e.preventDefault()
    }, { capture: true })

    tauriInvoke('prewarm_local_library').catch(() => {})

    const savedServer = SafeStorage.getItem('firmium_server')
    const savedUser = SafeStorage.getItem('firmium_user')
    const savePasswordEnabled = SafeStorage.getItem('firmium_save_pass') === 'true'
    const autoLoginEnabled = SafeStorage.getItem('firmium_auto_login') !== 'false'

    if (autoLoginEnabled && savedServer && savedUser) {
      if (savePasswordEnabled) {
        let savedPass: string | null = null
        try {
          savedPass = await Keyring.load(savedUser) as string | null
        } catch (_) {
          showAccountModal.set(true)
        }
        if (savedPass) {
          setupError = 'Connecting…'
          try {
            await doConnect(savedServer, savedUser, savedPass)
          } catch (err: any) {
            clearAuth()
            setupError = err.message ?? 'Auto-login failed'
            showAccountModal.set(true)
          }
        } else if (!$showAccountModal) {
          showAccountModal.set(true)
        }
      } else {
        showAccountModal.set(true)
      }
    }
  })

  async function doConnect(sUrl: string, uName: string, pWord: string) {
    let parsed: URL
    try { parsed = new URL(sUrl) } catch (_) { throw new Error('Invalid URL format') }
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') throw new Error('Protocol must be HTTP or HTTPS')
    setAuth(sUrl, uName, pWord)
    try {
      await tauriInvoke('validate_connection')
    } catch (err: any) {
      clearAuth()
      if (err === 'SESSION_EXPIRED') throw new Error('Wrong username or password')
      throw err
    }
    openSubsonicExtensions.set(await tauriInvoke<string[]>('get_open_subsonic_extensions'))
    navToView('home')
    checkRemotePlayQueue()
  }

  function handleSessionExpired() {
    clearAuth()
    setupError = 'Session expired — please reconnect'
    showAccountModal.set(true)
  }

  onMount(() => {
    const unlisten = listen('firmium:session-expired', handleSessionExpired)
    return () => { unlisten.then(f => f()) }
  })

  // Collapse the sidebar to a bottom tab bar (mobile) or icon-only rail (narrow desktop)
  // depending on available width.
  onMount(() => {
    const mobileQuery = window.matchMedia('(max-width: 640px)')
    const collapsedQuery = window.matchMedia('(max-width: 900px)')
    const update = () => {
      document.documentElement.classList.toggle('is-mobile-layout', mobileQuery.matches)
      document.documentElement.classList.toggle('sidebar-collapsed', collapsedQuery.matches && !mobileQuery.matches)
    }
    update()
    mobileQuery.addEventListener('change', update)
    collapsedQuery.addEventListener('change', update)
    return () => {
      mobileQuery.removeEventListener('change', update)
      collapsedQuery.removeEventListener('change', update)
    }
  })

  // Dragging audio files/folders onto the window copies them into ~/Music/Firmium.
  onMount(() => {
    let unlisten: (() => void) | undefined
    ;(async () => {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      unlisten = await getCurrentWindow().onDragDropEvent(async (event) => {
        switch (event.payload.type) {
          case 'enter':
          case 'over':
            dragActive = true
            break
          case 'drop':
            dragActive = false
            try {
              await importLocalFiles(event.payload.paths)
              bumpDataSourceVersion()
            } catch (e) {
              console.error('Import failed:', e)
            }
            break
          default:
            dragActive = false
        }
      })
    })()
    return () => { unlisten?.() }
  })
</script>

<div class="sidebar">
  <Sidebar />
</div>
<div class="main-area">
  <div class="list-panel">
    {#if $activeView.type === 'home'}
      <HomeView />
    {:else if $activeView.type === 'albums'}
      <AlbumList />
    {:else if $activeView.type === 'album'}
      <AlbumDetail id={$activeView.id!} />
    {:else if $activeView.type === 'artists'}
      <ArtistList />
    {:else if $activeView.type === 'artist'}
      <ArtistDetail id={$activeView.id!} />
    {:else if $activeView.type === 'search'}
      <SearchView />
    {:else if $activeView.type === 'playlists'}
      <PlaylistsView />
    {:else if $activeView.type === 'playlist'}
      <PlaylistDetail id={$activeView.id!} />
    {:else if $activeView.type === 'settings'}
      <Settings onapplyTheme={applyThemeById} onapplyDecorations={applyDecorations} themes={loadedThemes} />
    {/if}
  </div>
</div>
<LyricsPanel />
<SimilarTracksPanel />
<VisualizerPanel />
<PlayerBar />
<PlaylistMenu />
{#if $showAccountModal}
  <AccountModal bind:error={setupError} {doConnect} />
{/if}
{#if remoteQueue}
  <ResumeQueuePrompt {remoteQueue} onDismiss={() => remoteQueue = null} />
{/if}
{#if dragActive}
  <div class="drop-overlay">
    <div class="drop-overlay-content">Drop to add to your library</div>
  </div>
{/if}
