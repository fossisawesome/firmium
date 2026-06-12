<script lang="ts">
  import { IconMusic, IconList, IconPlay } from '../lib/icons'
  import { queue, currentTrack, navToAlbum } from '../lib/stores'
  import { Api, loadImage } from '../lib/api'
  import { playAt } from '../lib/playback'
  import { showPlaylistMenu } from '../lib/playlistMenu'
  import { lazyLoad } from '../lib/lazyLoad'
  import { formatDuration, createAbortController } from '../lib/utils'
  import type { Song, Album } from '../lib/types/tauri-commands'

  const SEARCH_INPUT_MAX_LENGTH = 500

  let query = $state('')
  let songs = $state<Song[]>([])
  let albums = $state<Album[]>([])
  let loading = $state(false)
  let error = $state('')
  let searched = $state(false)
  const abortCtrl = createAbortController()

  async function executeSearch() {
    if (!query.trim()) return
    const signal = abortCtrl.renew()
    loading = true
    error = ''
    searched = false
    try {
      const results = await Api.search(query.trim(), signal)
      if (signal.aborted) return
      songs = results.songs
      albums = results.albums
      searched = true
    } catch (e: any) {
      if (!signal.aborted) error = e.message
    } finally {
      if (!signal.aborted) loading = false
    }
  }

  function playTrack(idx: number) {
    queue.set(songs)
    playAt(idx)
  }

  function isPlaying(track: Song) {
    return $currentTrack?.id === track.id
  }
</script>

<div class="search-row">
  <!-- svelte-ignore a11y_autofocus -->
  <input
    type="text"
    placeholder="Search albums, songs…"
    maxlength={SEARCH_INPUT_MAX_LENGTH}
    bind:value={query}
    onkeydown={e => e.key === 'Enter' && executeSearch()}
    autofocus
  />
  <button onclick={executeSearch}>Search</button>
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
        <div class="track-list">
          {#each songs as track, idx}
            <div
              class="track-row"
              class:playing={isPlaying(track)}
              role="button"
              tabindex="0"
              onclick={() => playTrack(idx)}
              onkeydown={e => (e.key === 'Enter' || e.key === ' ') && playTrack(idx)}
            >
              <div class="track-num">{track.trackNumber ?? idx + 1}</div>
              <div class="track-thumb">
                {#if track.coverArtId}
                  <img use:lazyLoad={img => loadImage(img, track.coverArtId, abortCtrl.signal)} alt="" />
                {/if}
              </div>
              <div class="track-info">
                <div class="track-title">{track.title}</div>
                <div class="track-artist">{track.artist}</div>
              </div>
              <div class="track-duration">{formatDuration(track.duration)}</div>
              <button
                class="track-add-btn"
                title="Add to playlist"
                onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'tracks', tracks: [track] }) }}
              >+</button>
            </div>
          {/each}
        </div>
      {/if}

      {#if albums.length > 0}
        <div class="section-header">Albums</div>
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
                <img use:lazyLoad={img => loadImage(img, album.coverArtId, abortCtrl.signal)} alt="" />
              {:else}
                <div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconMusic}</span></div>
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
    </div>
  {/if}
{/if}
