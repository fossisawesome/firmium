<script lang="ts">
  import { IconDownload, IconLoading, IconChevronUp, IconChevronDown, IconStarFilled, IconStarEmpty } from '../lib/icons'
  import { loadImage, Api } from '../lib/api'
  import { showPlaylistMenu } from '../lib/playlistMenu'
  import { lazyLoad } from '../lib/lazyLoad'
  import { downloadFormat, isAuthed } from '../lib/stores'
  import { formatDuration } from '../lib/utils'
  import type { Song } from '../lib/types/tauri-commands'

  let {
    track, idx, playing = false, signal = null, albumArtist, displayNum, showAddButton = true, downloaded = false, onPlay, onRate, onRemove, onMove, isFirst = false, isLast = false,
  }: {
    track: Song
    idx: number
    playing?: boolean
    signal?: AbortSignal | null
    albumArtist?: string
    displayNum?: number
    showAddButton?: boolean
    downloaded?: boolean
    onPlay: (idx: number) => void
    onRate?: (track: Song, rating: number) => void
    onRemove?: (track: Song, idx: number) => void
    onMove?: (idx: number, direction: -1 | 1) => void
    isFirst?: boolean
    isLast?: boolean
  } = $props()

  let downloadState = $state<'idle' | 'loading' | 'done' | 'error'>(downloaded ? 'done' : 'idle')

  $effect(() => {
    if (downloaded && downloadState === 'idle') downloadState = 'done'
  })

  async function download(e: MouseEvent) {
    e.stopPropagation()
    if (downloadState === 'loading') return
    downloadState = 'loading'
    try {
      await Api.downloadTrack(track, $downloadFormat, albumArtist)
      downloadState = 'done'
    } catch (err) {
      console.error('Track download failed:', err)
      downloadState = 'error'
    } finally {
      setTimeout(() => { downloadState = 'idle' }, 2000)
    }
  }
</script>

<div
  class="track-row"
  class:playing={playing}
  role="button"
  tabindex="0"
  onclick={() => onPlay(idx)}
  onkeydown={e => (e.key === 'Enter' || e.key === ' ') && onPlay(idx)}
>
  <div class="track-num">{displayNum ?? track.trackNumber ?? idx + 1}</div>
  <div class="track-thumb">
    {#if track.coverArtId}
      <img use:lazyLoad={img => loadImage(img, track.coverArtId, signal)} alt="" />
    {/if}
  </div>
  <div class="track-info">
    <div class="track-title">{track.title}</div>
    <div class="track-artist">{track.artist}</div>
  </div>
  <div class="track-duration">{formatDuration(track.duration)}</div>
  {#if onRate}
    <div class="track-stars" class:has-rating={track.userRating && track.userRating > 0}>
      {#each [1, 2, 3, 4, 5] as star}
        <button
          class="star-btn"
          title="Rate {star}"
          onclick={e => { e.stopPropagation(); onRate!(track, track.userRating === star ? 0 : star) }}
        ><span class="icon" style="width:12px;height:12px">{@html star <= (track.userRating ?? 0) ? IconStarFilled : IconStarEmpty}</span></button>
      {/each}
    </div>
  {/if}
  {#if $isAuthed}
    <button
      class="track-download-btn"
      class:download-done={downloadState === 'done'}
      class:download-error={downloadState === 'error'}
      title="Download track"
      disabled={downloadState === 'loading'}
      onclick={download}
    >
      <span class="icon" style="width:13px;height:13px">{@html downloadState === 'loading' ? IconLoading : IconDownload}</span>
    </button>
  {/if}
  {#if showAddButton}
    <button
      class="track-add-btn"
      title="Add to playlist"
      onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'tracks', tracks: [track] }) }}
    >+</button>
  {/if}
  {#if onMove}
    <button
      class="track-move-btn"
      title="Move up"
      disabled={isFirst}
      onclick={e => { e.stopPropagation(); onMove!(idx, -1) }}
    ><span class="icon" style="width:13px;height:13px">{@html IconChevronUp}</span></button>
    <button
      class="track-move-btn"
      title="Move down"
      disabled={isLast}
      onclick={e => { e.stopPropagation(); onMove!(idx, 1) }}
    ><span class="icon" style="width:13px;height:13px">{@html IconChevronDown}</span></button>
  {/if}
  {#if onRemove}
    <button
      class="track-remove-btn"
      title="Remove from playlist"
      onclick={e => { e.stopPropagation(); onRemove!(track, idx) }}
    >×</button>
  {/if}
</div>
