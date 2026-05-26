<script>
  import { get } from 'svelte/store'
  import {
    currentTrack, playbackState, currentPosition, trackDuration, isSeeking,
    volume, repeatOne, repeatAll, audioBridge, queue, queueIdx,
    lyricsOpen, setVolume
  } from '../lib/stores.js'
  import { playAt, fetchAndShowLyrics } from '../lib/playback.js'
  import { formatDuration } from '../lib/utils.js'
  import { loadImage } from '../lib/api.js'
  import { lazyLoad } from '../lib/lazyLoad.js'

  const playIcon = $derived(
    $playbackState === 'loading' ? '⏳' : $playbackState === 'playing' ? '⏸' : '▶'
  )

  const posDisplay = $derived(formatDuration($currentPosition))
  const durDisplay = $derived(formatDuration($trackDuration ?? $currentTrack?.duration ?? 0))
  const seekMax = $derived($trackDuration ?? 100)
  const seekValue = $derived($trackDuration ? $currentPosition : 0)

  async function togglePlay() {
    const bridge = get(audioBridge)
    if (!get(currentTrack) || !bridge) return
    const state = bridge.lastKnownState
    if (state === 'paused') await bridge.resume()
    else if (state === 'playing') await bridge.pause()
    else if (!state || state === 'stopped') playAt(get(queueIdx))
  }

  function prevTrack() {
    const idx = get(queueIdx)
    if (idx > 0) playAt(idx - 1)
  }

  function nextTrack() {
    const idx = get(queueIdx)
    const len = get(queue).length
    if (idx < len - 1) playAt(idx + 1)
    else if (get(repeatAll)) playAt(0)
  }

  // Cycles: off → repeat-one → repeat-all → off
  function cycleRepeat() {
    if (!get(repeatOne) && !get(repeatAll)) {
      repeatOne.set(true)
    } else if (get(repeatOne)) {
      repeatOne.set(false)
      repeatAll.set(true)
    } else {
      repeatAll.set(false)
    }
  }

  function toggleLyrics() {
    const nowOpen = !get(lyricsOpen)
    lyricsOpen.set(nowOpen)
    if (nowOpen) {
      const track = get(currentTrack)
      if (track) fetchAndShowLyrics(track)
    }
  }

  function handleVolumeInput(e) {
    const v = setVolume(e.target.value)
    const bridge = get(audioBridge)
    if (bridge) bridge.setVolume(v).catch(console.error)
  }

  function startSeek() { isSeeking.set(true) }
  async function endSeek(e) {
    isSeeking.set(false)
    const bridge = get(audioBridge)
    if (bridge) {
      try { await bridge.seek(Number(e.target.value)) } catch (err) { console.error('Seek failed:', err) }
    }
  }

  // Cover art for the now-playing bar.
  let npCoverImg = $state()
  $effect(() => {
    if ($currentTrack?.coverArtId && npCoverImg) {
      loadImage(npCoverImg, $currentTrack.coverArtId, null)
    }
  })
</script>

<div class="player-bar">
  <div class="now-playing">
    <div class="np-art">
      {#if $currentTrack?.coverArtId}
        <img bind:this={npCoverImg} class="np-cover-img" alt="" />
      {:else}
        ♪
      {/if}
    </div>
    <div class="np-info">
      <div class="np-title">{$currentTrack?.title ?? '—'}</div>
      <div class="np-artist">{$currentTrack?.artist ?? 'No track selected'}</div>
    </div>
    <div class="vol-row">
      <span>☊</span>
      <input
        type="range"
        class="volume-slider"
        min="0" max="1" step="0.01"
        value={$volume}
        oninput={handleVolumeInput}
      />
    </div>
  </div>

  <div class="progress-row">
    <span class="time">{posDisplay}</span>
    <input
      type="range"
      id="seekBar"
      min="0"
      max={seekMax}
      step="0.1"
      value={seekValue}
      onmousedown={startSeek}
      onmouseup={endSeek}
      ontouchstart={startSeek}
      ontouchend={endSeek}
    />
    <span class="time right">{durDisplay}</span>
  </div>

  <div class="controls">
    <button class="ctrl-btn" onclick={prevTrack} title="Previous">⏮</button>
    <button class="ctrl-btn main-ctrl" onclick={togglePlay} title="Play/Pause">{playIcon}</button>
    <button class="ctrl-btn" onclick={nextTrack} title="Next">⏭</button>
    <!-- Single repeat button: off → repeat-one → repeat-all → off -->
    <button
      class="ctrl-btn secondary-ctrl repeat-btn"
      class:active={$repeatOne || $repeatAll}
      onclick={cycleRepeat}
      title={$repeatOne ? 'Repeat One' : $repeatAll ? 'Repeat All' : 'Repeat Off'}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="17 1 21 5 17 9"/>
        <path d="M3 11V9a4 4 0 0 1 4-4h14"/>
        <polyline points="7 23 3 19 7 15"/>
        <path d="M21 13v2a4 4 0 0 1-4 4H3"/>
      </svg>
      {#if $repeatOne}<span class="repeat-badge">1</span>{/if}
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$lyricsOpen} onclick={toggleLyrics} title="Lyrics">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-label="Lyrics">
        <rect x="8" y="2" width="8" height="11" rx="4"/>
        <line x1="8" y1="6" x2="16" y2="6"/>
        <line x1="8" y1="9" x2="16" y2="9"/>
        <path d="M6 13 Q6 17 12 17 Q18 17 18 13"/>
        <line x1="12" y1="17" x2="12" y2="21"/>
        <line x1="9" y1="21" x2="15" y2="21"/>
      </svg>
    </button>
  </div>
</div>
