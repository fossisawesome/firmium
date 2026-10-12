<script lang="ts">
  import { IconPlay, IconShuffle } from '../lib/icons'
  import { currentTrack, dataVersion, toastError } from '../lib/stores'
  import { Api } from '../lib/api'
  import { errorMessage } from '../lib/ipc'
  import LoadingState from '../components/LoadingState.svelte'
  import TrackRow from '../components/TrackRow.svelte'
  import type { Song } from '../lib/types'

  let { name }: { name: string } = $props()

  let songs = $state<Song[]>([])
  let loading = $state(true)
  let error = $state('')

  $effect(() => {
    $dataVersion
    const genre = name
    let live = true
    loading = true
    error = ''
    Api.songsByGenre(genre, 200)
      .then(r => { if (live) songs = r })
      .catch(e => { if (live) error = errorMessage(e) })
      .finally(() => { if (live) loading = false })
    return () => { live = false }
  })

  const play = (idx: number) => Api.setQueueSeamless(songs, idx).catch(toastError)
</script>

<div class="section-header">{name}</div>

<LoadingState {loading} {error} empty={songs.length === 0} loadingMessage="Loading songs…" emptyMessage="No songs found for this genre.">
  <div class="pl-detail-actions" style="margin: 0 0 12px;">
    <button class="play-all-btn" onclick={() => play(0)}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconPlay}</span>Play All</button>
    <button class="play-all-btn shuffle-all-btn" onclick={() => Api.shuffleAndPlay(songs).catch(toastError)}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconShuffle}</span>Shuffle</button>
  </div>
  <div class="track-list">
    {#each songs as track, idx (track.id)}
      <TrackRow {track} {idx} playing={$currentTrack?.id === track.id} onPlay={play} />
    {/each}
  </div>
</LoadingState>
