<script lang="ts">
  import { dataVersion } from '../lib/stores'
  import { Api } from '../lib/api'
  import { errorMessage } from '../lib/ipc'
  import { extractGenres, albumDecade } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import LoadingState from '../components/LoadingState.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
  import type { Album } from '../lib/types'

  const ROW_HEIGHT = 60

  let albums = $state<Album[]>([])
  let loading = $state(true)
  let error = $state('')
  let selectedGenres = $state(new Set<string>())
  let selectedDecades = $state(new Set<string>())

  const allGenres = $derived.by(() => {
    const counts = new Map<string, number>()
    for (const a of albums) for (const g of extractGenres(a.genres)) counts.set(g, (counts.get(g) ?? 0) + 1)
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([name]) => name)
  })
  const allDecades = $derived([...new Set(albums.map(a => albumDecade(a.year)).filter((d): d is string => !!d))].sort())

  const toggle = (set: Set<string>, v: string) => { const n = new Set(set); n.has(v) ? n.delete(v) : n.add(v); return n }

  // Only full albums here; singles/EPs/etc. are filtered out as in the original list.
  const visible = $derived(
    albums
      .filter(a => (a.releaseType ?? 'album').toLowerCase() === 'album')
      .filter(a => selectedGenres.size === 0 || extractGenres(a.genres).some(g => selectedGenres.has(g)))
      .filter(a => selectedDecades.size === 0 || selectedDecades.has(albumDecade(a.year) ?? '')),
  )
  const hasFilters = $derived(selectedGenres.size > 0 || selectedDecades.size > 0)

  $effect(() => {
    $dataVersion
    let live = true
    loading = true
    error = ''
    Api.albums()
      .then(r => { if (live) albums = r })
      .catch(e => { if (live) error = errorMessage(e) })
      .finally(() => { if (live) loading = false })
    return () => { live = false }
  })
</script>

<LoadingState {loading} {error} empty={albums.length === 0} loadingMessage="Loading albums…" emptyMessage="No albums found.">
  {#if allGenres.length > 0 || allDecades.length > 0}
    <div class="filter-bar">
      {#if allDecades.length > 0}
        <div class="filter-group">
          {#each allDecades as d}<button class="filter-chip" class:active={selectedDecades.has(d)} onclick={() => (selectedDecades = toggle(selectedDecades, d))}>{d}</button>{/each}
        </div>
      {/if}
      {#if allGenres.length > 0}
        <div class="filter-group">
          {#each allGenres.slice(0, 20) as g}<button class="filter-chip" class:active={selectedGenres.has(g)} onclick={() => (selectedGenres = toggle(selectedGenres, g))}>{g}</button>{/each}
        </div>
      {/if}
      {#if hasFilters}
        <button class="filter-chip filter-clear" onclick={() => { selectedGenres = new Set(); selectedDecades = new Set() }}>Clear filters</button>
      {/if}
    </div>
  {/if}
  <VirtualList items={visible} itemHeight={ROW_HEIGHT}>
    {#snippet children(album)}<AlbumRow {album} />{/snippet}
  </VirtualList>
</LoadingState>
