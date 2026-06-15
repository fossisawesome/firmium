<script lang="ts">
  import {
    lyricsOpen, lyricsLines, lyricsSynced, lyricsStatus, currentTrack,
    lyricsWordTimings, lyricsGlowColor, lyricsWordFillEnabled,
    currentPosition, playbackState,
  } from '../lib/stores'
  import { activeLyricIdx } from '../lib/playback'
  import { IconClose } from '../lib/icons'

  let lyricsBody: HTMLDivElement | undefined = $state()

  // Scroll the active lyric into view when it changes.
  $effect(() => {
    if ($activeLyricIdx >= 0 && lyricsBody) {
      const els = lyricsBody.querySelectorAll('.lyric-line')
      const el = els[$activeLyricIdx]
      if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    }
  })

  // Word-by-word karaoke fill: interpolate playback position between the
  // ~750ms position-poll updates and update each word's --progress var
  // directly on the DOM (avoids per-frame Svelte reactivity overhead).
  $effect(() => {
    const words = $lyricsWordTimings[$activeLyricIdx]
    if (!$lyricsOpen || !$lyricsSynced || !$lyricsWordFillEnabled || !words?.length) return

    const startPosMs = $currentPosition * 1000
    const startTs = performance.now()
    let rafId: number

    const tick = () => {
      const elapsedMs = $playbackState === 'playing' ? performance.now() - startTs : 0
      const nowMs = startPosMs + elapsedMs
      const spans = lyricsBody?.querySelectorAll('.lyric-line.active .lyric-word')
      spans?.forEach((el, i) => {
        const w = words[i]
        if (!w) return
        const span = Math.max(1, w.endMs - w.startMs)
        const progress = Math.min(1, Math.max(0, (nowMs - w.startMs) / span))
        ;(el as HTMLElement).style.setProperty('--progress', progress.toString())
      })
      rafId = requestAnimationFrame(tick)
    }
    rafId = requestAnimationFrame(tick)

    return () => cancelAnimationFrame(rafId)
  })

  function close() {
    lyricsOpen.set(false)
  }

  // Swipe-down-to-close: drag the header bar downward to dismiss the panel.
  let dragOffset = $state(0)
  let dragStartY = 0
  let isDragging = false

  function onDragStart(e: TouchEvent | MouseEvent) {
    dragStartY = ('touches' in e ? e.touches[0] : e).clientY
    isDragging = true
  }

  function onDragMove(e: TouchEvent | MouseEvent) {
    if (!isDragging) return
    const dy = ('touches' in e ? e.touches[0] : e).clientY - dragStartY
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
  style="--lyrics-glow: {$lyricsGlowColor};{dragOffset > 0 ? ` transform: translateY(${dragOffset}px); transition: none` : ''}"
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
          {#if i === $activeLyricIdx && $lyricsWordFillEnabled && $lyricsWordTimings[i]?.length}
            {#each $lyricsWordTimings[i] as word, wi}
              <span class="lyric-word" style="--progress: 0">{word.text}</span>{wi < $lyricsWordTimings[i].length - 1 ? ' ' : ''}
            {/each}
          {:else}
            {line.value.trim() === '' ? '· · ·' : line.value}
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>
