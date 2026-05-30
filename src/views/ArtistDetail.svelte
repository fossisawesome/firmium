<script>
  import { IconMusic, IconList, IconPlay } from '../lib/icons.js'
  import { onMount, onDestroy } from 'svelte'
  import { queue, currentTrack, navToAlbum } from '../lib/stores.js'
  import { Api, Keyring, loadImage } from '../lib/api.js'
  import { playAt } from '../lib/playback.js'
  import { showPlaylistMenu } from '../lib/playlistMenu.js'
  import { lazyLoad } from '../lib/lazyLoad.js'
  import { pooledMap, SafeStorage } from '../lib/utils.js'
  import { PLAY_ALL_CONCURRENCY } from '../lib/api.js'

  // Fetches artist bio from Last.fm API using the user's own key.
  // Note: Last.fm removed artist images from their API in 2019; images come from the server instead.
  async function fetchLastfmBio(artistName, apiKey, signal) {
    try {
      const url = `https://ws.audioscrobbler.com/2.0/?method=artist.getInfo&artist=${encodeURIComponent(artistName)}&api_key=${encodeURIComponent(apiKey)}&format=json`
      const res = await fetch(url, { signal })
      const data = await res.json()
      const bio = data.artist?.bio?.summary || null
      return bio
    } catch { return null }
  }

let { id } = $props()

  let name = $state('')
  let groups = $state({ Albums: [], EPs: [], Singles: [] })
  let loading = $state(true)
  let error = $state('')
  let ctrl
  let bio = $state('Fetching biography…')
  let wikiImage = $state(null)
  let playingAll = $state(false)

  onMount(async () => {
    ctrl = new AbortController()
    try {
      const result = await Api.getArtistDetails(id, ctrl.signal)
      if (ctrl.signal.aborted) return
      name = result.name
      buildGroups(result.albums)
      const lastfmEnabled = SafeStorage.getItem('firmium_lastfm') === 'true'
      const lastfmKey = lastfmEnabled ? (await Keyring.load('lastfm_api_key').catch(() => '')) || '' : ''
      // Always fetch image from the server (Last.fm removed images from their API in 2019).
      Api.getArtistInfo(id, ctrl.signal).then(info => {
        if (ctrl.signal.aborted || !info?.image) return
        wikiImage = info.image
      })
      // Fetch bio: Last.fm client-side if configured, otherwise server's getArtistInfo2.
      const resolveBio = async () => {
        if (lastfmEnabled && lastfmKey) {
          const lfmBio = await fetchLastfmBio(name, lastfmKey, ctrl.signal)
          if (ctrl.signal.aborted) return
          if (lfmBio) { bio = lfmBio.replace(/<[^>]+>/g, '').trim(); return }
        } else {
          const serverInfo = await Api.getArtistInfo(id, ctrl.signal)
          if (ctrl.signal.aborted) return
          if (serverInfo?.bio) { bio = serverInfo.bio.replace(/<[^>]+>/g, '').trim(); return }
        }
        bio = 'Biography not available.'
      }
      resolveBio()
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
    <div class="artist-page-actions">
      <button class="play-all-btn" onclick={playAll} disabled={playingAll}>
        {#if !playingAll}<span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconPlay}</span>{/if}{playingAll ? 'Loading Queue…' : 'Play All Songs'}
      </button>
    </div>
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
              <div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconMusic}</span></div>
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
