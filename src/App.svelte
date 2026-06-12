<script lang="ts">
  import { onMount } from 'svelte'
  import {
    isAuthed, setAuth, clearAuth, navToView,
    authServer, activeView, lyricsOpen, currentTrack,
  } from './lib/stores'
  import { SafeStorage } from './lib/utils'
  import { Keyring } from './lib/api'
  import { tauriInvoke } from './lib/tauri'
  import { fetchAndShowLyrics } from './lib/playback'
  import { playlistMenuState, hidePlaylistMenu } from './lib/playlistMenu'
  import { Api } from './lib/api'
  import type { Theme } from './lib/types/tauri-commands'

  import Setup from './components/Setup.svelte'
  import Sidebar from './components/Sidebar.svelte'
  import PlayerBar from './components/PlayerBar.svelte'
  import LyricsPanel from './components/LyricsPanel.svelte'
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

  onMount(async () => {
    try {
      loadedThemes = await tauriInvoke<Theme[]>('list_themes')
    } catch (_) {}
    applyThemeById(SafeStorage.getItem('firmium_theme') || 'firmium')
    applyDecorations()
    document.addEventListener('contextmenu', e => e.preventDefault())

    // Block devtools shortcuts unless --debug was passed at launch.
    const debugMode = await tauriInvoke<boolean>('is_debug_mode').catch(() => false)
    if (!debugMode) {
      document.addEventListener('keydown', e => {
        const devtoolsKey = e.key === 'F12' ||
          ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'I' || e.key === 'i' || e.key === 'J' || e.key === 'j' || e.key === 'C' || e.key === 'c'))
        if (devtoolsKey) e.preventDefault()
      }, { capture: true })
    }

    const savedServer = SafeStorage.getItem('firmium_server')
    const savedUser = SafeStorage.getItem('firmium_user')
    const savePasswordEnabled = SafeStorage.getItem('firmium_save_pass') === 'true'
    const autoLoginEnabled = SafeStorage.getItem('firmium_auto_login') !== 'false'

    if (autoLoginEnabled && savePasswordEnabled && savedServer && savedUser) {
      try {
        const savedPass = await Keyring.load(savedUser) as string | null
        if (savedPass) {
          setupError = 'Connecting…'
          try {
            await doConnect(savedServer, savedUser, savedPass)
          } catch (err: any) {
            clearAuth()
            setupError = err.message ?? 'Auto-login failed'
          }
        }
      } catch (_) {}
    }
  })

  async function doConnect(sUrl: string, uName: string, pWord: string) {
    let parsed: URL
    try { parsed = new URL(sUrl) } catch (_) { throw new Error('Invalid URL format') }
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') throw new Error('Protocol must be HTTP or HTTPS')
    setAuth(sUrl, uName, pWord)
    try {
      await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: 1 }, null, { silentSessionExpiry: true })
    } catch (err: any) {
      clearAuth()
      if (err?.code === 'SESSION_EXPIRED') throw new Error('Wrong username or password')
      throw err
    }
    navToView('home')
  }

  function handleSessionExpired() {
    clearAuth()
    setupError = 'Session expired — please reconnect'
  }

  onMount(() => {
    window.addEventListener('firmium:session-expired', handleSessionExpired)
    return () => window.removeEventListener('firmium:session-expired', handleSessionExpired)
  })
</script>

{#if $isAuthed}
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
  <PlayerBar />
  <PlaylistMenu />
{:else}
  <div id="setup">
    <Setup bind:error={setupError} {doConnect} />
  </div>
{/if}
