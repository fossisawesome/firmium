<script lang="ts">
  import { app, navTo } from '../lib/stores.svelte'
  import { Api } from '../lib/api'
  import { artistPhoto } from '../lib/cover'
  import { errorMessage } from '../lib/ipc'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import type { Artist } from '../lib/types'

  const ROW_HEIGHT = 64
  let artists = $state<Artist[]>([])
  let loading = $state(true)
  let error = $state('')

  $effect(() => {
    app.dataVersion
    let live = true
    loading = true
    error = ''
    Api.artists()
      .then(r => { if (live) artists = r })
      .catch(e => { if (live) error = errorMessage(e) })
      .finally(() => { if (live) loading = false })
    return () => { live = false }
  })
</script>

<div class="section-header">Artists</div>

<LoadingState {loading} {error} empty={artists.length === 0} loadingMessage="Loading artists…" emptyMessage="No artists found.">
  <div class="artist-list">
    <VirtualList items={artists} itemHeight={ROW_HEIGHT}>
      {#snippet children(artist)}
        <div class="artist-row" role="button" tabindex="0" onclick={() => navTo('artist', artist.id)}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navTo('artist', artist.id)}>
          <img class="artist-row-avatar" {@attach artistPhoto(artist.id)} alt="" />
          <div class="artist-info">
            <div class="artist-name">{artist.name}</div>
            <div class="artist-album-count">{artist.albumCount} albums</div>
          </div>
        </div>
      {/snippet}
    </VirtualList>
  </div>
</LoadingState>
