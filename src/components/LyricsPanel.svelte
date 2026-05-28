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
</script>

<div class="lyrics-panel" class:open={$lyricsOpen}>
  <!-- Fills the status bar / safe area on mobile full-screen; zero-height on desktop -->
  <div class="lyrics-safe-top"></div>
  <div class="lyrics-header">
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
