<script>
  import { onDestroy } from 'svelte'
  import { get } from 'svelte/store'
  import {
    currentTrack, playbackState, currentPosition, trackDuration, isSeeking,
    volume, repeatOne, repeatAll, audioBridge, shuffleEnabled,
    lyricsOpen, setVolume, mobilePlayerOpen, queueSheetOpen
  } from '../lib/stores.js'
  import { fetchAndShowLyrics } from '../lib/playback.js'
  import { togglePlay, prevTrack, nextTrack, cycleRepeat, toggleShuffle } from '../lib/playerControls.js'
  import { formatDuration } from '../lib/utils.js'
  import { loadImage } from '../lib/api.js'
  import {
    IconPlay, IconPause, IconLoading, IconPrev, IconNext,
    IconRepeat, IconLyrics, IconVolume, IconVolumeHigh, IconMusic,
    IconShuffle, IconChevronDown, IconQueue
  } from '../lib/icons.js'

  // ── Cover art + dynamic background ──────────────────────────────────────────
  let coverImg = $state()
  let bgGradient = $state(null)

  $effect(() => {
    if ($currentTrack?.coverArtId && coverImg) {
      bgGradient = null
      loadImage(coverImg, $currentTrack.coverArtId, null)
    } else if (!$currentTrack?.coverArtId) {
      bgGradient = null
    }
  })

  // Extracts a dominant, saturation-weighted color from the loaded art and
  // builds a top-gradient that fades into the theme background.
  function onArtLoad() {
    if (!coverImg || !coverImg.complete) return
    try {
      const canvas = document.createElement('canvas')
      canvas.width = canvas.height = 12
      const ctx = canvas.getContext('2d')
      ctx.drawImage(coverImg, 0, 0, 12, 12)
      const data = ctx.getImageData(0, 0, 12, 12).data
      let r = 0, g = 0, b = 0, totalW = 0
      for (let i = 0; i < data.length; i += 4) {
        const max = Math.max(data[i], data[i + 1], data[i + 2])
        const min = Math.min(data[i], data[i + 1], data[i + 2])
        const sat = max === 0 ? 0 : (max - min) / max
        const w = sat * 2.5 + 0.3
        r += data[i] * w; g += data[i + 1] * w; b += data[i + 2] * w; totalW += w
      }
      const dr = Math.round((r / totalW) * 0.22)
      const dg = Math.round((g / totalW) * 0.22)
      const db = Math.round((b / totalW) * 0.22)
      bgGradient = `linear-gradient(180deg, rgb(${dr},${dg},${db}) 0%, var(--bg) 60%)`
    } catch (_) {
      bgGradient = null
    }
  }

  // ── Derived playback display ─────────────────────────────────────────────────
  const playIcon = $derived(
    $playbackState === 'loading' ? IconLoading : $playbackState === 'playing' ? IconPause : IconPlay
  )
  const posDisplay = $derived(formatDuration($currentPosition))
  const durDisplay = $derived(formatDuration($trackDuration ?? $currentTrack?.duration ?? 0))
  const seekMax = $derived($trackDuration ?? 100)
  const seekValue = $derived($trackDuration ? $currentPosition : 0)
  const progressPct = $derived(seekMax > 0 ? (seekValue / seekMax) * 100 : 0)

  // ── Marquee overflow detection ───────────────────────────────────────────────
  let titleEl = $state()
  let artistEl = $state()
  let titleOverflows = $state(false)
  let artistOverflows = $state(false)

  // Re-check overflow whenever the track changes (text content changes width).
  $effect(() => {
    $currentTrack
    requestAnimationFrame(() => {
      if (titleEl) {
        const overflow = titleEl.scrollWidth - titleEl.clientWidth
        titleOverflows = overflow > 2
        if (titleOverflows) titleEl.style.setProperty('--marquee-dist', `-${overflow}px`)
      }
      if (artistEl) {
        const overflow = artistEl.scrollWidth - artistEl.clientWidth
        artistOverflows = overflow > 2
        if (artistOverflows) artistEl.style.setProperty('--marquee-dist', `-${overflow}px`)
      }
    })
  })

  // ── Lyrics toggle ────────────────────────────────────────────────────────────
  function toggleLyrics() {
    const nowOpen = !get(lyricsOpen)
    lyricsOpen.set(nowOpen)
    mobilePlayerOpen.set(false)
    if (nowOpen) {
      const track = get(currentTrack)
      if (track) fetchAndShowLyrics(track)
    }
  }

  // ── Volume ───────────────────────────────────────────────────────────────────
  function handleVolumeInput(e) {
    const v = setVolume(e.target.value)
    const bridge = get(audioBridge)
    if (bridge) bridge.setVolume(v).catch(console.error)
  }

  // ── Seek ─────────────────────────────────────────────────────────────────────
  function startSeek(e) { e.stopPropagation(); isSeeking.set(true) }
  async function endSeek(e) {
    e.stopPropagation()
    isSeeking.set(false)
    const bridge = get(audioBridge)
    if (bridge) {
      try { await bridge.seek(Number(e.target.value)) } catch (err) { console.error('Seek failed:', err) }
    }
  }

  // ── Swipe-to-close (vertical) + swipe-art for prev/next (horizontal) ─────────
  let touchStartY = 0
  let touchStartX = 0
  let dragOffset = $state(0)
  let closing = $state(false)
  let overlayEl = $state()
  let closeTimer = null
  onDestroy(() => { if (closeTimer !== null) clearTimeout(closeTimer) })

  // Non-passive so we can preventDefault and stop page scroll while dragging down.
  $effect(() => {
    if (!overlayEl) return
    const handler = (e) => {
      const delta = e.touches[0].clientY - touchStartY
      if (delta > 0) {
        dragOffset = delta
        e.preventDefault()
      }
    }
    overlayEl.addEventListener('touchmove', handler, { passive: false })
    return () => overlayEl.removeEventListener('touchmove', handler)
  })

  function closeWithAnimation() {
    closing = true
    dragOffset = window.innerHeight
    if (closeTimer !== null) clearTimeout(closeTimer)
    closeTimer = setTimeout(() => mobilePlayerOpen.set(false), 300)
  }

  function onTouchStart(e) {
    touchStartY = e.touches[0].clientY
    touchStartX = e.touches[0].clientX
    dragOffset = 0
  }

  function onTouchEnd(e) {
    const deltaY = e.changedTouches[0].clientY - touchStartY
    if (deltaY > 72) {
      closeWithAnimation()
    } else {
      dragOffset = 0
    }
  }

  // Swipe left/right on the cover art thumbnail for prev/next.
  let artTouchStartX = 0
  let artTouchStartY = 0

  function onArtTouchStart(e) {
    artTouchStartX = e.touches[0].clientX
    artTouchStartY = e.touches[0].clientY
    // Stop propagation so the overlay drag-to-close doesn't also fire.
    e.stopPropagation()
  }

  function onArtTouchEnd(e) {
    e.stopPropagation()
    const dx = e.changedTouches[0].clientX - artTouchStartX
    const dy = e.changedTouches[0].clientY - artTouchStartY
    // Only treat as horizontal swipe if sideways motion dominates.
    if (Math.abs(dx) > 40 && Math.abs(dx) > Math.abs(dy) * 1.5) {
      if (dx < 0) nextTrack()
      else prevTrack()
    }
  }

  // ── Queue sheet ──────────────────────────────────────────────────────────────
  function openQueue() {
    queueSheetOpen.set(true)
    mobilePlayerOpen.set(false)
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={overlayEl}
  class="mp-overlay"
  class:mp-closing={closing}
  style:background={bgGradient ?? 'var(--bg)'}
  style={dragOffset > 0 ? `transform: translateY(${dragOffset}px)${closing ? '' : '; transition: none'}` : ''}
  ontouchstart={onTouchStart}
  ontouchend={onTouchEnd}
>
  <!-- Top bar: handle + close chevron -->
  <div class="mp-topbar">
    <div class="mp-handle-row">
      <div class="mp-handle"></div>
    </div>
    <button class="mp-close-btn" onclick={closeWithAnimation} aria-label="Close player">
      <span class="icon" style="width:26px;height:26px">{@html IconChevronDown}</span>
    </button>
  </div>

  <!-- Album art — swipe left/right for prev/next -->
  <div
    class="mp-art"
    ontouchstart={onArtTouchStart}
    ontouchend={onArtTouchEnd}
  >
    {#if $currentTrack?.coverArtId}
      <img bind:this={coverImg} alt="" onload={onArtLoad} />
    {:else}
      <span class="icon mp-no-art" style="width:64px;height:64px">{@html IconMusic}</span>
    {/if}
  </div>

  <!-- Track info with marquee for overflow -->
  <div class="mp-info">
    <div
      class="mp-title"
      class:mp-marquee={titleOverflows}
      bind:this={titleEl}
    >{$currentTrack?.title ?? '—'}</div>
    <div
      class="mp-artist"
      class:mp-marquee={artistOverflows}
      bind:this={artistEl}
    >{$currentTrack?.artist ?? 'Nothing playing'}</div>
  </div>

  <!-- Seek bar -->
  <div class="mp-progress">
    <div class="mp-progress-track">
      <div class="mp-progress-fill" style="width: {progressPct}%"></div>
    </div>
    <input
      class="mp-seek-input"
      type="range"
      aria-label="Seek"
      min="0"
      max={seekMax}
      step="0.1"
      value={seekValue}
      ontouchstart={startSeek}
      ontouchend={endSeek}
    />
    <div class="mp-times">
      <span>{posDisplay}</span>
      <span>{durDisplay}</span>
    </div>
  </div>

  <!-- Primary controls: prev / play / next -->
  <div class="mp-controls">
    <button class="mp-btn" onclick={prevTrack} aria-label="Previous">
      <span class="icon" style="width:28px;height:28px">{@html IconPrev}</span>
    </button>
    <button class="mp-btn mp-btn-main" onclick={togglePlay} aria-label="Play/Pause">
      <span class="icon" style="width:30px;height:30px">{@html playIcon}</span>
    </button>
    <button class="mp-btn" onclick={nextTrack} aria-label="Next">
      <span class="icon" style="width:28px;height:28px">{@html IconNext}</span>
    </button>
  </div>

  <!-- Secondary controls: shuffle / repeat / lyrics / queue -->
  <div class="mp-secondary">
    <button
      class="mp-btn mp-btn-sm"
      class:active={$shuffleEnabled}
      onclick={toggleShuffle}
      aria-label="Shuffle"
    >
      <span class="icon" style="width:22px;height:22px">{@html IconShuffle}</span>
    </button>

    <button
      class="mp-btn mp-btn-sm"
      class:active={$repeatOne || $repeatAll}
      onclick={cycleRepeat}
      aria-label="Repeat"
      style="position:relative"
    >
      <span class="icon" style="width:22px;height:22px">{@html IconRepeat}</span>
      {#if $repeatOne}<span class="mp-repeat-badge">1</span>{/if}
    </button>

    <button
      class="mp-btn mp-btn-sm"
      class:active={$lyricsOpen}
      onclick={toggleLyrics}
      aria-label="Lyrics"
    >
      <span class="icon" style="width:22px;height:22px">{@html IconLyrics}</span>
    </button>

    <button
      class="mp-btn mp-btn-sm"
      onclick={openQueue}
      aria-label="Queue"
    >
      <span class="icon" style="width:22px;height:22px">{@html IconQueue}</span>
    </button>
  </div>

  <!-- Volume slider -->
  <div class="mp-volume">
    <span class="icon mp-vol-icon" style="width:18px;height:18px">{@html IconVolume}</span>
    <input
      type="range"
      min="0" max="1" step="0.01"
      value={$volume}
      oninput={handleVolumeInput}
      aria-label="Volume"
    />
    <span class="icon mp-vol-icon" style="width:18px;height:18px">{@html IconVolumeHigh}</span>
  </div>
</div>
