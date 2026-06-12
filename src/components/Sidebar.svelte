<script lang="ts">
  import { get } from 'svelte/store'
  import { activeView, navToView, clearAuth, authServer, audioBridge, type ViewType } from '../lib/stores'
  import { stopPositionTracking } from '../lib/playback'
  import { clearAll } from '../lib/coverCache'
  import { clearAll as clearListCache } from '../lib/listCache'
  import {
    IconHome, IconDisc, IconMusic, IconSearch,
    IconList, IconSettings, IconHexagon, IconClose
  } from '../lib/icons'

  const NAV_ITEMS: { view: ViewType; label: string; icon: string }[] = [
    { view: 'home',      label: 'Home',      icon: IconHome },
    { view: 'albums',    label: 'Albums',    icon: IconDisc },
    { view: 'artists',   label: 'Artists',   icon: IconMusic },
    { view: 'search',    label: 'Search',    icon: IconSearch },
    { view: 'playlists', label: 'Playlists', icon: IconList },
    { view: 'settings',  label: 'Settings',  icon: IconSettings },
  ]

  const visibleItems = $derived(NAV_ITEMS)

  const serverLabel = $derived((() => {
    try { return new URL($authServer ?? '').hostname } catch (_) { return 'online' }
  })())

  function isActive(view: ViewType) {
    const t = $activeView.type
    const parent = $activeView.parentType
    if (view === 'home')    return t === 'home' || parent === 'home'
    if (view === 'albums')  return (t === 'albums') || (t === 'album' && parent !== 'home')
    if (view === 'artists') return (t === 'artists') || (t === 'artist' && parent !== 'home')
    if (view === 'playlists') return t === 'playlists' || t === 'playlist'
    return t === view
  }

  async function handleLogout() {
    const bridge = get(audioBridge)
    if (bridge) { bridge.destroy() }
    stopPositionTracking()
    clearAll()
    clearListCache()
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
