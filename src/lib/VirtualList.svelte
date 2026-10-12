<script lang="ts" generics="T">
  import { onMount } from 'svelte'
  import type { Snippet } from 'svelte'

  // Fixed-row-height virtual list. Renders only the rows visible within the
  // nearest `.list-panel` scroll container (plus `overscan` rows above/below),
  // using absolute positioning inside a full-height spacer so native scrolling
  // and scrollbar sizing keep working unchanged.
  interface Props {
    items: T[]
    itemHeight: number
    overscan?: number
    children: Snippet<[T, number]>
  }

  let { items, itemHeight, overscan = 6, children }: Props = $props()

  let wrapper = $state<HTMLDivElement>()
  let scrollParent: HTMLElement | null = null
  let range = $state({ start: 0, end: 0 })

  function update() {
    if (!wrapper) return
    if (!scrollParent) { range = { start: 0, end: items.length }; return }
    const wrapperTop = wrapper.getBoundingClientRect().top - scrollParent.getBoundingClientRect().top + scrollParent.scrollTop
    const relScroll = Math.max(0, scrollParent.scrollTop - wrapperTop)
    const viewHeight = scrollParent.clientHeight
    const start = Math.max(0, Math.floor(relScroll / itemHeight) - overscan)
    const end = Math.min(items.length, Math.ceil((relScroll + viewHeight) / itemHeight) + overscan)
    range = { start, end }
  }

  onMount(() => {
    scrollParent = wrapper?.closest('.list-panel') as HTMLElement | null
    update()
    if (!scrollParent) return
    scrollParent.addEventListener('scroll', update, { passive: true })
    const ro = new ResizeObserver(update)
    ro.observe(scrollParent)
    return () => {
      scrollParent?.removeEventListener('scroll', update)
      ro.disconnect()
    }
  })

  $effect(() => { items.length; update() })
</script>

<div bind:this={wrapper} style="position:relative; height:{items.length * itemHeight}px;">
  {#each items.slice(range.start, range.end) as item, i (range.start + i)}
    <div style="position:absolute; top:{(range.start + i) * itemHeight}px; left:0; right:0; height:{itemHeight}px;">
      {@render children(item, range.start + i)}
    </div>
  {/each}
</div>
