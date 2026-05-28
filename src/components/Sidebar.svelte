<script>
  import { get } from 'svelte/store'
  import { activeView, navToView, clearAuth, authServer, audioBridge } from '../lib/stores.js'
  import { stopPositionTracking } from '../lib/playback.js'
  import { clearAll } from '../lib/coverCache.js'
  import { isMobile } from '../lib/platform.js'
  import {
    IconHome, IconDisc, IconMusic, IconSearch,
    IconList, IconSettings, IconHexagon, IconClose
  } from '../lib/icons.js'

  const NAV_ITEMS = [
    { view: 'home',      label: 'Home',      icon: IconHome },
    { view: 'albums',    label: 'Albums',    icon: IconDisc },
    { view: 'artists',   label: 'Artists',   icon: IconMusic },
    { view: 'search',    label: 'Search',    icon: IconSearch },
    { view: 'playlists', label: 'Playlists', icon: IconList },
    { view: 'settings',  label: 'Settings',  icon: IconSettings },
  ]

  // Search and Settings are hidden from the mobile tab bar — they're accessed
  // via the header icons on each page instead.
  const MOBILE_HIDDEN_TABS = new Set(['search', 'settings'])
  const visibleItems = $derived(
    isMobile ? NAV_ITEMS.filter(item => !MOBILE_HIDDEN_TABS.has(item.view)) : NAV_ITEMS
  )

  const serverLabel = $derived((() => {
    try { return new URL($authServer ?? '').hostname } catch (_) { return 'online' }
  })())

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
  <span class="icon logo" style="width:20px;height:20px;color:var(--accent)">{@html IconHexagon}</span>
  <span class="server-lbl">{serverLabel}</span>
</div>

<div class="nav-links">
  {#each visibleItems as item}
    <button
      class="nav-btn"
      class:active={isActive(item.view)}
      onclick={() => navToView(item.view)}
    >
      <span class="icon nav-icon">{@html item.icon}</span>
      <span class="nav-label">{item.label}</span>
    </button>
  {/each}
</div>

<button class="logout-btn" onclick={handleLogout}>
  <span class="icon" style="width:11px;height:11px;margin-right:4px">{@html IconClose}</span>disconnect
</button>
