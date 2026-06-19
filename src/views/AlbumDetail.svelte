<script lang="ts">
  import { IconMusic, IconPlay, IconShuffle } from '../lib/icons'
  import { currentTrack, isAuthed } from '../lib/stores'
  import { loadImage, Api } from '../lib/api'
  import { dataSource } from '../lib/dataSource'
  import { dataSourceVersion } from '../lib/stores'
  import { tauriInvoke } from '../lib/tauri'
  import { lazyLoad } from '../lib/lazyLoad'
  import { createAbortController } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import TrackRow from '../components/TrackRow.svelte'
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

  let downloadedKeys = $state<Set<string>>(new Set())

  function trackKey(trackNumber: number | null | undefined, title: string): string {
    return `${trackNumber ?? ''}|${title.trim().toLowerCase()}`
  }

  function isDownloaded(track: Song): boolean {
    return downloadedKeys.has(trackKey(track.trackNumber, track.title))
  }

  $effect(() => {
    const source = $dataSource
    $dataSourceVersion
    const albumId = id
    loading = true
    error = ''
    const signal = abortCtrl.renew()
    ;(async () => {
      try {
        const result = await source.getAlbumTracks(albumId, signal)
        if (signal.aborted) return
        tracks = result.tracks
        albumName = result.albumName
        albumArtist = result.albumArtist
        coverArtId = result.coverArtId

        try {
          const localKeys = await Api.getLocalAlbumTrackKeys(albumArtist, albumName)
          if (signal.aborted) return
          downloadedKeys = new Set(localKeys.map(k => trackKey(k.trackNumber, k.title)))
        } catch (_) {
          downloadedKeys = new Set()
        }
      } catch (e: any) {
        if (!signal.aborted) error = e.message
      } finally {
        if (!signal.aborted) loading = false
      }
    })()
  })

  function playTrack(idx: number) {
    tauriInvoke('set_queue_seamless', { songs: tracks, startIdx: idx }).catch(console.error)
  }

  function playAll() {
    if (!tracks.length) return
    tauriInvoke('set_queue_seamless', { songs: tracks, startIdx: 0 }).catch(console.error)
  }

  function shuffleAll() {
    if (!tracks.length) return
    tauriInvoke('shuffle_and_play', { songs: tracks }).catch(console.error)
  }

  function rateTrack(track: Song, rating: number) {
    Api.setRating(track.id, rating)
    const idx = tracks.findIndex(t => t.id === track.id)
    if (idx >= 0) tracks[idx] = { ...tracks[idx], userRating: rating || undefined }
  }

  const BPM_RANGES = [
    { label: 'All', min: 0, max: Infinity },
    { label: '<80', min: 0, max: 79 },
    { label: '80-120', min: 80, max: 120 },
    { label: '120+', min: 121, max: Infinity },
  ] as const

  let selectedBpm = $state(0)

  const hasBpmData = $derived(tracks.some(t => t.bpm && t.bpm > 0))

  const filteredTracks = $derived.by(() => {
    if (selectedBpm === 0) return tracks
    const range = BPM_RANGES[selectedBpm]
    return tracks.filter(t => t.bpm && t.bpm >= range.min && t.bpm <= range.max)
  })

  function isPlaying(track: Song) {
    return $currentTrack?.id === track.id
  }
</script>

<div class="tracklist-header">
  <div class="tl-art">
    {#if coverArtId}
      <img bind:this={coverImg} use:lazyLoad={img => loadImage(img, coverArtId, abortCtrl.signal)} alt="{albumName} by {albumArtist}" />
    {:else}
      <span class="icon" style="width:32px;height:32px;color:var(--muted)">{@html IconMusic}</span>
    {/if}
  </div>
  <div class="tl-info">
    <div class="tl-title">{albumName}</div>
    <div class="tl-subtitle">{albumArtist}</div>
    <div class="pl-detail-actions">
      <button class="play-all-btn" onclick={playAll} disabled={!tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconPlay}</span>Play All</button>
      <button class="play-all-btn shuffle-all-btn" onclick={shuffleAll} disabled={!tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconShuffle}</span>Shuffle</button>
    </div>
  </div>

</div>

<LoadingState {loading} {error} empty={tracks.length === 0} loadingMessage="Loading album tracks…" emptyMessage="No tracks found.">
  {#if hasBpmData}
    <div class="filter-bar">
      <div class="filter-group">
        {#each BPM_RANGES as range, i}
          <button
            class="filter-chip"
            class:active={selectedBpm === i}
            onclick={() => selectedBpm = i}
          >BPM {range.label}</button>
        {/each}
      </div>
    </div>
  {/if}
  <div class="track-list">
    <VirtualList items={filteredTracks} itemHeight={TRACK_ROW_HEIGHT}>
      {#snippet children(track, idx)}
        <TrackRow
          {track} {idx}
          playing={isPlaying(track)}
          signal={abortCtrl.signal}
          {albumArtist}
          downloaded={isDownloaded(track)}
          onPlay={playTrack}
          onRate={$isAuthed ? rateTrack : undefined}
        />
      {/snippet}
    </VirtualList>
  </div>
</LoadingState>
