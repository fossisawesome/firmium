<script>
  import { playlists, navToPlaylist } from '../lib/stores.js'
  import { loadImage } from '../lib/api.js'
  import { lazyLoad } from '../lib/lazyLoad.js'

  function createNew() {
    const name = prompt('Playlist name:')
    if (name?.trim()) {
      const pl = playlists.create(name.trim())
      navToPlaylist(pl.id)
    }
  }
</script>

{#if $playlists.length === 0}
  <div class="section-header">Playlists</div>
  <div class="pl-empty-state">
    <div class="pl-empty-icon">♫</div>
    <div>No playlists yet</div>
    <button class="pl-new-btn" onclick={createNew}>New Playlist</button>
  </div>
{:else}
  <div class="pl-list-header">
    <span class="section-header" style="margin:0">Playlists</span>
    <button class="pl-new-btn" onclick={createNew}>+ New</button>
  </div>
  {#each $playlists as pl}
    <div
      class="pl-card"
      role="button"
      tabindex="0"
      onclick={() => navToPlaylist(pl.id)}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToPlaylist(pl.id)}
    >
      <div class="pl-card-art">
        {#if pl.coverDataUrl}
          <img src={pl.coverDataUrl} alt="" />
        {:else if pl.coverArtId}
          <img use:lazyLoad={img => loadImage(img, pl.coverArtId, null)} alt="" />
        {:else}
          <div class="no-art">♫</div>
        {/if}
      </div>
      <div class="pl-card-info">
        <div class="pl-card-name">{pl.name}</div>
        <div class="pl-card-meta">
          {pl.tracks.length} track{pl.tracks.length !== 1 ? 's' : ''}
          {#if pl.description} · {pl.description.slice(0, 60)}{/if}
        </div>
      </div>
    </div>
  {/each}
{/if}
