<script lang="ts">
  import { currentTrack, toastError } from '../lib/stores'
  import { Api } from '../lib/api'
  import { errorMessage } from '../lib/ipc'
  import TrackRow from '../components/TrackRow.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
  import type { Song, Album } from '../lib/types'

  const MAX_LEN = 500
  const RATING_FILTERS = [0, 1, 2, 3, 4, 5]

  let query = $state('')
  let songs = $state<Song[]>([])
  let albums = $state<Album[]>([])
  let loading = $state(false)
  let error = $state('')
  let searched = $state(false)
  let minRating = $state(0)

  // Filter by the higher of the user's own and the server-wide average rating.
  const visibleSongs = $derived(
    minRating === 0 ? songs : songs.filter(s => Math.max(s.userRating ?? 0, s.averageRating ?? 0) >= minRating),
  )

  async function execute() {
    if (!query.trim()) return
    loading = true
    error = ''
    searched = false
    try {
      const r = await Api.search(query.trim())
      songs = r.songs
      albums = r.albums
      searched = true
    } catch (e) { error = errorMessage(e) }
    finally { loading = false }
  }

  const play = (idx: number) => Api.setQueueSeamless(visibleSongs, idx).catch(toastError)
</script>

<div class="search-row">
  <!-- svelte-ignore a11y_autofocus -->
  <input type="text" placeholder="Search albums, songs…" maxlength={MAX_LEN} bind:value={query}
    onkeydown={e => e.key === 'Enter' && execute()} autofocus />
  <button onclick={execute}>Search</button>
</div>

{#if loading}
  <div class="loading-msg">Searching…</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else if searched}
  {#if songs.length === 0 && albums.length === 0}
    <div class="loading-msg">No results found.</div>
  {:else}
    <div class="search-results-container">
      {#if songs.length > 0}
        <div class="section-header">Songs</div>
        <div class="filter-bar"><div class="filter-group">
          {#each RATING_FILTERS as r}
            <button class="filter-chip" class:active={minRating === r} onclick={() => (minRating = r)}>{r === 0 ? 'Any rating' : `${r}+ ★`}</button>
          {/each}
        </div></div>
        <div class="track-list">
          {#each visibleSongs as track, idx (track.id)}
            <TrackRow {track} {idx} playing={$currentTrack?.id === track.id} onPlay={play} />
          {/each}
        </div>
      {/if}
      {#if albums.length > 0}
        <div class="section-header">Albums</div>
        {#each albums as album (album.id)}<AlbumRow {album} />{/each}
      {/if}
    </div>
  {/if}
{/if}
