<script lang="ts">
  import { navToArtist } from '../lib/stores'
  import { dataSource } from '../lib/dataSource'
  import { dataSourceVersion } from '../lib/stores'
  import { lazyLoad } from '../lib/lazyLoad'
  import { getCached, setCached } from '../lib/listCache'
  import { createAbortController } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import type { Artist } from '../lib/types/tauri-commands'

  const ARTIST_ROW_HEIGHT = 64

  let artists = $state<Artist[]>(getCached<Artist[]>('artists') ?? [])
  let loading = $state(artists.length === 0)
  let error = $state('')
  const abortCtrl = createAbortController()

  let initialized = false

  $effect(() => {
    const source = $dataSource
    $dataSourceVersion
    if (!initialized && artists.length > 0) { initialized = true; return }
    initialized = true
    loading = true
    error = ''
    const signal = abortCtrl.renew()
    ;(async () => {
      try {
        artists = await source.getArtists(signal)
        setCached('artists', artists)
      } catch (e: any) {
        if (!signal.aborted) error = e.message
      } finally {
        if (!signal.aborted) loading = false
      }
    })()
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
    $dataSource.getArtistInfo(artistId, abortCtrl.signal).then(info => {
      setCached(cacheKey, info ?? {})
      if (info?.image && !abortCtrl.signal?.aborted) img.src = info.image
    }).catch(() => {})
  }

  const DEFAULT_AVATAR = `data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23888' viewBox='0 0 24 24'><path d='M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z'/></svg>`
</script>

<div class="section-header">Artists</div>

<LoadingState {loading} {error} empty={artists.length === 0} loadingMessage="Loading artists…" emptyMessage="No artists found.">
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
</LoadingState>
