<script lang="ts">
  import { activeView, navToView, isAuthed, authServer, openAccountModal, type ViewType } from '../lib/stores'
  import {
    IconHome, IconDisc, IconMusic, IconSearch,
    IconList, IconSettings, IconHexagon, IconLogo, IconUser
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
    if (!$isAuthed) return 'Local Files'
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

</script>

<div class="app-brand">
  <span class="icon logo" style="width:20px;height:20px">{@html IconLogo}</span>
  <span class="server-lbl">{serverLabel}</span>
  <button class="account-btn" title={$isAuthed ? 'Account' : 'Connect to server'} aria-label={$isAuthed ? 'Account' : 'Connect to server'} onclick={openAccountModal}>
    <span class="icon" style="width:16px;height:16px" aria-hidden="true">{@html IconUser}</span>
  </button>
</div>

<div class="nav-links">
  {#each visibleItems as item}
    <button
      class="nav-btn"
      class:active={isActive(item.view)}
      aria-current={isActive(item.view) ? 'page' : undefined}
      aria-label={item.label}
      onclick={() => navToView(item.view)}
    >
      <span class="icon nav-icon" aria-hidden="true">{@html item.icon}</span>
      <span class="nav-label">{item.label}</span>
    </button>
  {/each}
</div>
