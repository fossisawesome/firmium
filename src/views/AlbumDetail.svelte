<script lang="ts">
  import { IconMusic, IconPlay, IconShuffle, IconHeart, IconHeartOutline, IconDownload } from '../lib/icons'
  import { currentTrack, isAuthed, dataVersion, toastError, toast } from '../lib/stores'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { errorMessage } from '../lib/ipc'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import TrackRow from '../components/TrackRow.svelte'
  import type { Song } from '../lib/types'

  const ROW_HEIGHT = 56
  let { id }: { id: string } = $props()

  let tracks = $state<Song[]>([])
  let albumName = $state('')
  let albumArtist = $state('')
  let coverArtId = $state<string | undefined>()
  let starred = $state(false)
  let loading = $state(true)
  let error = $state('')
  let downloadedKeys = $state(new Set<string>())

  const key = (n: number | null | undefined, title: string) => `${n ?? ''}|${title.trim().toLowerCase()}`

  $effect(() => {
    $dataVersion
    const albumId = id
    let live = true
    loading = true
    error = ''
    ;(async () => {
      try {
        const r = await Api.albumTracks(albumId)
        if (!live) return
        tracks = r.tracks; albumName = r.albumName; albumArtist = r.albumArtist; coverArtId = r.coverArtId; starred = r.starred
        const keys = await Api.localAlbumTrackKeys(r.albumArtist, r.albumName).catch(() => [])
        if (live) downloadedKeys = new Set(keys.map(k => key(k.trackNumber, k.title)))
      } catch (e) { if (live) error = errorMessage(e) }
      finally { if (live) loading = false }
    })()
    return () => { live = false }
  })

  const play = (idx: number) => Api.setQueueSeamless(tracks, idx).catch(toastError)
  const shuffle = () => tracks.length && Api.shuffleAndPlay(tracks).catch(toastError)

  async function rate(track: Song, rating: number) {
    await Api.setRating(track.id, rating).catch(toastError)
    tracks = tracks.map(t => (t.id === track.id ? { ...t, userRating: rating || undefined } : t))
  }

  async function toggleStar() {
    starred = !starred
    try { await Api.setStarred(id, 'album', starred) } catch (e) { starred = !starred; toastError(e) }
  }

  async function downloadAll() {
    try { await Api.downloadAlbum(id); toast('Album downloaded') } catch (e) { toastError(e) }
  }

  const BPM_RANGES = [
    { label: 'All', min: 0, max: Infinity }, { label: '<80', min: 0, max: 79 },
    { label: '80-120', min: 80, max: 120 }, { label: '120+', min: 121, max: Infinity },
  ]
  let bpm = $state(0)
  const hasBpm = $derived(tracks.some(t => t.bpm && t.bpm > 0))
  const filtered = $derived(bpm === 0 ? tracks : tracks.filter(t => t.bpm && t.bpm >= BPM_RANGES[bpm].min && t.bpm <= BPM_RANGES[bpm].max))
</script>

<div class="tracklist-header">
  <div class="tl-art">
    {#if coverArtId}<img use:cover={coverArtId} alt="{albumName} by {albumArtist}" />
    {:else}<span class="icon" style="width:32px;height:32px;color:var(--muted)">{@html IconMusic}</span>{/if}
  </div>
  <div class="tl-info">
    <div class="tl-title">{albumName}</div>
    <div class="tl-subtitle">{albumArtist}</div>
    <div class="pl-detail-actions">
      <button class="play-all-btn" onclick={() => play(0)} disabled={!tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconPlay}</span>Play All</button>
      <button class="play-all-btn shuffle-all-btn" onclick={shuffle} disabled={!tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconShuffle}</span>Shuffle</button>
      {#if $isAuthed}
        <button class="play-all-btn shuffle-all-btn" onclick={toggleStar} aria-label={starred ? 'Remove from favorites' : 'Add to favorites'}>
          <span class="icon" style="width:12px;height:12px">{@html starred ? IconHeart : IconHeartOutline}</span>
        </button>
        <button class="play-all-btn shuffle-all-btn" onclick={downloadAll} aria-label="Download album">
          <span class="icon" style="width:12px;height:12px">{@html IconDownload}</span>
        </button>
      {/if}
    </div>
  </div>
</div>

<LoadingState {loading} {error} empty={tracks.length === 0} loadingMessage="Loading album tracks…" emptyMessage="No tracks found.">
  {#if hasBpm}
    <div class="filter-bar"><div class="filter-group">
      {#each BPM_RANGES as r, i}<button class="filter-chip" class:active={bpm === i} onclick={() => (bpm = i)}>BPM {r.label}</button>{/each}
    </div></div>
  {/if}
  <div class="track-list">
    <VirtualList items={filtered} itemHeight={ROW_HEIGHT}>
      {#snippet children(track, idx)}
        <TrackRow {track} {idx} playing={$currentTrack?.id === track.id} {albumArtist}
          downloaded={downloadedKeys.has(key(track.trackNumber, track.title))}
          onPlay={() => play(tracks.indexOf(track))} onRate={$isAuthed ? rate : undefined} />
      {/snippet}
    </VirtualList>
  </div>
</LoadingState>
