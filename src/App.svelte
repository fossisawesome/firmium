<script lang="ts">
  import { onMount } from 'svelte'
  import {
    initBackend, auth, setAuthState, activeView, navTo, showAccountModal, isAuthed, currentTrack, bumpData,
    toast, toastError,
  } from './lib/stores'
  import { Api } from './lib/api'
  import { errorMessage } from './lib/ipc'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  import AccountModal from './components/AccountModal.svelte'
  import Onboarding from './components/Onboarding.svelte'
  import ResumeQueuePrompt from './components/ResumeQueuePrompt.svelte'
  import Sidebar from './components/Sidebar.svelte'
  import PlayerBar from './components/PlayerBar.svelte'
  import LyricsPanel from './components/LyricsPanel.svelte'
  import SimilarTracksPanel from './components/SimilarTracksPanel.svelte'
  import QueuePanel from './components/QueuePanel.svelte'
  import EqPanel from './components/EqPanel.svelte'
  import AudioStatsPanel from './components/AudioStatsPanel.svelte'
  import PlaylistMenu from './components/PlaylistMenu.svelte'
  import Toasts from './components/Toasts.svelte'
  import AlbumList from './views/AlbumList.svelte'
  import AlbumDetail from './views/AlbumDetail.svelte'
  import ArtistList from './views/ArtistList.svelte'
  import ArtistDetail from './views/ArtistDetail.svelte'
  import SearchView from './views/SearchView.svelte'
  import PlaylistsView from './views/PlaylistsView.svelte'
  import PlaylistDetail from './views/PlaylistDetail.svelte'
  import Settings from './views/Settings.svelte'
  import HomeView from './views/HomeView.svelte'
  import MixView from './views/MixView.svelte'
  import FavoritesView from './views/FavoritesView.svelte'
  import GenreView from './views/GenreView.svelte'
  import PodcastsView from './views/PodcastsView.svelte'
  import PodcastDetail from './views/PodcastDetail.svelte'

  let ready = $state(false)
  let showOnboarding = $state(false)
  let setupError = $state('')
  let dragActive = $state(false)
  let showResume = $state(false)

  // First-run flag is a per-device UI convenience, not app state.
  const ONBOARDED = 'firmium_onboarded'
  function finishOnboarding() {
    try { localStorage.setItem(ONBOARDED, 'true') } catch { /* private mode */ }
    showOnboarding = false
    showAccountModal.set(true)
  }

  $effect(() => { document.documentElement.classList.toggle('has-player', !!$currentTrack) })
  $effect(() => { document.title = $currentTrack ? `▶ ${$currentTrack.title} - Firmium` : 'Firmium' })

  async function offerResume() {
    try {
      const q = await Api.remotePlayQueue()
      showResume = !!q && q.entries.length > 0
    } catch { /* offline or unsupported */ }
  }

  onMount(() => {
    let stop: (() => void) | undefined
    ;(async () => {
      stop = await initBackend()
      try {
        const a = await Api.autoLogin()
        if (a) { await setAuthState(a); offerResume() }
        if (!$auth.authed) {
          let onboarded = false
          try { onboarded = localStorage.getItem(ONBOARDED) === 'true' } catch { /* ignore */ }
          if (!onboarded && $auth.accounts.length === 0) showOnboarding = true
        }
      } catch (e) {
        setupError = errorMessage(e)
        showAccountModal.set(true)
      }
      ready = true
    })()

    document.addEventListener('contextmenu', e => e.preventDefault())

    // Dropping audio files/folders copies them into ~/Music/Firmium.
    let unlistenDrop: (() => void) | undefined
    getCurrentWindow().onDragDropEvent(async event => {
      const p = event.payload
      if (p.type === 'enter' || p.type === 'over') dragActive = true
      else if (p.type === 'drop') {
        dragActive = false
        try {
          const n = await Api.importLocalFiles(p.paths)
          toast(`Imported ${n} file${n === 1 ? '' : 's'}`)
          bumpData()
        } catch (e) { toastError(e) }
      } else dragActive = false
    }).then(u => (unlistenDrop = u))

    // Collapse the sidebar to icons, then to a bottom tab bar, as the window narrows.
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
      stop?.()
      unlistenDrop?.()
      mobileQuery.removeEventListener('change', update)
      collapsedQuery.removeEventListener('change', update)
    }
  })
</script>

{#if showOnboarding}<Onboarding onFinish={finishOnboarding} />{/if}
<div class="sidebar"><Sidebar /></div>
<div class="main-area">
  <div class="list-panel">
    {#if ready}
      {#key $activeView.type + ($activeView.id ?? '')}
        {#if $activeView.type === 'home'}<HomeView />
        {:else if $activeView.type === 'favorites'}<FavoritesView />
        {:else if $activeView.type === 'albums'}<AlbumList />
        {:else if $activeView.type === 'album'}<AlbumDetail id={$activeView.id!} />
        {:else if $activeView.type === 'artists'}<ArtistList />
        {:else if $activeView.type === 'artist'}<ArtistDetail id={$activeView.id!} />
        {:else if $activeView.type === 'genre'}<GenreView name={$activeView.id!} />
        {:else if $activeView.type === 'mix'}<MixView />
        {:else if $activeView.type === 'search'}<SearchView />
        {:else if $activeView.type === 'playlists'}<PlaylistsView />
        {:else if $activeView.type === 'playlist'}<PlaylistDetail id={$activeView.id!} />
        {:else if $activeView.type === 'podcasts'}<PodcastsView />
        {:else if $activeView.type === 'podcast'}<PodcastDetail id={$activeView.id!} />
        {:else if $activeView.type === 'settings'}<Settings />
        {/if}
      {/key}
    {/if}
  </div>
</div>
<QueuePanel />
<LyricsPanel />
<SimilarTracksPanel />
<EqPanel />
<AudioStatsPanel />
<PlayerBar />
<PlaylistMenu />
<Toasts />
{#if $showAccountModal}<AccountModal bind:error={setupError} />{/if}
{#if showResume}<ResumeQueuePrompt onDismiss={() => (showResume = false)} />{/if}
{#if dragActive}
  <div class="drop-overlay"><div class="drop-overlay-content">Drop to add to your library</div></div>
{/if}
