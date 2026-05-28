<script>
  import { onDestroy } from 'svelte'
  import { get } from 'svelte/store'
  import {
    currentTrack, playbackState, currentPosition, trackDuration, isSeeking,
    volume, repeatOne, repeatAll, audioBridge, shuffleEnabled,
    lyricsOpen, setVolume, mobilePlayerOpen, queueSheetOpen
  } from '../lib/stores.js'
  import { fetchAndShowLyrics } from '../lib/playback.js'
  import { showPlaylistMenu } from '../lib/playlistMenu.js'
  import { togglePlay, prevTrack, nextTrack, cycleRepeat, toggleShuffle } from '../lib/playerControls.js'
  import { formatDuration } from '../lib/utils.js'
  import { loadImage } from '../lib/api.js'
  import {
    IconPlay, IconPause, IconLoading, IconPrev, IconNext,
    IconRepeat, IconVolume, IconVolumeHigh, IconMusic,
    IconShuffle, IconQueue, IconPlus
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

  // ── Lyrics (tap album art to toggle) ────────────────────────────────────────
  // LyricsPanel renders above MobilePlayer (z-index 400 > 300 on mobile).
  function onArtClick() {
    const track = get(currentTrack)
    if (!track) return
    const nowOpen = !get(lyricsOpen)
    lyricsOpen.set(nowOpen)
    if (nowOpen) fetchAndShowLyrics(track)
  }

  // ── Add to playlist ──────────────────────────────────────────────────────────
  function handleAddToPlaylist(e) {
    const track = get(currentTrack)
    if (!track) return
    showPlaylistMenu(e.currentTarget, { type: 'tracks', tracks: [track] })
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
  let springing = $state(false)
  let overlayEl = $state()
  let closeTimer = null
  let springTimer = null
  // null = undetermined, true = tracking a vertical close gesture, false = not
  let gestureVertical = null

  onDestroy(() => {
    if (closeTimer !== null) clearTimeout(closeTimer)
    if (springTimer !== null) clearTimeout(springTimer)
  })

  // Non-passive so we can preventDefault and stop page scroll while dragging down.
  // Only intercepts when at the top of scroll AND the gesture is primarily downward.
  $effect(() => {
    if (!overlayEl) return
    const handler = (e) => {
      const dy = e.touches[0].clientY - touchStartY
      const dx = Math.abs(e.touches[0].clientX - touchStartX)
      // Determine gesture axis once the finger has moved enough to be confident
      if (gestureVertical === null && (Math.abs(dy) > 8 || dx > 8)) {
        gestureVertical = dy > 0 && Math.abs(dy) > dx
      }
      if (gestureVertical && overlayEl.scrollTop === 0 && dy > 0) {
        dragOffset = dy
        springing = false
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
    springing = false
    gestureVertical = null
  }

  function onTouchEnd(e) {
    const deltaY = e.changedTouches[0].clientY - touchStartY
    if (deltaY > 72) {
      closeWithAnimation()
    } else if (dragOffset > 0) {
      // Smooth spring back to resting position
      springing = true
      dragOffset = 0
      if (springTimer !== null) clearTimeout(springTimer)
      springTimer = setTimeout(() => { springing = false }, 380)
    }
  }

  // Swipe left/right on the cover art thumbnail for prev/next.
  let artTouchStartX = 0
  let artTouchStartY = 0
  let artTouchMoved = false

  function onArtTouchStart(e) {
    artTouchStartX = e.touches[0].clientX
    artTouchStartY = e.touches[0].clientY
    artTouchMoved = false
    e.stopPropagation()
  }

  function onArtTouchMove(e) {
    const dx = Math.abs(e.touches[0].clientX - artTouchStartX)
    const dy = Math.abs(e.touches[0].clientY - artTouchStartY)
    if (dx > 8 || dy > 8) artTouchMoved = true
    e.stopPropagation()
  }

  function onArtTouchEnd(e) {
    e.stopPropagation()
    const dx = e.changedTouches[0].clientX - artTouchStartX
    const dy = e.changedTouches[0].clientY - artTouchStartY
    if (Math.abs(dx) > 40 && Math.abs(dx) > Math.abs(dy) * 1.5) {
      // Horizontal swipe: prev/next
      if (dx < 0) nextTrack()
      else prevTrack()
    } else if (!artTouchMoved) {
      // Tap with no significant movement: toggle lyrics
      onArtClick()
    }
  }

  // ── Queue sheet — keep mobile player open underneath ─────────────────────────
  function openQueue() {
    queueSheetOpen.set(true)
    // Intentionally NOT closing mobilePlayerOpen so queue overlays the player
  }

  // ── Compose inline style for drag/spring animation ────────────────────────────
  const overlayTransformStyle = $derived(() => {
    if (dragOffset > 0 && !closing) return `transform: translateY(${dragOffset}px); transition: none`
    if (dragOffset > 0 && closing) return `transform: translateY(${dragOffset}px)`
    if (springing) return `transform: translateY(0); transition: transform 0.38s cubic-bezier(0.2, 0, 0, 1)`
    return ''
  })
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={overlayEl}
  class="mp-overlay"
  class:mp-closing={closing}
  style:background={bgGradient ?? 'var(--bg)'}
  style={overlayTransformStyle()}
  ontouchstart={onTouchStart}
  ontouchend={onTouchEnd}
>
  <!-- Drag handle only (no close button — swipe down to close) -->
  <div class="mp-topbar">
    <div class="mp-handle-row">
      <div class="mp-handle"></div>
    </div>
  </div>

  <!-- Album art — swipe left/right for prev/next, tap for lyrics -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="mp-art"
    role="button"
    tabindex="0"
    ontouchstart={onArtTouchStart}
    ontouchmove={onArtTouchMove}
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

  <!-- Secondary controls: shuffle / repeat / add-to-playlist / queue -->
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
      onclick={handleAddToPlaylist}
      aria-label="Add to playlist"
    >
      <span class="icon" style="width:22px;height:22px">{@html IconPlus}</span>
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
