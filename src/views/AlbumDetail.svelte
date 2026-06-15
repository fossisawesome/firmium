<script lang="ts">
  import { IconMusic } from '../lib/icons'
  import { currentTrack } from '../lib/stores'
  import { loadImage, Api } from '../lib/api'
  import { dataSource } from '../lib/dataSource'
  import { dataSourceVersion } from '../lib/stores'
  import { setQueueSeamless } from '../lib/playback'
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
    setQueueSeamless(tracks, idx)
  }

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
  </div>

</div>

<LoadingState {loading} {error} empty={tracks.length === 0} loadingMessage="Loading album tracks…" emptyMessage="No tracks found.">
  <div class="track-list">
    <VirtualList items={tracks} itemHeight={TRACK_ROW_HEIGHT}>
      {#snippet children(track, idx)}
        <TrackRow
          {track} {idx}
          playing={isPlaying(track)}
          signal={abortCtrl.signal}
          {albumArtist}
          downloaded={isDownloaded(track)}
          onPlay={playTrack}
        />
      {/snippet}
    </VirtualList>
  </div>
</LoadingState>
