<script lang="ts">
  import { IconMusic, IconList, IconPlay } from '../lib/icons'
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { queue, queueIdx, currentTrack, navBack } from '../lib/stores'
  import { Api, loadImage } from '../lib/api'
  import { playAt } from '../lib/playback'
  import { showPlaylistMenu } from '../lib/playlistMenu'
  import { lazyLoad } from '../lib/lazyLoad'
  import { formatDuration, createAbortController } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import type { Song } from '../lib/types/tauri-commands'

  const TRACK_ROW_HEIGHT = 56

  let { id }: { id: string } = $props()

  let tracks = $state<Song[]>([])
  let albumName = $state('')
  let albumArtist = $state('')
  let coverArtId = $state<string | undefined>(undefined)
  let loading = $state(true)
  let error = $state('')
  const abortCtrl = createAbortController()
  let coverImg = $state<HTMLImageElement>()

  const isCurrentTrackHere = $derived($currentTrack && tracks.some(t => t.id === $currentTrack!.id))

  onMount(async () => {
    const signal = abortCtrl.renew()
    try {
      const result = await Api.getAlbumTracks(id, signal)
      if (signal.aborted) return
      tracks = result.tracks
      albumName = result.albumName
      albumArtist = result.albumArtist
      coverArtId = result.coverArtId
    } catch (e: any) {
      if (!signal.aborted) error = e.message
    } finally {
      if (!signal.aborted) loading = false
    }
  })

  function playTrack(idx: number) {
    queue.set(tracks)
    playAt(idx)
  }

  function isPlaying(track: Song) {
    return $currentTrack?.id === track.id
  }
</script>

<div class="tracklist-header">
  <div class="tl-art">
    {#if coverArtId}
      <img bind:this={coverImg} use:lazyLoad={img => loadImage(img, coverArtId, abortCtrl.signal)} alt="" />
    {:else}
      <span class="icon" style="width:32px;height:32px;color:var(--muted)">{@html IconMusic}</span>
    {/if}
  </div>
  <div class="tl-info">
    <div class="tl-title">{albumName}</div>
    <div class="tl-subtitle">{albumArtist}</div>
  </div>

</div>

<LoadingState {loading} {error} empty={tracks.length === 0} loadingMessage="Loading album tracks…" emptyMessage="No tracks found.">
  <div class="track-list">
    <VirtualList items={tracks} itemHeight={TRACK_ROW_HEIGHT}>
      {#snippet children(track, idx)}
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
      {/snippet}
    </VirtualList>
  </div>
</LoadingState>
