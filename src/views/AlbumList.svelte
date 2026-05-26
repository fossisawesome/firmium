<script>
  import { onMount, onDestroy } from 'svelte'
  import { navToAlbum } from '../lib/stores.js'
  import { Api, loadImage } from '../lib/api.js'
  import { showPlaylistMenu } from '../lib/playlistMenu.js'
  import { lazyLoad } from '../lib/lazyLoad.js'

  let albums = $state([])
  let loading = $state(true)
  let error = $state('')
  let ctrl

  const RELEASE_ORDER = ['album', 'ep', 'single', 'live', 'compilation', 'other']
  const RELEASE_LABELS = { album: 'Albums', ep: 'EPs', single: 'Singles', live: 'Live', compilation: 'Compilations', other: 'Other' }

  const grouped = $derived.by(() => {
    const map = {}
    for (const a of albums) {
      const rt = (a.releaseType ?? 'album').toLowerCase()
      const key = RELEASE_ORDER.includes(rt) ? rt : 'other'
      if (!map[key]) map[key] = []
      map[key].push(a)
    }
    // Only show full albums in the Albums tab; EPs/singles live on artist pages
    return RELEASE_ORDER.filter(k => k === 'album' && map[k]?.length).map(k => ({ key: k, label: RELEASE_LABELS[k], items: map[k] }))
  })

  onMount(async () => {
    ctrl = new AbortController()
    try {
      albums = await Api.getAlbums(ctrl.signal)
    } catch (e) {
      if (!ctrl.signal.aborted) error = e.message
    } finally {
      if (!ctrl.signal.aborted) loading = false
    }
  })

  onDestroy(() => ctrl?.abort())
</script>

{#if loading}
  <div class="loading-msg">Loading albums…</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else if albums.length === 0}
  <div class="loading-msg">No albums found.</div>
{:else}
  {#each grouped as section}
    {#each section.items as album}
      <div
        class="album-row"
        role="button"
        tabindex="0"
        onclick={() => navToAlbum(album.id)}
        onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToAlbum(album.id)}
      >
        <div class="album-art-sm">
          {#if album.coverArtId}
            <img use:lazyLoad={img => loadImage(img, album.coverArtId, ctrl?.signal)} alt="" />
          {:else}
            <div class="no-art">♪</div>
          {/if}
        </div>
        <div class="album-info">
          <div class="album-title">{album.name}</div>
          <div class="album-artist">{album.albumArtist}</div>
        </div>
        <button
          class="album-add-btn"
          title="Add to playlist"
          onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'album', albumId: album.id, albumName: album.name }) }}
        >+</button>
      </div>
    {/each}
  {/each}
{/if}
