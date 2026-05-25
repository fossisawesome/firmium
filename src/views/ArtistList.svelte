<script>
  import { onMount, onDestroy } from 'svelte'
  import { navToArtist } from '../lib/stores.js'
  import { Api } from '../lib/api.js'

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
        <div class="artist-info">
          <div class="artist-name">{artist.name}</div>
          <div class="artist-album-count">{artist.albumCount} albums</div>
        </div>
      </div>
    {/each}
  </div>
{/if}
