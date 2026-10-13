<script lang="ts">
  import { onMount } from 'svelte'
  import type { Attachment } from 'svelte/attachments'
  import { app, initBackend, setAuthState, navTo, bumpData, toast, toastError, navBack, navForward, setScrollHost } from './lib/stores.svelte'
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

  const scrollHost: Attachment<HTMLElement> = el => { setScrollHost(el); return () => setScrollHost(null) }

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
    app.showAccountModal = true
  }

  $effect(() => { document.documentElement.classList.toggle('has-player', !!app.currentTrack) })
  $effect(() => { document.title = app.currentTrack ? `▶ ${app.currentTrack.title} - Firmium` : 'Firmium' })

  // Offer once per launch, and only while nothing is playing (same as Android).
  let resumeOffered = false
  async function offerResume() {
    if (resumeOffered || app.currentTrack) return
    resumeOffered = true
    try {
      const q = await Api.remotePlayQueue()
      showResume = !!q && q.entries.length > 0 && !app.currentTrack
    } catch { /* offline or unsupported */ }
  }

  onMount(() => {
    let stop: (() => void) | undefined
    ;(async () => {
      stop = await initBackend()
      try {
        const a = await Api.autoLogin()
        if (a) { await setAuthState(a); offerResume() }
        if (!app.auth.authed) {
          let onboarded = false
          try { onboarded = localStorage.getItem(ONBOARDED) === 'true' } catch { /* ignore */ }
          if (!onboarded && app.auth.accounts.length === 0) showOnboarding = true
        }
      } catch (e) {
        setupError = errorMessage(e)
        app.showAccountModal = true
      }
      ready = true
    })()

    document.addEventListener('contextmenu', e => e.preventDefault())

    // Mouse side buttons: 3 = back, 4 = forward.
    const onMouseUp = (e: MouseEvent) => {
      if (e.button === 3) { e.preventDefault(); navBack() }
      else if (e.button === 4) { e.preventDefault(); navForward() }
    }
    window.addEventListener('mouseup', onMouseUp)

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
      window.removeEventListener('mouseup', onMouseUp)
      setScrollHost(null)
      unlistenDrop?.()
      mobileQuery.removeEventListener('change', update)
      collapsedQuery.removeEventListener('change', update)
    }
  })
</script>

{#if showOnboarding}<Onboarding onFinish={finishOnboarding} />{/if}
<div class="sidebar"><Sidebar /></div>
<div class="main-area">
  <div class="list-panel" {@attach scrollHost}>
    {#if ready}
      {#key app.activeView.type + (app.activeView.id ?? '')}
        {#if app.activeView.type === 'home'}<HomeView />
        {:else if app.activeView.type === 'favorites'}<FavoritesView />
        {:else if app.activeView.type === 'albums'}<AlbumList />
        {:else if app.activeView.type === 'album'}<AlbumDetail id={app.activeView.id!} />
        {:else if app.activeView.type === 'artists'}<ArtistList />
        {:else if app.activeView.type === 'artist'}<ArtistDetail id={app.activeView.id!} />
        {:else if app.activeView.type === 'genre'}<GenreView name={app.activeView.id!} />
        {:else if app.activeView.type === 'mix'}<MixView />
        {:else if app.activeView.type === 'search'}<SearchView />
        {:else if app.activeView.type === 'playlists'}<PlaylistsView />
        {:else if app.activeView.type === 'playlist'}<PlaylistDetail id={app.activeView.id!} />
        {:else if app.activeView.type === 'podcasts'}<PodcastsView />
        {:else if app.activeView.type === 'podcast'}<PodcastDetail id={app.activeView.id!} />
        {:else if app.activeView.type === 'settings'}<Settings />
        {/if}
      {/key}
    {/if}
  </div>
</div>
<QueuePanel />
<LyricsPanel />
<SimilarTracksPanel />
<AudioStatsPanel />
<PlayerBar />
<PlaylistMenu />
<Toasts />
{#if app.showAccountModal}<AccountModal bind:error={setupError} />{/if}
{#if showResume}<ResumeQueuePrompt onDismiss={() => (showResume = false)} />{/if}
{#if dragActive}
  <div class="drop-overlay"><div class="drop-overlay-content">Drop to add to your library</div></div>
{/if}
