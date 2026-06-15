<script lang="ts">
  import { IconPlay, IconLoading } from '../lib/icons'
  import LoadingState from '../components/LoadingState.svelte'
  import { Keyring } from '../lib/api'
  import { dataSource } from '../lib/dataSource'
  import { dataSourceVersion } from '../lib/stores'
  import { setQueueSeamless } from '../lib/playback'
  import { pooledMap, SafeStorage, createAbortController } from '../lib/utils'
  import { PLAY_ALL_CONCURRENCY } from '../lib/api'
  import { tauriFetch } from '../lib/tauri'
  import VirtualList from '../lib/VirtualList.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
  import type { Album } from '../lib/types/tauri-commands'

  const ALBUM_ROW_HEIGHT = 60

  interface ReleaseGroups {
    Albums: Album[]
    EPs: Album[]
    Singles: Album[]
  }

  // Fetches artist bio from Last.fm API using the user's own key.
  // Note: Last.fm removed artist images from their API in 2019; images come from the server instead.
  async function fetchLastfmBio(artistName: string, apiKey: string, signal?: AbortSignal | null): Promise<string | null> {
    try {
      const url = `https://ws.audioscrobbler.com/2.0/?method=artist.getInfo&artist=${encodeURIComponent(artistName)}&api_key=${encodeURIComponent(apiKey)}&format=json`
      const res = await tauriFetch(url, { signal: signal ?? undefined })
      const data = await res.json()
      const bio = data.artist?.bio?.summary || null
      return bio
    } catch { return null }
  }

let { id }: { id: string } = $props()

  let name = $state('')
  let groups = $state<ReleaseGroups>({ Albums: [], EPs: [], Singles: [] })
  let loading = $state(true)
  let error = $state('')
  const abortCtrl = createAbortController()
  let bio = $state('Fetching biography…')
  let wikiImage = $state<string | null>(null)
  let playingAll = $state(false)

  $effect(() => {
    const source = $dataSource
    $dataSourceVersion
    const artistId = id
    loading = true
    error = ''
    bio = 'Fetching biography…'
    wikiImage = null
    const signal = abortCtrl.renew()
    ;(async () => {
      try {
        const result = await source.getArtistDetails(artistId, signal)
        if (signal.aborted) return
        name = result.name
        buildGroups(result.albums)
        const lastfmEnabled = SafeStorage.getItem('firmium_lastfm') === 'true'
        const lastfmKey = lastfmEnabled ? ((await Keyring.load('lastfm_api_key').catch(() => '')) as string) || '' : ''
        // Always fetch image from the server (Last.fm removed images from their API in 2019).
        source.getArtistInfo(artistId, signal).then(info => {
          if (signal.aborted || !info?.image) return
          wikiImage = info.image
        })
        // Fetch bio: Last.fm client-side if configured, otherwise server's getArtistInfo2.
        const resolveBio = async () => {
          if (lastfmEnabled && lastfmKey) {
            const lfmBio = await fetchLastfmBio(name, lastfmKey, signal)
            if (signal.aborted) return
            if (lfmBio) { bio = lfmBio.replace(/<[^>]+>/g, '').trim(); return }
          } else {
            const serverInfo = await source.getArtistInfo(artistId, signal)
            if (signal.aborted) return
            if (serverInfo?.bio) { bio = serverInfo.bio.replace(/<[^>]+>/g, '').trim(); return }
          }
          bio = 'Biography not available.'
        }
        resolveBio()
      } catch (e: any) {
        if (!signal.aborted) error = e.message
      } finally {
        if (!signal.aborted) loading = false
      }
    })()
  })

  function buildGroups(albums: Album[]) {
    groups = { Albums: [], EPs: [], Singles: [] }
    albums.forEach(a => {
      const type = String(a.releaseType || '').toLowerCase()
      const titleLower = a.name.toLowerCase()
      if (type === 'single') groups.Singles.push(a)
      else if (type === 'ep') groups.EPs.push(a)
      else if (type === 'album') groups.Albums.push(a)
      else if (titleLower.includes(' - single') || titleLower.endsWith('(single)')) groups.Singles.push(a)
      else if (titleLower.includes(' - ep') || titleLower.endsWith('(ep)')) groups.EPs.push(a)
      else groups.Albums.push(a)
    })
  }

  async function playAll() {
    playingAll = true
    try {
      const allAlbums = [...groups.Albums, ...groups.EPs, ...groups.Singles]
      const completed = await pooledMap(allAlbums, PLAY_ALL_CONCURRENCY, a => $dataSource.getAlbumTracks(a.id))
      const allTracks = completed.flatMap(r => r.tracks)
      if (allTracks.length > 0) { setQueueSeamless(allTracks, 0) }
      else alert('No playable tracks found for this artist.')
    } catch (err) {
      console.error('Play artist all failed:', err)
      alert('Failed to load artist queue.')
    } finally {
      playingAll = false
    }
  }

  const DEFAULT_AVATAR = `data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23888' viewBox='0 0 24 24'><path d='M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z'/></svg>`
</script>

<div class="artist-page-header">
  <img
    class="artist-img-circle"
    src={wikiImage ?? DEFAULT_AVATAR}
    alt={name}
  />
  <div class="artist-page-info">
    <div class="artist-page-name">{name}</div>
    <div class="artist-page-bio">{bio}</div>
    <div class="artist-page-actions">
      <button class="play-all-btn" onclick={playAll} disabled={playingAll}>
        <span class="icon" class:icon-spin={playingAll} style="width:12px;height:12px;margin-right:6px">{@html playingAll ? IconLoading : IconPlay}</span>{playingAll ? 'Loading Queue…' : 'Play All Songs'}
      </button>
    </div>
  </div>
</div>


<LoadingState {loading} {error} empty={!groups.Albums.length && !groups.EPs.length && !groups.Singles.length} loadingMessage="Loading artist profile…" emptyMessage="No releases found.">
  {#each ['Albums', 'EPs', 'Singles'] as const as category}
    {#if groups[category].length > 0}
      <div class="release-group-title">{category}</div>
      <VirtualList items={groups[category]} itemHeight={ALBUM_ROW_HEIGHT}>
        {#snippet children(album, _index)}
          <AlbumRow {album} signal={abortCtrl.signal} />
        {/snippet}
      </VirtualList>
    {/if}
  {/each}
</LoadingState>
