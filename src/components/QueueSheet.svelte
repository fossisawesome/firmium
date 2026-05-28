<script>
  import { onDestroy } from 'svelte'
  import { queue, queueIdx, queueSheetOpen } from '../lib/stores.js'
  import { playAt } from '../lib/playback.js'
  import { formatDuration } from '../lib/utils.js'
  import { IconMusic, IconChevronDown } from '../lib/icons.js'
  import { loadImage } from '../lib/api.js'

  // Drag-to-close state, mirrors MobilePlayer pattern.
  let touchStartY = 0
  let dragOffset = $state(0)
  let closing = $state(false)
  let sheetEl = $state()
  let closeTimer = null
  onDestroy(() => { if (closeTimer !== null) clearTimeout(closeTimer) })

  $effect(() => {
    if (!sheetEl) return
    const handler = (e) => {
      const delta = e.touches[0].clientY - touchStartY
      if (delta > 0) { dragOffset = delta; e.preventDefault() }
    }
    sheetEl.addEventListener('touchmove', handler, { passive: false })
    return () => sheetEl.removeEventListener('touchmove', handler)
  })

  function closeSheet() {
    closing = true
    dragOffset = window.innerHeight
    if (closeTimer !== null) clearTimeout(closeTimer)
    closeTimer = setTimeout(() => queueSheetOpen.set(false), 300)
  }

  function onTouchStart(e) { touchStartY = e.touches[0].clientY; dragOffset = 0 }
  function onTouchEnd(e) {
    const delta = e.changedTouches[0].clientY - touchStartY
    if (delta > 60) closeSheet()
    else dragOffset = 0
  }

  function playTrack(idx) {
    playAt(idx)
    closeSheet()
  }

  // Scroll active track into view when sheet opens.
  let listEl = $state()
  $effect(() => {
    if (!listEl) return
    const active = listEl.querySelector('.qs-track-active')
    if (active) active.scrollIntoView({ block: 'center' })
  })
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={sheetEl}
  class="qs-overlay"
  class:qs-closing={closing}
  style={dragOffset > 0 ? `transform: translateY(${dragOffset}px)${closing ? '' : '; transition: none'}` : ''}
  ontouchstart={onTouchStart}
  ontouchend={onTouchEnd}
>
  <div class="qs-handle-row">
    <div class="qs-handle"></div>
  </div>

  <div class="qs-header">
    <span class="qs-title">Up Next</span>
    <button class="qs-close-btn" onclick={closeSheet} aria-label="Close queue">
      <span class="icon" style="width:20px;height:20px">{@html IconChevronDown}</span>
    </button>
  </div>

  <div class="qs-list" bind:this={listEl}>
    {#if $queue.length === 0}
      <div class="qs-empty">Queue is empty</div>
    {:else}
      {#each $queue as track, i}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="qs-track"
          class:qs-track-active={i === $queueIdx}
          onclick={() => playTrack(i)}
          role="button"
          tabindex="0"
        >
          <div class="qs-track-index">{i === $queueIdx ? '▶' : i + 1}</div>
          <div class="qs-track-info">
            <div class="qs-track-title">{track.title ?? '—'}</div>
            <div class="qs-track-artist">{track.artist ?? ''}</div>
          </div>
          <div class="qs-track-dur">{formatDuration(track.duration ?? 0)}</div>
        </div>
      {/each}
    {/if}
  </div>
</div>
