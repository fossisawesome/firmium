<script>
  import { IconMusic, IconList, IconPlay } from '../lib/icons.js'
  import { onMount, onDestroy } from 'svelte'
  import { get } from 'svelte/store'
  import { queue, queueIdx, currentTrack, navBack } from '../lib/stores.js'
  import { Api, loadImage } from '../lib/api.js'
  import { playAt } from '../lib/playback.js'
  import { showPlaylistMenu } from '../lib/playlistMenu.js'
  import { lazyLoad } from '../lib/lazyLoad.js'
  import { formatDuration } from '../lib/utils.js'

  let { id } = $props()

  let tracks = $state([])
  let albumName = $state('')
  let albumArtist = $state('')
  let coverArtId = $state(null)
  let loading = $state(true)
  let error = $state('')
  let ctrl
  let coverImg = $state()

  const isCurrentTrackHere = $derived($currentTrack && tracks.some(t => t.id === $currentTrack.id))

  onMount(async () => {
    ctrl = new AbortController()
    try {
      const result = await Api.getAlbumTracks(id, ctrl.signal)
      if (ctrl.signal.aborted) return
      tracks = result.tracks
      albumName = result.albumName
      albumArtist = result.albumArtist
      coverArtId = result.coverArtId
    } catch (e) {
      if (!ctrl.signal.aborted) error = e.message
    } finally {
      if (!ctrl.signal.aborted) loading = false
    }
  })

  onDestroy(() => ctrl?.abort())

  function playTrack(idx) {
    queue.set(tracks)
    playAt(idx)
  }

  function isPlaying(track) {
    return $currentTrack?.id === track.id
  }
</script>

<div class="tracklist-header">
  <div class="tl-art">
    {#if coverArtId}
      <img bind:this={coverImg} use:lazyLoad={img => loadImage(img, coverArtId, ctrl?.signal)} alt="" />
    {:else}
      <span class="icon" style="width:32px;height:32px;color:var(--muted)">{@html IconMusic}</span>
    {/if}
  </div>
  <div class="tl-info">
    <div class="tl-title">{albumName}</div>
    <div class="tl-subtitle">{albumArtist}</div>
  </div>
</div>

{#if loading}
  <div class="loading-msg">Loading album tracks…</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else}
  <div class="track-list">
    {#each tracks as track, idx}
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
