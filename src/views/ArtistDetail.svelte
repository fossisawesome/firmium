<script lang="ts">
  import { IconPlay, IconLoading, IconShuffle } from '../lib/icons'
  import LoadingState from '../components/LoadingState.svelte'
  import { Keyring, Api } from '../lib/api'
  import { dataSource } from '../lib/dataSource'
  import { dataSourceVersion, navToArtist, isAuthed } from '../lib/stores'
  import { showPlaylistMenu } from '../lib/playlistMenu'
  import { tauriInvoke } from '../lib/tauri'
  import { pooledMap, SafeStorage, createAbortController } from '../lib/utils'
  import { PLAY_ALL_CONCURRENCY } from '../lib/api'
  import { tauriFetch } from '../lib/tauri'
  import VirtualList from '../lib/VirtualList.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
  import type { Album, Artist } from '../lib/types/tauri-commands'

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

  // Fallback similar-artist names from Last.fm directly (artist.getSimilar),
  // used when the server's getArtistInfo2 returns no similar artists.
  async function fetchLastfmSimilar(artistName: string, apiKey: string, signal?: AbortSignal | null): Promise<string[]> {
    try {
      const url = `https://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist=${encodeURIComponent(artistName)}&api_key=${encodeURIComponent(apiKey)}&format=json&limit=40`
      const res = await tauriFetch(url, { signal: signal ?? undefined })
      const data = await res.json()
      const arr = data.similarartists?.artist ?? []
      return (Array.isArray(arr) ? arr : [arr]).map((a: { name?: string }) => a?.name ?? '').filter(Boolean)
    } catch { return [] }
  }

  // Resolves similar-artist names (server first, Last.fm fallback) and keeps only
  // those the user actually has in their library, so every suggestion is playable.
  async function resolveRecommendations(artistId: string, artistName: string, signal: AbortSignal): Promise<void> {
    recommendations = []
    if (!$isAuthed) return
    try {
      let names = await Api.getSimilarArtists(artistId).catch(() => [] as string[])
      if (!names.length) {
        const lastfmEnabled = SafeStorage.getItem('firmium_lastfm') === 'true'
        const lastfmKey = lastfmEnabled ? ((await Keyring.load('lastfm_api_key').catch(() => '')) as string) || '' : ''
        if (lastfmKey) names = await fetchLastfmSimilar(artistName, lastfmKey, signal)
      }
      if (signal.aborted || !names.length) return
      const library: Artist[] = await Api.getArtists(signal).catch(() => [])
      const byName = new Map(library.map(a => [a.name.toLowerCase(), a]))
      const seen = new Set<string>([artistId])
      const matched: Artist[] = []
      for (const n of names) {
        const hit = byName.get(n.toLowerCase())
        if (hit && !seen.has(hit.id)) { seen.add(hit.id); matched.push(hit) }
      }
      if (!signal.aborted) recommendations = matched.slice(0, 12)
    } catch { /* recommendations are best-effort */ }
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
  let shufflingAll = $state(false)
  let recommendations = $state<Artist[]>([])

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
        resolveRecommendations(artistId, name, signal)
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

  async function playAll(shuffle = false) {
    if (shuffle) shufflingAll = true
    else playingAll = true
    try {
      const allAlbums = [...groups.Albums, ...groups.EPs, ...groups.Singles]
      const completed = await pooledMap(allAlbums, PLAY_ALL_CONCURRENCY, a => $dataSource.getAlbumTracks(a.id))
      const allTracks = completed.flatMap(r => r.tracks)
      if (allTracks.length > 0) {
        if (shuffle) tauriInvoke('shuffle_and_play', { songs: allTracks }).catch(console.error)
        else tauriInvoke('set_queue_seamless', { songs: allTracks, startIdx: 0 }).catch(console.error)
      } else alert('No playable tracks found for this artist.')
    } catch (err) {
      console.error('Play artist all failed:', err)
      alert('Failed to load artist queue.')
    } finally {
      playingAll = false
      shufflingAll = false
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
      <button class="play-all-btn" onclick={() => playAll(false)} disabled={playingAll || shufflingAll}>
        <span class="icon" class:icon-spin={playingAll} style="width:12px;height:12px;margin-right:6px">{@html playingAll ? IconLoading : IconPlay}</span>{playingAll ? 'Loading Queue…' : 'Play All Songs'}
      </button>
      <button class="play-all-btn shuffle-all-btn" onclick={() => playAll(true)} disabled={playingAll || shufflingAll}>
        <span class="icon" class:icon-spin={shufflingAll} style="width:12px;height:12px;margin-right:6px">{@html shufflingAll ? IconLoading : IconShuffle}</span>{shufflingAll ? 'Loading Queue…' : 'Shuffle'}
      </button>
      <button
        class="play-all-btn artist-add-btn"
        title="More options"
        aria-label="More options"
        onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'artist', artistId: id, artistName: name }) }}
      >+</button>
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

  {#if recommendations.length > 0}
    <div class="release-group-title">You might also like</div>
    <div class="reco-row">
      {#each recommendations as artist (artist.id)}
        <button class="reco-card" onclick={() => navToArtist(artist.id)} title={artist.name}>
          <img class="reco-img" src={DEFAULT_AVATAR} alt={artist.name} />
          <span class="reco-name">{artist.name}</span>
        </button>
      {/each}
    </div>
  {/if}
</LoadingState>

<style>
  .reco-row { display: flex; flex-wrap: wrap; gap: 16px; padding: 8px 0 24px; }
  .reco-card {
    display: flex; flex-direction: column; align-items: center; gap: 8px;
    width: 96px; background: none; border: none; cursor: pointer; color: var(--text);
  }
  .reco-img {
    width: 72px; height: 72px; border-radius: 50%; object-fit: cover;
    background: var(--surface2); border: 1px solid var(--border);
  }
  .reco-name {
    font-size: 13px; text-align: center; line-height: 1.3;
    overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  }
  .reco-card:hover .reco-name { color: var(--accent); }
</style>
