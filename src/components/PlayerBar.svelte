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
    const state = await bridge.getState()
    if (state === 'paused') await bridge.resume()
    else if (state === 'playing') await bridge.pause()
    else if (state === 'stopped') playAt(get(queueIdx))
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

  function toggleRepeatOne() {
    const next = !get(repeatOne)
    repeatOne.set(next)
    if (next) repeatAll.set(false)
  }

  function toggleRepeatAll() {
    const next = !get(repeatAll)
    repeatAll.set(next)
    if (next) repeatOne.set(false)
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
    <button class="ctrl-btn secondary-ctrl" class:active={$repeatOne} onclick={toggleRepeatOne} title="Repeat One">⥀</button>
    <button class="ctrl-btn secondary-ctrl" class:active={$repeatAll} onclick={toggleRepeatAll} title="Repeat All">⥁</button>
    <button class="ctrl-btn secondary-ctrl" class:active={$lyricsOpen} onclick={toggleLyrics} title="Lyrics">🎙</button>
  </div>
</div>
