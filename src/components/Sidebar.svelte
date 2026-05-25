<script>
  import { get } from 'svelte/store'
  import { activeView, navToView, clearAuth, authServer, audioBridge } from '../lib/stores.js'
  import { stopPositionTracking } from '../lib/playback.js'
  import { clearAll } from '../lib/coverCache.js'

  const NAV_ITEMS = [
    { view: 'home', label: 'Home' },
    { view: 'albums', label: 'Albums' },
    { view: 'artists', label: 'Artists' },
    { view: 'search', label: 'Search' },
    { view: 'playlists', label: 'Playlists' },
    { view: 'settings', label: 'Settings' },
  ]

  // Derive the hostname label from the stored server URL.
  const serverLabel = $derived((() => {
    try { return new URL($authServer ?? '').hostname } catch (_) { return 'online' }
  })())

  // Active nav button highlights for both top-level views and their sub-views.
  function isActive(view) {
    const t = $activeView.type
    if (view === 'home') return t === 'home'
    if (view === 'albums') return t === 'albums' || t === 'album'
    if (view === 'artists') return t === 'artists' || t === 'artist'
    if (view === 'playlists') return t === 'playlists' || t === 'playlist'
    return t === view
  }

  async function handleLogout() {
    const bridge = get(audioBridge)
    if (bridge) { bridge.destroy() }
    stopPositionTracking()
    clearAll()
    clearAuth()
    document.title = 'Firmium'
  }
</script>

<div class="app-brand">
  <span class="logo">⬡</span>
  <span class="server-lbl">{serverLabel}</span>
</div>

<div class="nav-links">
  {#each NAV_ITEMS as item}
    <button
      class="nav-btn"
      class:active={isActive(item.view)}
      onclick={() => navToView(item.view)}
    >
      {item.label}
    </button>
  {/each}
</div>

<button class="logout-btn" onclick={handleLogout}>✕ disconnect</button>
