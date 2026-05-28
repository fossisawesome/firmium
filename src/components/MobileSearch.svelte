<script>
  import { onDestroy, onMount } from 'svelte'
  import { get } from 'svelte/store'
  import {
    mobileSearchOpen, savedSearchQuery, savedSearchSongs, savedSearchAlbums,
    queue, currentTrack, navToAlbum
  } from '../lib/stores.js'
  import { Api, loadImage } from '../lib/api.js'
  import { playAt } from '../lib/playback.js'
  import { showPlaylistMenu } from '../lib/playlistMenu.js'
  import { lazyLoad } from '../lib/lazyLoad.js'
  import { formatDuration } from '../lib/utils.js'
  import { IconBack, IconSearch, IconMusic } from '../lib/icons.js'

  let query = $state(get(savedSearchQuery))
  let songs = $state(get(savedSearchSongs))
  let albums = $state(get(savedSearchAlbums))
  let loading = $state(false)
  let error = $state('')
  let searched = $state(get(savedSearchSongs).length > 0 || get(savedSearchAlbums).length > 0)
  let ctrl
  let inputEl = $state()
  let closing = $state(false)

  onMount(() => {
    // Listen for genre-based searches triggered from HomeView
    const onGenreSearch = (e) => {
      query = e.detail
      executeSearch()
    }
    window.addEventListener('firmium:search-genre', onGenreSearch)
    // Autofocus after animation starts
    setTimeout(() => inputEl?.focus(), 100)
    return () => window.removeEventListener('firmium:search-genre', onGenreSearch)
  })

  onDestroy(() => ctrl?.abort())

  async function executeSearch() {
    if (!query.trim()) return
    ctrl?.abort()
    ctrl = new AbortController()
    loading = true
    error = ''
    searched = false
    try {
      const results = await Api.search(query.trim(), ctrl.signal)
      if (ctrl.signal.aborted) return
      songs = results.songs
      albums = results.albums
      searched = true
      savedSearchQuery.set(query)
      savedSearchSongs.set(results.songs)
      savedSearchAlbums.set(results.albums)
    } catch (e) {
      if (!ctrl.signal.aborted) error = e.message
    } finally {
      if (!ctrl.signal.aborted) loading = false
    }
  }

  function closeSearch() {
    // Blur keyboard first, then animate out
    inputEl?.blur()
    closing = true
    setTimeout(() => mobileSearchOpen.set(false), 280)
  }

  function playTrack(idx) {
    queue.set(songs)
    playAt(idx)
    closeSearch()
  }

  function isPlaying(track) {
    return $currentTrack?.id === track.id
  }
</script>

<div class="ms-overlay" class:ms-closing={closing}>
  <!-- Header: back arrow + search input -->
  <div class="ms-header">
    <button class="ms-back-btn" onclick={closeSearch} aria-label="Close search">
      <span class="icon" style="width:24px;height:24px">{@html IconBack}</span>
    </button>
    <div class="ms-input-wrap">
      <!-- svelte-ignore a11y_autofocus -->
      <input
        bind:this={inputEl}
        bind:value={query}
        class="ms-input"
        type="search"
        placeholder="Search albums, songs…"
        maxlength="500"
        onkeydown={e => e.key === 'Enter' && executeSearch()}
      />
    </div>
    <button class="ms-search-exec" onclick={executeSearch} aria-label="Search">
      <span class="icon" style="width:20px;height:20px">{@html IconSearch}</span>
    </button>
  </div>

  <!-- Results body -->
  <div class="ms-body">
    {#if loading}
      <div class="ms-status">Searching…</div>
    {:else if error}
      <div class="ms-status ms-error">{error}</div>
    {:else if searched && songs.length === 0 && albums.length === 0}
      <div class="ms-status">No results found.</div>
    {:else if searched}
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
                  <img use:lazyLoad={img => loadImage(img, track.coverArtId, ctrl?.signal)} alt="" />
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
            onclick={() => { navToAlbum(album.id); closeSearch() }}
            onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToAlbum(album.id)}
          >
            <div class="album-art-sm">
              {#if album.coverArtId}
                <img use:lazyLoad={img => loadImage(img, album.coverArtId, ctrl?.signal)} alt="" />
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
    {:else}
      <div class="ms-hint">Type above and press Search or Enter</div>
    {/if}
  </div>
</div>
