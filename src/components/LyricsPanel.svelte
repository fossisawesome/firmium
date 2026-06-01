<script>
  import { lyricsOpen, lyricsLines, lyricsSynced, lyricsStatus, currentTrack } from '../lib/stores.js'
  import { activeLyricIdx } from '../lib/playback.js'
  import { IconClose } from '../lib/icons.js'

  let lyricsBody = $state()

  // Scroll the active lyric into view when it changes.
  $effect(() => {
    if ($activeLyricIdx >= 0 && lyricsBody) {
      const els = lyricsBody.querySelectorAll('.lyric-line')
      const el = els[$activeLyricIdx]
      if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    }
  })

  function close() {
    lyricsOpen.set(false)
  }

  // Swipe-down-to-close: drag the header bar downward to dismiss the panel.
  let dragOffset = $state(0)
  let dragStartY = 0
  let isDragging = false

  function onDragStart(e) {
    dragStartY = (e.touches ?? [e])[0].clientY
    isDragging = true
  }

  function onDragMove(e) {
    if (!isDragging) return
    const dy = (e.touches ?? [e])[0].clientY - dragStartY
    dragOffset = Math.max(0, dy)
  }

  function onDragEnd() {
    isDragging = false
    if (dragOffset > window.innerHeight * 0.25) {
      close()
    }
    dragOffset = 0
  }
</script>

<div
  class="lyrics-panel"
  class:open={$lyricsOpen}
  style={dragOffset > 0 ? `transform: translateY(${dragOffset}px); transition: none` : ''}
>
  <!-- Fills the status bar / safe area on mobile full-screen; zero-height on desktop -->
  <div class="lyrics-safe-top"></div>
  <div
    class="lyrics-header"
    role="presentation"
    ontouchstart={onDragStart}
    ontouchmove={onDragMove}
    ontouchend={onDragEnd}
  >
    <span class="lyrics-header-title">Lyrics</span>
    <button class="lyrics-close" onclick={close}>
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">

  <div class="lyrics-body" bind:this={lyricsBody}>
    {#if !$currentTrack}
      <div class="lyrics-status">No track playing</div>
    {:else if $lyricsLines.length === 0}
      <div class="lyrics-status">{$lyricsStatus}</div>
    {:else if !$lyricsSynced}
      {#each $lyricsLines as line}
        <div class="lyric-line unsynced">{line.value || ' '}</div>
      {/each}
    {:else}
      {#each $lyricsLines as line, i}
        <div
          class="lyric-line"
          class:active={i === $activeLyricIdx}
          class:past={i < $activeLyricIdx}
          class:upcoming={i > $activeLyricIdx}
          class:empty-line={line.value.trim() === ''}
        >
          {line.value.trim() === '' ? '· · ·' : line.value}
        </div>
      {/each}
    {/if}
  </div>
</div>
