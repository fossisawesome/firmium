<script lang="ts">
  import { activeView, navTo, isAuthed, auth, showAccountModal, settings, type ViewType } from '../lib/stores'
  import { localPlaylists, loadPlaylists } from '../lib/playlists'
  import {
    IconHome, IconDisc, IconMusic, IconSearch, IconList, IconSettings,
    IconLogo, IconUser, IconWaveform, IconHeartOutline, IconPodcast,
  } from '../lib/icons'

  const NAV_ITEMS: { view: ViewType; label: string; icon: string }[] = [
    { view: 'home',      label: 'Home',      icon: IconHome },
    { view: 'favorites', label: 'Favorites', icon: IconHeartOutline },
    { view: 'albums',    label: 'Albums',    icon: IconDisc },
    { view: 'artists',   label: 'Artists',   icon: IconMusic },
    { view: 'playlists', label: 'Playlists', icon: IconList },
    { view: 'podcasts',  label: 'Podcasts',  icon: IconPodcast },
    { view: 'search',    label: 'Search',    icon: IconSearch },
    { view: 'mix',       label: 'Mix',       icon: IconWaveform },
    { view: 'settings',  label: 'Settings',  icon: IconSettings },
  ]

  // Detail views highlight the list they belong to.
  const PARENT: Partial<Record<ViewType, ViewType>> = {
    album: 'albums', artist: 'artists', playlist: 'playlists', podcast: 'podcasts', genre: 'home', recap: 'home',
  }

  const serverLabel = $derived.by(() => {
    if (!$isAuthed) return 'Local Files'
    try { return new URL($auth.server ?? '').hostname } catch { return 'online' }
  })

  // Spotify layout shows the playlist library under the nav.
  const spotify = $derived($settings?.uiThemeId === 'spotify')
  $effect(() => { if (spotify) loadPlaylists(false).catch(() => {}) })

  const isActive = (view: ViewType) => ($activeView.type === view) || PARENT[$activeView.type] === view
</script>

<div class="app-brand">
  <span class="icon logo" style="width:20px;height:20px">{@html IconLogo}</span>
  <span class="server-lbl">{serverLabel}</span>
  <button class="account-btn" title={$isAuthed ? 'Account' : 'Connect to server'} aria-label={$isAuthed ? 'Account' : 'Connect to server'} onclick={() => showAccountModal.set(true)}>
    <span class="icon" style="width:16px;height:16px" aria-hidden="true">{@html IconUser}</span>
  </button>
</div>

<div class="nav-links">
  {#each NAV_ITEMS as item}
    <button
      class="nav-btn"
      class:active={isActive(item.view)}
      aria-current={isActive(item.view) ? 'page' : undefined}
      aria-label={item.label}
      onclick={() => navTo(item.view)}
    >
      <span class="icon nav-icon" aria-hidden="true">{@html item.icon}</span>
      <span class="nav-label">{item.label}</span>
    </button>
  {/each}
</div>

{#if spotify}
  <div class="spotify-library">
    <div class="spotify-library-title">Your Library</div>
    {#each $localPlaylists as pl (pl.id)}
      <button class="spotify-library-row" onclick={() => navTo('playlist', pl.id)}>{pl.name}</button>
    {/each}
  </div>
{/if}
