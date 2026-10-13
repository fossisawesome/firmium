<script lang="ts">
  import { app, navTo, toastError } from '../lib/stores.svelte'
  import { Api } from '../lib/api'
  import { cover, artistPhoto } from '../lib/cover'
  import { errorMessage } from '../lib/ipc'
  import LoadingState from '../components/LoadingState.svelte'
  import TrackRow from '../components/TrackRow.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
  import type { Starred } from '../lib/types'

  let data = $state<Starred>({ artists: [], albums: [], songs: [] })
  let loading = $state(true)
  let error = $state('')
  const empty = $derived(!data.artists.length && !data.albums.length && !data.songs.length)

  $effect(() => {
    app.dataVersion
    let live = true
    loading = true
    error = ''
    Api.starred()
      .then(r => { if (live) data = r })
      .catch(e => { if (live) error = errorMessage(e) })
      .finally(() => { if (live) loading = false })
    return () => { live = false }
  })

  const play = (idx: number) => Api.setQueueSeamless(data.songs, idx).catch(toastError)
</script>

<div class="section-header">Favorites</div>

<LoadingState {loading} {error} {empty} loadingMessage="Loading favorites…" emptyMessage="Nothing favorited yet. Tap the heart on a song, album or artist.">
  {#if data.songs.length > 0}
    <div class="release-group-title">Songs</div>
    <div class="track-list">
      {#each data.songs as track, idx (track.id)}
        <TrackRow {track} {idx} playing={app.currentTrack?.id === track.id} onPlay={play} />
      {/each}
    </div>
  {/if}
  {#if data.albums.length > 0}
    <div class="release-group-title">Albums</div>
    {#each data.albums as album (album.id)}<AlbumRow {album} />{/each}
  {/if}
  {#if data.artists.length > 0}
    <div class="release-group-title">Artists</div>
    <div class="artist-list">
      {#each data.artists as artist (artist.id)}
        <div class="artist-row" role="button" tabindex="0" onclick={() => navTo('artist', artist.id)}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navTo('artist', artist.id)}>
          <img class="artist-row-avatar" {@attach artistPhoto(artist.id)} alt="" />
          <div class="artist-info">
            <div class="artist-name">{artist.name}</div>
            <div class="artist-album-count">{artist.albumCount} albums</div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</LoadingState>
