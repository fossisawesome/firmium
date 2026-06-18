<script lang="ts">
  import { IconList } from '../lib/icons'
  import { loadImage } from '../lib/api'
  import { lazyLoad } from '../lib/lazyLoad'

  // Spotify-style playlist cover built from the first distinct song covers.
  // 1 cover fills the square; 2 split side-by-side; 3 = one tall left + two
  // stacked right; 4 = 2x2 grid. 0 covers falls back to a placeholder glyph.
  let { covers = [] }: { covers: (string | null | undefined)[] } = $props()

  const tiles = $derived((() => {
    const seen = new Set<string>()
    const out: string[] = []
    for (const c of covers) {
      if (c && !seen.has(c)) { seen.add(c); out.push(c) }
      if (out.length === 4) break
    }
    return out
  })())
</script>

<div
  class="pl-mosaic"
  class:count-1={tiles.length === 1}
  class:count-2={tiles.length === 2}
  class:count-3={tiles.length === 3}
  class:count-4={tiles.length >= 4}
>
  {#if tiles.length === 0}
    <span class="pl-mosaic-empty icon">{@html IconList}</span>
  {:else}
    {#each tiles as cover (cover)}
      <img class="pl-mosaic-tile" use:lazyLoad={img => loadImage(img, cover, null)} alt="" />
    {/each}
  {/if}
</div>

<style>
  .pl-mosaic {
    display: grid;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--surface2);
  }
  .pl-mosaic.count-1 { grid-template-columns: 1fr; grid-template-rows: 1fr; }
  .pl-mosaic.count-2 { grid-template-columns: 1fr 1fr; grid-template-rows: 1fr; }
  .pl-mosaic.count-3,
  .pl-mosaic.count-4 { grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr; }
  /* 3 tiles: first spans the full-height left column, other two stack on the right. */
  .pl-mosaic.count-3 .pl-mosaic-tile:first-child { grid-row: 1 / span 2; }
  .pl-mosaic-tile { width: 100%; height: 100%; object-fit: cover; display: block; }
  .pl-mosaic-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    color: var(--muted);
  }
</style>
