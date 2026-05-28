<script>
  import { get } from 'svelte/store'
  import {
    currentTrack, playbackState, currentPosition, trackDuration, isSeeking,
    volume, repeatOne, repeatAll, audioBridge,
    lyricsOpen, setVolume
  } from '../lib/stores.js'
  import { fetchAndShowLyrics } from '../lib/playback.js'
  import { formatDuration } from '../lib/utils.js'
  import { loadImage } from '../lib/api.js'
  import { isMobile } from '../lib/platform.js'
  import { mobilePlayerOpen } from '../lib/stores.js'
  import { togglePlay, prevTrack, nextTrack, cycleRepeat } from '../lib/playerControls.js'
  import {
    IconPlay, IconPause, IconLoading, IconPrev, IconNext,
    IconRepeat, IconLyrics, IconVolume, IconMusic, IconChevronDown
  } from '../lib/icons.js'

  function openFullPlayer() {
    if (isMobile && $currentTrack) mobilePlayerOpen.set(true)
  }

  const playIcon = $derived(
    $playbackState === 'loading' ? IconLoading : $playbackState === 'playing' ? IconPause : IconPlay
  )

  const posDisplay = $derived(formatDuration($currentPosition))
  const durDisplay = $derived(formatDuration($trackDuration ?? $currentTrack?.duration ?? 0))
  const seekMax = $derived($trackDuration ?? 100)
  const seekValue = $derived($trackDuration ? $currentPosition : 0)
  const progressPct = $derived(seekMax > 0 ? (seekValue / seekMax) * 100 : 0)

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

  let npCoverImg = $state()
  $effect(() => {
    if ($currentTrack?.coverArtId && npCoverImg) {
      loadImage(npCoverImg, $currentTrack.coverArtId, null)
    }
  })
</script>

<div class="player-bar">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="now-playing" role={isMobile ? 'button' : undefined} onclick={openFullPlayer}>
    <div class="np-art">
      {#if $currentTrack?.coverArtId}
        <img bind:this={npCoverImg} class="np-cover-img" alt="" />
      {:else}
        <span class="icon" style="width:20px;height:20px;color:var(--muted)">{@html IconMusic}</span>
      {/if}
    </div>
    <div class="np-info">
      <div class="np-title">{$currentTrack?.title ?? '—'}</div>
      <div class="np-artist">{$currentTrack?.artist ?? 'No track selected'}</div>
    </div>
    {#if isMobile && $currentTrack}
      <span class="np-chevron icon" style="width:18px;height:18px;color:var(--muted);transform:rotate(-90deg);flex-shrink:0">{@html IconChevronDown}</span>
    {/if}
    <div class="vol-row">
      <span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconVolume}</span>
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
      style="--pct: {progressPct}%"
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
    <button class="ctrl-btn prev-ctrl" onclick={prevTrack} title="Previous">
      <span class="icon" style="width:15px;height:15px">{@html IconPrev}</span>
    </button>
    <button class="ctrl-btn main-ctrl" onclick={togglePlay} title="Play/Pause">
      <span class="icon" style="width:20px;height:20px">{@html playIcon}</span>
    </button>
    <button class="ctrl-btn" onclick={nextTrack} title="Next">
      <span class="icon" style="width:15px;height:15px">{@html IconNext}</span>
    </button>
    <button
      class="ctrl-btn secondary-ctrl repeat-btn"
      class:active={$repeatOne || $repeatAll}
      onclick={cycleRepeat}
      title={$repeatOne ? 'Repeat One' : $repeatAll ? 'Repeat All' : 'Repeat Off'}
    >
      <span class="icon" style="width:16px;height:16px">{@html IconRepeat}</span>
      {#if $repeatOne}<span class="repeat-badge">1</span>{/if}
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$lyricsOpen} onclick={toggleLyrics} title="Lyrics">
      <span class="icon" style="width:16px;height:16px">{@html IconLyrics}</span>
    </button>
  </div>
</div>
