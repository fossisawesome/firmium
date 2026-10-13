<script lang="ts">
  import { IconDownload, IconLoading, IconChevronUp, IconChevronDown, IconStarFilled, IconStarEmpty, IconHeart, IconHeartOutline } from '../lib/icons'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { showMenu } from '../lib/menu.svelte'
  import { app, toastError } from '../lib/stores.svelte'
  import { formatDuration } from '../lib/utils'
  import type { Song } from '../lib/types'

  let {
    track, idx, playing = false, albumArtist, displayNum, showAddButton = true, downloaded = false,
    onPlay, onRate, onRemove, onMove, isFirst = false, isLast = false,
  }: {
    track: Song
    idx: number
    playing?: boolean
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

  const serverTrack = $derived(!track.id.startsWith('local:') && !track.id.startsWith('podcast:'))

  let dl = $state<'idle' | 'loading' | 'done' | 'error'>('idle')
  $effect(() => { if (downloaded && dl === 'idle') dl = 'done' })

  async function download(e: MouseEvent) {
    e.stopPropagation()
    if (dl === 'loading') return
    dl = 'loading'
    try { await Api.downloadTrack(track, albumArtist); dl = 'done' }
    catch (err) { toastError(err); dl = 'error' }
    finally { setTimeout(() => { if (dl !== 'done') dl = 'idle' }, 2000) }
  }

  let starred = $state(false)
  $effect(() => { starred = track.starred })
  async function toggleStar(e: MouseEvent) {
    e.stopPropagation()
    starred = !starred
    try { await Api.setStarred(track.id, 'song', starred) } catch (err) { starred = !starred; toastError(err) }
  }
</script>

<div class={["track-row", { playing }]}
  role="button"
  tabindex="0"
  onclick={() => onPlay(idx)}
  onkeydown={e => (e.key === 'Enter' || e.key === ' ') && onPlay(idx)}>
  <div class="track-num">{displayNum ?? track.trackNumber ?? idx + 1}</div>
  <div class="track-thumb">{#if track.coverArtId}<img {@attach cover(track.coverArtId)} alt="" />{/if}</div>
  <div class="track-info">
    <div class="track-title">{track.title}</div>
    <div class="track-artist">{track.artist}</div>
  </div>
  <div class="track-duration">{formatDuration(track.duration)}</div>
  {#if onRate}
    <div class={["track-stars", { 'has-rating': track.userRating && track.userRating > 0 }]}>
      {#each [1, 2, 3, 4, 5] as star}
        <button class="star-btn" title="Rate {star}" onclick={e => { e.stopPropagation(); onRate!(track, track.userRating === star ? 0 : star) }}>
          <span class="icon" style="width:12px;height:12px">{@html star <= (track.userRating ?? 0) ? IconStarFilled : IconStarEmpty}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#if app.isAuthed && serverTrack}
    <button class={["track-heart-btn", { starred }]} title={starred ? 'Remove from favorites' : 'Add to favorites'} onclick={toggleStar}>
      <span class="icon" style="width:13px;height:13px">{@html starred ? IconHeart : IconHeartOutline}</span>
    </button>
    <button class={["track-download-btn", { 'download-done': dl === 'done', 'download-error': dl === 'error' }]}
      title="Download track"
      disabled={dl === 'loading'}
      onclick={download}>
      <span class="icon" style="width:13px;height:13px">{@html dl === 'loading' ? IconLoading : IconDownload}</span>
    </button>
  {/if}
  {#if showAddButton}
    <button class="track-add-btn" title="More" onclick={e => { e.stopPropagation(); showMenu(e.currentTarget, { type: 'tracks', tracks: [track] }) }}>+</button>
  {/if}
  {#if onMove}
    <button class="track-move-btn" title="Move up" disabled={isFirst} onclick={e => { e.stopPropagation(); onMove!(idx, -1) }}>
      <span class="icon" style="width:13px;height:13px">{@html IconChevronUp}</span>
    </button>
    <button class="track-move-btn" title="Move down" disabled={isLast} onclick={e => { e.stopPropagation(); onMove!(idx, 1) }}>
      <span class="icon" style="width:13px;height:13px">{@html IconChevronDown}</span>
    </button>
  {/if}
  {#if onRemove}
    <button class="track-remove-btn" title="Remove from playlist" onclick={e => { e.stopPropagation(); onRemove!(track, idx) }}>×</button>
  {/if}
</div>
