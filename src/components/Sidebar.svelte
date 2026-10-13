<script lang="ts">
  import { app, navTo, type ViewType } from '../lib/stores.svelte'
    import {
    IconHome, IconDisc, IconMusic, IconSearch, IconList, IconSettings,
    IconUser, IconWaveform, IconHeartOutline, IconPodcast,
  } from '../lib/icons'

  const NAV_ITEMS: { view: ViewType; label: string; icon: string; mobileOnly?: boolean }[] = [
    { view: 'home',      label: 'Home',      icon: IconHome },
    { view: 'favorites', label: 'Favorites', icon: IconHeartOutline },
    { view: 'albums',    label: 'Albums',    icon: IconDisc },
    { view: 'artists',   label: 'Artists',   icon: IconMusic },
    { view: 'playlists', label: 'Playlists', icon: IconList },
    { view: 'podcasts',  label: 'Podcasts',  icon: IconPodcast },
    { view: 'search',    label: 'Search',    icon: IconSearch, mobileOnly: true },
    { view: 'mix',       label: 'Mix',       icon: IconWaveform },
    { view: 'settings',  label: 'Settings',  icon: IconSettings },
  ]

  // Detail views highlight the list they belong to.
  const PARENT: Partial<Record<ViewType, ViewType>> = {
    album: 'albums', artist: 'artists', playlist: 'playlists', podcast: 'podcasts', genre: 'home',
  }

  const isActive = (view: ViewType) => (app.activeView.type === view) || PARENT[app.activeView.type] === view
</script>

<div class="app-brand">
  <button class={["account-btn", { active: app.activeView.type === 'search' }]} title="Search" aria-label="Search" onclick={() => navTo('search')}>
    <span class="icon" style="width:16px;height:16px" aria-hidden="true">{@html IconSearch}</span>
  </button>
  <button class="account-btn" title={app.isAuthed ? 'Account' : 'Connect to server'} aria-label={app.isAuthed ? 'Account' : 'Connect to server'} onclick={() => (app.showAccountModal = true)}>
    <span class="icon" style="width:16px;height:16px" aria-hidden="true">{@html IconUser}</span>
  </button>
</div>

<div class="nav-links">
  {#each NAV_ITEMS as item}
    <button class={["nav-btn", { active: isActive(item.view), 'mobile-only': item.mobileOnly }]}
      aria-current={isActive(item.view) ? 'page' : undefined}
      aria-label={item.label}
      onclick={() => navTo(item.view)}>
      <span class="icon nav-icon" aria-hidden="true">{@html item.icon}</span>
      <span class="nav-label">{item.label}</span>
    </button>
  {/each}
</div>
