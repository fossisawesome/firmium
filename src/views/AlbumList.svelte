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

<div class="section-header">Albums</div>

{#if loading}
  <div class="loading-msg">Loading albums…</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else if albums.length === 0}
  <div class="loading-msg">No albums found.</div>
{:else}
  {#each albums as album}
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
        title="Add album to playlist"
        onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'album', albumId: album.id, albumName: album.name }) }}
      >+</button>
    </div>
  {/each}
{/if}
