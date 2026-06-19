<script lang="ts">
  import { dataSource } from '../lib/dataSource'
  import { dataSourceVersion } from '../lib/stores'
  import { getCached, setCached } from '../lib/listCache'
  import { createAbortController, extractGenres, albumDecade } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
  import type { Album } from '../lib/types/tauri-commands'

  const ALBUM_ROW_HEIGHT = 60

  let albums = $state<Album[]>(getCached<Album[]>('albums') ?? [])
  let loading = $state(albums.length === 0)
  let error = $state('')
  const abortCtrl = createAbortController()

  let selectedGenres = $state(new Set<string>())
  let selectedDecades = $state(new Set<string>())

  const allGenres = $derived.by(() => {
    const counts = new Map<string, number>()
    for (const a of albums) {
      for (const g of extractGenres(a.genres)) {
        counts.set(g, (counts.get(g) ?? 0) + 1)
      }
    }
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([name]) => name)
  })

  const allDecades = $derived.by(() => {
    const set = new Set<string>()
    for (const a of albums) {
      const d = albumDecade(a.year)
      if (d) set.add(d)
    }
    return [...set].sort()
  })

  function toggleFilter(set: Set<string>, value: string): Set<string> {
    const next = new Set(set)
    if (next.has(value)) next.delete(value); else next.add(value)
    return next
  }

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
    return RELEASE_ORDER.filter(k => k === 'album' && map[k]?.length).map(k => ({ key: k, label: RELEASE_LABELS[k], items: map[k] }))
  })

  const flatAlbums = $derived.by(() => {
    let list = grouped.flatMap(section => section.items)
    if (selectedGenres.size > 0) {
      list = list.filter(a => {
        const ag = extractGenres(a.genres)
        return [...selectedGenres].some(g => ag.includes(g))
      })
    }
    if (selectedDecades.size > 0) {
      list = list.filter(a => {
        const d = albumDecade(a.year)
        return d !== null && selectedDecades.has(d)
      })
    }
    return list
  })

  const hasActiveFilters = $derived(selectedGenres.size > 0 || selectedDecades.size > 0)

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
  {#if allGenres.length > 0 || allDecades.length > 0}
    <div class="filter-bar">
      {#if allDecades.length > 0}
        <div class="filter-group">
          {#each allDecades as decade}
            <button
              class="filter-chip"
              class:active={selectedDecades.has(decade)}
              onclick={() => selectedDecades = toggleFilter(selectedDecades, decade)}
            >{decade}</button>
          {/each}
        </div>
      {/if}
      {#if allGenres.length > 0}
        <div class="filter-group">
          {#each allGenres.slice(0, 20) as genre}
            <button
              class="filter-chip"
              class:active={selectedGenres.has(genre)}
              onclick={() => selectedGenres = toggleFilter(selectedGenres, genre)}
            >{genre}</button>
          {/each}
        </div>
      {/if}
      {#if hasActiveFilters}
        <button class="filter-chip filter-clear" onclick={() => { selectedGenres = new Set(); selectedDecades = new Set() }}>Clear filters</button>
      {/if}
    </div>
  {/if}
  <VirtualList items={flatAlbums} itemHeight={ALBUM_ROW_HEIGHT}>
    {#snippet children(album, _index)}
      <AlbumRow {album} signal={abortCtrl.signal} />
    {/snippet}
  </VirtualList>
</LoadingState>
