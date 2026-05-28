<script>
  import { onMount, onDestroy } from 'svelte'
  import { navToArtist } from '../lib/stores.js'
  import { Api } from '../lib/api.js'
  import { lazyLoad } from '../lib/lazyLoad.js'

  let artists = $state([])
  let loading = $state(true)
  let error = $state('')
  let ctrl

  onMount(async () => {
    ctrl = new AbortController()
    try {
      artists = await Api.getArtists(ctrl.signal)
    } catch (e) {
      if (!ctrl.signal.aborted) error = e.message
    } finally {
      if (!ctrl.signal.aborted) loading = false
    }
  })

  onDestroy(() => ctrl?.abort())

  // Fetch artist photo from the server's MusicBrainz/Last.fm integration.
  // Called lazily via the lazyLoad directive so only visible rows fire requests.
  function loadArtistImage(img, artistId) {
    Api.getArtistInfo(artistId, ctrl?.signal).then(info => {
      if (info?.image && !ctrl?.signal?.aborted) img.src = info.image
    }).catch(() => {})
  }

  const DEFAULT_AVATAR = `data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23888' viewBox='0 0 24 24'><path d='M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z'/></svg>`
</script>

<div class="section-header">Artists</div>

{#if loading}
  <div class="loading-msg">Loading artists…</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else if artists.length === 0}
  <div class="loading-msg">No artists found.</div>
{:else}
  <div class="artist-list">
    {#each artists as artist}
      <div
        class="artist-row"
        role="button"
        tabindex="0"
        onclick={() => navToArtist(artist.id)}
        onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToArtist(artist.id)}
      >
        <!-- Circular artist photo; falls back to default avatar when no server image -->
        <img
          class="artist-row-avatar"
          src={DEFAULT_AVATAR}
          alt=""
          use:lazyLoad={img => loadArtistImage(img, artist.id)}
        />
        <div class="artist-info">
          <div class="artist-name">{artist.name}</div>
          <div class="artist-album-count">{artist.albumCount} albums</div>
        </div>
      </div>
    {/each}
  </div>
{/if}
