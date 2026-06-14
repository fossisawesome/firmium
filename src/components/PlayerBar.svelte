<script lang="ts">
  import { get } from 'svelte/store'
  import {
    currentTrack, playbackState, currentPosition, trackDuration, isSeeking,
    volume, repeatOne, repeatAll, audioBridge,
    lyricsOpen, setVolume, activeStreamInfo,
    hasSonicSimilarity, similarTracksOpen, similarTracksTrackId, similarTracksResults, similarTracksStatus,
    visualizerOpen
  } from '../lib/stores'
  import { fetchAndShowLyrics } from '../lib/playback'
  import { formatDuration } from '../lib/utils'
  import { Api, loadImage } from '../lib/api'
  import { togglePlay, prevTrack, nextTrack, cycleRepeat } from '../lib/playerControls'
  import {
    IconPlay, IconPause, IconLoading, IconPrev, IconNext,
    IconRepeat, IconLyrics, IconVolume, IconMusic, IconHexagon, IconWaveform
  } from '../lib/icons'

  const playIcon = $derived(
    $playbackState === 'loading' ? IconLoading : $playbackState === 'playing' ? IconPause : IconPlay
  )

  const trackInfo = $derived.by(() => {
    const base = $currentTrack?.trackInfo ?? ''
    if ($activeStreamInfo?.bitPerfect) return base ? `${base} · Bit-perfect` : 'Bit-perfect'
    return base
  })

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

  function firstGenre(track: { genres?: unknown }): string | undefined {
    const genres = track.genres
    if (!Array.isArray(genres) || genres.length === 0) return undefined
    const first = genres[0]
    if (typeof first === 'object' && first !== null && 'name' in first) {
      const name = (first as { name?: unknown }).name
      return typeof name === 'string' ? name : undefined
    }
    return typeof first === 'string' ? first : undefined
  }

  async function toggleSimilarTracks() {
    const nowOpen = !get(similarTracksOpen)
    similarTracksOpen.set(nowOpen)
    if (!nowOpen) return
    const track = get(currentTrack)
    if (!track) return
    if (get(similarTracksTrackId) === track.id) return
    similarTracksTrackId.set(track.id)
    similarTracksStatus.set('Loading similar tracks…')
    similarTracksResults.set([])
    try {
      const results = get(hasSonicSimilarity)
        ? await Api.getSonicSimilarTracks(track.id)
        : await Api.getSimilarTracksFallback(track.id, track.artistId, firstGenre(track), 10)
      if (get(similarTracksTrackId) !== track.id) return
      similarTracksResults.set(results)
      similarTracksStatus.set('')
    } catch (e) {
      if (get(similarTracksTrackId) === track.id) {
        similarTracksStatus.set('Failed to load similar tracks')
        console.error('Similar tracks error:', e)
      }
    }
  }

  function handleVolumeInput(e: Event) {
    const v = setVolume(Number((e.target as HTMLInputElement).value))
    const bridge = get(audioBridge)
    if (bridge) bridge.setVolume(v).catch(console.error)
  }

  function startSeek() { isSeeking.set(true) }
  async function endSeek(e: Event) {
    isSeeking.set(false)
    const bridge = get(audioBridge)
    if (bridge) {
      try { await bridge.seek(Number((e.target as HTMLInputElement).value)) } catch (err) { console.error('Seek failed:', err) }
    }
  }

  let npCoverImg: HTMLImageElement | undefined = $state()
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
        <span class="icon" style="width:20px;height:20px;color:var(--muted)">{@html IconMusic}</span>
      {/if}
    </div>
    <div class="np-info">
      <div class="np-title">{$currentTrack?.title ?? '—'}</div>
      <div class="np-artist">{$currentTrack?.artist ?? 'No track selected'}</div>
      {#if trackInfo}
        <div class="np-format">{trackInfo}</div>
      {/if}
    </div>
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
    <button class="ctrl-btn secondary-ctrl" class:active={$similarTracksOpen} onclick={toggleSimilarTracks} title="Similar Tracks">
      <span class="icon" style="width:16px;height:16px">{@html IconHexagon}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$visualizerOpen} onclick={() => visualizerOpen.update(v => !v)} title="Visualizer">
      <span class="icon" style="width:16px;height:16px">{@html IconWaveform}</span>
    </button>
  </div>
</div>
