<script lang="ts">
  import { IconList } from '../lib/icons'
  import { cover } from '../lib/cover'

  // Spotify-style playlist cover from the first distinct song covers: 1 fills,
  // 2 split, 3 = tall left + two stacked right, 4 = 2x2 grid, 0 = placeholder.
  let { covers = [] }: { covers: (string | null | undefined)[] } = $props()

  const tiles = $derived.by(() => {
    const seen = new Set<string>()
    for (const c of covers) {
      if (c && !seen.has(c)) seen.add(c)
      if (seen.size === 4) break
    }
    return [...seen]
  })
</script>

<div class="pl-mosaic" class:count-1={tiles.length === 1} class:count-2={tiles.length === 2} class:count-3={tiles.length === 3} class:count-4={tiles.length >= 4}>
  {#if tiles.length === 0}
    <span class="pl-mosaic-empty icon">{@html IconList}</span>
  {:else}
    {#each tiles as id (id)}<img class="pl-mosaic-tile" use:cover={id} alt="" />{/each}
  {/if}
</div>

<style>
  .pl-mosaic { display: grid; width: 100%; height: 100%; overflow: hidden; background: var(--surface2); }
  .pl-mosaic.count-1 { grid-template-columns: 1fr; grid-template-rows: 1fr; }
  .pl-mosaic.count-2 { grid-template-columns: 1fr 1fr; grid-template-rows: 1fr; }
  .pl-mosaic.count-3, .pl-mosaic.count-4 { grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr; }
  .pl-mosaic.count-3 .pl-mosaic-tile:first-child { grid-row: 1 / span 2; }
  .pl-mosaic-tile { width: 100%; height: 100%; object-fit: cover; display: block; }
  .pl-mosaic-empty { display: flex; align-items: center; justify-content: center; width: 100%; height: 100%; color: var(--muted); }
</style>
