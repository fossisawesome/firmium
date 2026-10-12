<script lang="ts">
  import { openPanel, currentTrack, position, playbackState, settings } from '../lib/stores'
  import { Api } from '../lib/api'
  import { IconClose } from '../lib/icons'
  import type { Lyrics, Song } from '../lib/types'

  let lyrics = $state<Lyrics | null>(null)
  let status = $state('No track playing')
  let glow = $state('transparent')
  let body: HTMLDivElement | undefined = $state()

  const open = $derived($openPanel === 'lyrics')
  const lines = $derived(lyrics?.lines ?? [])
  const wordFill = $derived($settings?.lyricsWordFill ?? false)

  // Index of the line containing the playback position (synced lyrics only).
  const activeIdx = $derived.by(() => {
    if (!lyrics?.synced) return -1
    const ms = $position * 1000
    let idx = -1
    for (let i = 0; i < lines.length; i++) { if (lines[i].start <= ms) idx = i; else break }
    return idx
  })

  let loadedFor: string | null = null
  async function load(song: Song) {
    loadedFor = song.id
    lyrics = null
    status = 'Loading lyrics…'
    glow = 'transparent'
    Api.coverColors(song.coverArtId ?? '').then(c => {
      if (loadedFor === song.id && c.dominant) glow = `rgb(${c.dominant.r}, ${c.dominant.g}, ${c.dominant.b})`
    }).catch(() => {})
    try {
      const res = await Api.lyrics(song)
      if (loadedFor !== song.id) return
      lyrics = res
      if (!res) status = 'No lyrics available for this track'
    } catch {
      if (loadedFor === song.id) status = 'Failed to load lyrics'
    }
  }

  $effect(() => {
    const t = $currentTrack
    if (!open) return
    if (!t) { lyrics = null; status = 'No track playing'; loadedFor = null; return }
    if (t.id !== loadedFor) load(t)
  })

  $effect(() => {
    if (activeIdx >= 0 && body) body.querySelectorAll('.lyric-line')[activeIdx]?.scrollIntoView({ behavior: 'smooth', block: 'center' })
  })

  // Karaoke fill: interpolate between ~750ms position events and set each
  // word's --progress straight on the DOM to avoid per-frame reactivity.
  $effect(() => {
    const words = lines[activeIdx]?.words
    if (!open || !wordFill || !words?.length) return
    const startMs = $position * 1000
    const startTs = performance.now()
    const playing = $playbackState === 'playing'
    let raf = 0
    const tick = () => {
      const now = startMs + (playing ? performance.now() - startTs : 0)
      body?.querySelectorAll('.lyric-line.active .lyric-word').forEach((el, i) => {
        const w = words[i]
        if (!w) return
        const p = Math.min(1, Math.max(0, (now - w.startMs) / Math.max(1, w.endMs - w.startMs)))
        ;(el as HTMLElement).style.setProperty('--progress', String(p))
      })
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  })
</script>

<div class="lyrics-panel" class:open style="--lyrics-glow: {glow}">
  <div class="lyrics-header">
    <span class="lyrics-header-title">Lyrics</span>
    <button class="lyrics-close" onclick={() => openPanel.set(null)} aria-label="Close lyrics">
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">

  <div class="lyrics-body" bind:this={body}>
    {#if !$currentTrack}
      <div class="lyrics-status">No track playing</div>
    {:else if lines.length === 0}
      <div class="lyrics-status">{status}</div>
    {:else if !lyrics?.synced}
      {#each lines as line}<div class="lyric-line unsynced">{line.value || ' '}</div>{/each}
    {:else}
      {#each lines as line, i}
        <div class="lyric-line" class:active={i === activeIdx} class:past={i < activeIdx} class:upcoming={i > activeIdx}
          class:empty-line={line.value.trim() === ''}>
          {#if i === activeIdx && wordFill && line.words.length}
            {#each line.words as word, wi}
              <span class="lyric-word" style="--progress: 0">{word.text}</span>{wi < line.words.length - 1 ? ' ' : ''}
            {/each}
          {:else}
            {line.value.trim() === '' ? '· · ·' : line.value}
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>
