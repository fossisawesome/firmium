<script>
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import {
    isAuthed, setAuth, clearAuth, navToView,
    authServer, activeView, lyricsOpen, lyricsTrackId, lyricsLines
  } from './lib/stores.js'
  import { SafeStorage } from './lib/utils.js'
  import { Keyring } from './lib/api.js'
  import { fetchAndShowLyrics } from './lib/playback.js'
  import { playlistMenuState, hidePlaylistMenu } from './lib/playlistMenu.js'
  import { Api } from './lib/api.js'

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

  function applyTheme(theme) {
    if (!theme || theme === 'firmium') document.documentElement.removeAttribute('data-theme')
    else document.documentElement.setAttribute('data-theme', theme)
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

  onMount(async () => {
    applyTheme(SafeStorage.getItem('firmium_theme'))
    applyDecorations()
    document.addEventListener('contextmenu', e => e.preventDefault())

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
    await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: 1 })
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
        <Settings onapplyTheme={val => applyTheme(val)} onapplyDecorations={applyDecorations} />
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
