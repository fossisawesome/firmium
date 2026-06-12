<script lang="ts">
  import { IconMusic, IconList, IconPlay } from '../lib/icons'
  import { onMount } from 'svelte'
  import { navToAlbum } from '../lib/stores'
  import { Api, loadImage } from '../lib/api'
  import { showPlaylistMenu } from '../lib/playlistMenu'
  import { lazyLoad } from '../lib/lazyLoad'
  import { getCached, setCached } from '../lib/listCache'
  import { createAbortController } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import type { Album } from '../lib/types/tauri-commands'

  const ALBUM_ROW_HEIGHT = 60

  let albums = $state<Album[]>(getCached<Album[]>('albums') ?? [])
  let loading = $state(albums.length === 0)
  let error = $state('')
  const abortCtrl = createAbortController()

  const RELEASE_ORDER = ['album', 'ep', 'single', 'live', 'compilation', 'other']
  const RELEASE_LABELS: Record<string, string> = { album: 'Albums', ep: 'EPs', single: 'Singles', live: 'Live', compilation: 'Compilations', other: 'Other' }

  const grouped = $derived.by(() => {
    const map: Record<string, Album[]> = {}
    for (const a of albums) {
      const rt = (a.releaseType ?? 'album').toLowerCase()
      const key = RELEASE_ORDER.includes(rt) ? rt : 'other'
      if (!map[key]) map[key] = []
      map[key].push(a)
    }
    // Only show full albums in the Albums tab; EPs/singles live on artist pages
    return RELEASE_ORDER.filter(k => k === 'album' && map[k]?.length).map(k => ({ key: k, label: RELEASE_LABELS[k], items: map[k] }))
  })

  const flatAlbums = $derived(grouped.flatMap(section => section.items))

  onMount(async () => {
    if (albums.length > 0) return
    const signal = abortCtrl.renew()
    try {
      albums = await Api.getAlbums(signal)
      setCached('albums', albums)
    } catch (e: any) {
      if (!signal.aborted) error = e.message
    } finally {
      if (!signal.aborted) loading = false
    }
  })
</script>

{#if loading}
  <div class="loading-msg">Loading albums…</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else if albums.length === 0}
  <div class="loading-msg">No albums found.</div>
{:else}
  <VirtualList items={flatAlbums} itemHeight={ALBUM_ROW_HEIGHT}>
    {#snippet children(album, _index)}
      <div
        class="album-row"
        role="button"
        tabindex="0"
        onclick={() => navToAlbum(album.id)}
        onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToAlbum(album.id)}
      >
        <div class="album-art-sm">
          {#if album.coverArtId}
            <img use:lazyLoad={img => loadImage(img, album.coverArtId, abortCtrl.signal)} alt="" />
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
          title="Add to playlist"
          onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'album', albumId: album.id, albumName: album.name }) }}
        >+</button>
      </div>
    {/snippet}
  </VirtualList>
{/if}
