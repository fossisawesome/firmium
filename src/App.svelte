<script>
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import {
    isAuthed, setAuth, clearAuth, navToView,
    authServer, activeView, lyricsOpen, lyricsTrackId, lyricsLines, currentTrack
  } from './lib/stores.js'
  import { SafeStorage } from './lib/utils.js'
  import { Keyring } from './lib/api.js'
  import { tauriInvoke } from './lib/tauri.js'
  import { fetchAndShowLyrics } from './lib/playback.js'
  import { playlistMenuState, hidePlaylistMenu } from './lib/playlistMenu.js'
  import { Api } from './lib/api.js'

  import { isMobile } from './lib/platform.js'
  import { mobilePlayerOpen, queueSheetOpen } from './lib/stores.js'
  import MobilePlayer from './components/MobilePlayer.svelte'
  import QueueSheet from './components/QueueSheet.svelte'

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
  let loadedThemes = $state([])

  // Apply a theme by setting CSS custom properties directly on :root.
  // Falls back gracefully if the theme ID isn't in the loaded list.
  function applyThemeById(id) {
    const theme = loadedThemes.find(t => t.id === id)
    if (theme) applyThemeData(theme)
  }

  function applyThemeData(theme) {
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
      if (window.__TAURI__) {
        const tauriWindow = window.__TAURI__.window || window.__TAURI__
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

  // Apply mobile layout class when on Android or a narrow viewport.
  // Drives the is-mobile-layout CSS rules instead of a media query so Android
  // tablets wider than 640px also get the mobile layout.
  $effect(() => {
    const mql = window.matchMedia('(max-width: 640px)')
    const syncLayout = () => {
      document.documentElement.classList.toggle('is-mobile-layout', isMobile || mql.matches)
    }
    syncLayout()
    mql.addEventListener('change', syncLayout)
    return () => mql.removeEventListener('change', syncLayout)
  })

  onMount(async () => {
    try {
      loadedThemes = await tauriInvoke('list_themes')
    } catch (_) {}
    applyThemeById(SafeStorage.getItem('firmium_theme') || 'firmium')
    applyDecorations()
    document.addEventListener('contextmenu', e => e.preventDefault())

    // Block devtools shortcuts unless --debug was passed at launch.
    const debugMode = await tauriInvoke('is_debug_mode').catch(() => false)
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
        const savedPass = await Keyring.load(savedUser)
        if (savedPass) {
          setupError = 'Connecting…'
          try {
            await doConnect(savedServer, savedUser, savedPass)
          } catch (err) {
            clearAuth()
            setupError = err.message ?? 'Auto-login failed'
          }
        }
      } catch (_) {
        // Keyring entry may not exist yet — user will need to log in manually.
      }
    }
  })

  async function doConnect(sUrl, uName, pWord) {
    let parsed
    try { parsed = new URL(sUrl) } catch (_) { throw new Error('Invalid URL format') }
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') throw new Error('Protocol must be HTTP or HTTPS')
    setAuth(sUrl, uName, pWord)
    try {
      await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: 1 })
    } catch (err) {
      clearAuth()
      throw err
    }
    navToView('home')
  }

  function handleSessionExpired() {
    clearAuth()
    setupError = 'Session expired — please reconnect'
  }
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
        <AlbumDetail id={$activeView.id} />
      {:else if $activeView.type === 'artists'}
        <ArtistList />
      {:else if $activeView.type === 'artist'}
        <ArtistDetail id={$activeView.id} />
      {:else if $activeView.type === 'search'}
        <SearchView />
      {:else if $activeView.type === 'playlists'}
        <PlaylistsView />
      {:else if $activeView.type === 'playlist'}
        <PlaylistDetail id={$activeView.id} />
      {:else if $activeView.type === 'settings'}
        <Settings onapplyTheme={applyThemeById} onapplyDecorations={applyDecorations} themes={loadedThemes} />
      {/if}
    </div>
  </div>
  <LyricsPanel />
  {#if $currentTrack || !isMobile}
    <PlayerBar />
  {/if}
  <PlaylistMenu />
  {#if isMobile && $mobilePlayerOpen}
    <MobilePlayer />
  {/if}
  {#if isMobile && $queueSheetOpen}
    <QueueSheet />
  {/if}
{:else}
  <div id="setup">
    <Setup bind:error={setupError} {doConnect} />
  </div>
{/if}
