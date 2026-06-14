<script lang="ts">
  import { dataSource } from '../lib/dataSource'
  import { dataSourceVersion } from '../lib/stores'
  import { getCached, setCached } from '../lib/listCache'
  import { createAbortController } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
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

  let initialized = false

  $effect(() => {
    const source = $dataSource
    $dataSourceVersion
    if (!initialized && albums.length > 0) { initialized = true; return }
    initialized = true
    loading = true
    error = ''
    const signal = abortCtrl.renew()
    ;(async () => {
      try {
        albums = await source.getAlbums(signal)
        setCached('albums', albums)
      } catch (e: any) {
        if (!signal.aborted) error = e.message
      } finally {
        if (!signal.aborted) loading = false
      }
    })()
  })
</script>

<LoadingState {loading} {error} empty={albums.length === 0} loadingMessage="Loading albums…" emptyMessage="No albums found.">
  <VirtualList items={flatAlbums} itemHeight={ALBUM_ROW_HEIGHT}>
    {#snippet children(album, _index)}
      <AlbumRow {album} signal={abortCtrl.signal} />
    {/snippet}
  </VirtualList>
</LoadingState>
