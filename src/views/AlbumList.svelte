<script lang="ts">
  import { app, navTo } from '../lib/stores.svelte'
  import { Api } from '../lib/api'
  import { errorMessage } from '../lib/ipc'
  import { extractGenres, albumDecade } from '../lib/utils'
import { cover } from '../lib/cover'
  import { IconMusic } from '../lib/icons'
  import LoadingState from '../components/LoadingState.svelte'
  import type { Album } from '../lib/types'

  let albums = $state<Album[]>([])
  let loading = $state(true)
  let error = $state('')
  let selectedGenres = $state(new Set<string>())
  let selectedDecades = $state(new Set<string>())
  let sort = $state<'name' | 'newest'>('name')

  const allGenres = $derived.by(() => {
    const counts = new Map<string, number>()
    for (const a of albums) for (const g of extractGenres(a.genres)) counts.set(g, (counts.get(g) ?? 0) + 1)
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([name]) => name)
  })
  const allDecades = $derived([...new Set(albums.map(a => albumDecade(a.year)).filter((d): d is string => !!d))].sort())

  const toggle = (set: Set<string>, v: string) => { const n = new Set(set); n.has(v) ? n.delete(v) : n.add(v); return n }

  const visible = $derived(
    albums
      .filter(a => selectedGenres.size === 0 || extractGenres(a.genres).some(g => selectedGenres.has(g)))
      .filter(a => selectedDecades.size === 0 || selectedDecades.has(albumDecade(a.year) ?? '')),
  )
  const groups = $derived.by(() => {
    const g: Record<'Albums' | 'EPs' | 'Singles', Album[]> = { Albums: [], EPs: [], Singles: [] }
    for (const a of visible) {
      const t = (a.releaseType ?? '').toLowerCase()
      if (t === 'single') g.Singles.push(a)
      else if (t === 'ep') g.EPs.push(a)
      else g.Albums.push(a)
    }
    return g
  })
  const hasFilters = $derived(selectedGenres.size > 0 || selectedDecades.size > 0)

  $effect(() => {
    app.dataVersion
    const bySort = sort
    let live = true
    loading = true
    error = ''
    // The server caps a single album-list page at 500, so 'newest' shows the latest 500.
    ;(bySort === 'newest' ? Api.albumList('newest', 500) : Api.albums())
      .then(r => { if (live) albums = r })
      .catch(e => { if (live) error = errorMessage(e) })
      .finally(() => { if (live) loading = false })
    return () => { live = false }
  })
</script>

<LoadingState {loading} {error} empty={albums.length === 0} loadingMessage="Loading albums…" emptyMessage="No albums found.">
  <div class="filter-bar">
    <div class="filter-group">
      <button class={["filter-chip", { active: sort === 'name' }]} onclick={() => (sort = 'name')}>A–Z</button>
      <button class={["filter-chip", { active: sort === 'newest' }]} onclick={() => (sort = 'newest')}>Recently added</button>
    </div>
  </div>
  {#if allGenres.length > 0 || allDecades.length > 0}
    <div class="filter-bar">
      {#if allDecades.length > 0}
        <div class="filter-group">
          {#each allDecades as d}<button class={["filter-chip", { active: selectedDecades.has(d) }]} onclick={() => (selectedDecades = toggle(selectedDecades, d))}>{d}</button>{/each}
        </div>
      {/if}
      {#if allGenres.length > 0}
        <div class="filter-group">
          {#each allGenres.slice(0, 20) as g}<button class={["filter-chip", { active: selectedGenres.has(g) }]} onclick={() => (selectedGenres = toggle(selectedGenres, g))}>{g}</button>{/each}
        </div>
      {/if}
      {#if hasFilters}
        <button class="filter-chip filter-clear" onclick={() => { selectedGenres = new Set(); selectedDecades = new Set() }}>Clear filters</button>
      {/if}
    </div>
  {/if}
  {#each ['Albums', 'EPs', 'Singles'] as const as category}
    {#if groups[category].length > 0}
      <div class="release-group-title">{category}</div>
      <div class="album-grid">
        {#each groups[category] as album (album.id)}
          <button class="album-tile" onclick={() => navTo('album', album.id)}>
            <div class="home-card-art">
              {#if album.coverArtId}<img {@attach cover(album.coverArtId)} alt="" />
              {:else}<div class="home-card-no-art"><span class="icon" style="width:24px;height:24px">{@html IconMusic}</span></div>{/if}
            </div>
            <span class="album-tile-name">{album.name}</span>
            <span class="album-tile-artist">{album.albumArtist}</span>
          </button>
        {/each}
      </div>
    {/if}
  {/each}
</LoadingState>

<style lang="scss">
  .album-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 16px; padding: 8px 0 16px; }
  .album-tile { display: flex; flex-direction: column; gap: 4px; min-width: 0; background: none; border: none; padding: 0; cursor: pointer; color: var(--text); text-align: left; content-visibility: auto; contain-intrinsic-size: 200px;
    :global(.home-card-art) { width: 100%; height: auto; aspect-ratio: 1; margin-bottom: 4px; }
    &:hover :global(.home-card-art) { box-shadow: inset 0 0 0 2px var(--accent); }
  }
  .album-tile-name { font-size: 13px; font-weight: bold; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .album-tile-artist { font-size: 11px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
