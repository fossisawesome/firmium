<script>
  import { onMount, onDestroy } from 'svelte'
  import { queue, currentTrack, navToAlbum } from '../lib/stores.js'
  import { Api, loadImage } from '../lib/api.js'
  import { playAt } from '../lib/playback.js'
  import { showPlaylistMenu } from '../lib/playlistMenu.js'
  import { lazyLoad } from '../lib/lazyLoad.js'
  import { pooledMap, SafeStorage } from '../lib/utils.js'
  import { PLAY_ALL_CONCURRENCY } from '../lib/api.js'

  const WikiApi = {
    getInfo: async (artistName, signal) => {
      try {
        const searchUrl = `https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch=${encodeURIComponent(artistName + ' music')}&utf8=&format=json&origin=*`
        const searchRes = await fetch(searchUrl, { signal })
        const searchData = await searchRes.json()
        const title = searchData.query?.search?.[0]?.title
        if (!title) return null
        const summaryUrl = `https://en.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(title)}`
        const summaryRes = await fetch(summaryUrl, { signal })
        const summaryData = await summaryRes.json()
        return { extract: summaryData.extract, image: summaryData.thumbnail?.source || null }
      } catch { return null }
    }
  }

  let { id } = $props()

  let name = $state('')
  let groups = $state({ Albums: [], EPs: [], Singles: [] })
  let loading = $state(true)
  let error = $state('')
  let ctrl
  let bio = $state(SafeStorage.getItem('firmium_wikipedia') !== 'false' ? 'Fetching artist biography…' : 'Biography disabled.')
  let wikiImage = $state(null)
  let playingAll = $state(false)

  onMount(async () => {
    ctrl = new AbortController()
    try {
      const result = await Api.getArtistDetails(id, ctrl.signal)
      if (ctrl.signal.aborted) return
      name = result.name
      buildGroups(result.albums)
      if (SafeStorage.getItem('firmium_wikipedia') !== 'false') {
        WikiApi.getInfo(name, ctrl.signal).then(wiki => {
          if (ctrl.signal.aborted) return
          if (wiki) { bio = wiki.extract ?? 'Biography not available.'; wikiImage = wiki.image }
          else bio = 'Biography not available.'
        })
      }
    } catch (e) {
      if (!ctrl.signal.aborted) error = e.message
    } finally {
      if (!ctrl.signal.aborted) loading = false
    }
  })

  onDestroy(() => ctrl?.abort())

  function buildGroups(albums) {
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
      const completed = await pooledMap(allAlbums, PLAY_ALL_CONCURRENCY, a => Api.getAlbumTracks(a.id))
      const allTracks = completed.flatMap(r => r.tracks)
      if (allTracks.length > 0) { queue.set(allTracks); playAt(0) }
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
    <button class="play-all-btn" onclick={playAll} disabled={playingAll}>
      {playingAll ? 'Loading Queue…' : '▶ Play All Songs'}
    </button>
  </div>
</div>

{#if loading}
  <div class="loading-msg">Loading artist profile…</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else}
  {#each ['Albums', 'EPs', 'Singles'] as category}
    {#if groups[category].length > 0}
      <div class="release-group-title">{category}</div>
      {#each groups[category] as album}
        <div
          class="album-row"
          role="button"
          tabindex="0"
          onclick={() => navToAlbum(album.id)}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToAlbum(album.id)}
        >
          <div class="album-art-sm">
            {#if album.coverArtId}
              <img use:lazyLoad={img => loadImage(img, album.coverArtId, ctrl?.signal)} alt="" />
            {:else}
              <div class="no-art">♪</div>
            {/if}
          </div>
          <div class="album-info">
            <div class="album-title">{album.name}</div>
            <div class="album-artist">{album.albumArtist}</div>
          </div>
          <button
            class="album-add-btn"
            title="Add album to playlist"
            onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'album', albumId: album.id, albumName: album.name }) }}
          >+</button>
        </div>
      {/each}
    {/if}
  {/each}
{/if}
