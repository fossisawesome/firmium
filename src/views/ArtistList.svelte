<script lang="ts">
  import { onMount } from 'svelte'
  import { navToArtist } from '../lib/stores'
  import { Api } from '../lib/api'
  import { lazyLoad } from '../lib/lazyLoad'
  import { getCached, setCached } from '../lib/listCache'
  import { createAbortController } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import type { Artist } from '../lib/types/tauri-commands'

  const ARTIST_ROW_HEIGHT = 64

  let artists = $state<Artist[]>(getCached<Artist[]>('artists') ?? [])
  let loading = $state(artists.length === 0)
  let error = $state('')
  const abortCtrl = createAbortController()

  onMount(async () => {
    if (artists.length > 0) return
    const signal = abortCtrl.renew()
    try {
      artists = await Api.getArtists(signal)
      setCached('artists', artists)
    } catch (e: any) {
      if (!signal.aborted) error = e.message
    } finally {
      if (!signal.aborted) loading = false
    }
  })

  // Fetch artist photo from the server's MusicBrainz/Last.fm integration.
  // Called lazily via the lazyLoad directive so only visible rows fire requests.
  // Results are cached so revisiting this view doesn't re-fetch every artist's info.
  function loadArtistImage(img: HTMLImageElement, artistId: string) {
    const cacheKey = `artistInfo:${artistId}`
    const cached = getCached<{ image?: string }>(cacheKey)
    if (cached) {
      if (cached.image) img.src = cached.image
      return
    }
    Api.getArtistInfo(artistId, abortCtrl.signal).then(info => {
      setCached(cacheKey, info ?? {})
      if (info?.image && !abortCtrl.signal?.aborted) img.src = info.image
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
    <VirtualList items={artists} itemHeight={ARTIST_ROW_HEIGHT}>
      {#snippet children(artist, _index)}
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
      {/snippet}
    </VirtualList>
  </div>
{/if}
