<script lang="ts">
  import { get } from 'svelte/store'
  import {
    currentTrack, playbackState, currentPosition, trackDuration, isSeeking,
    volume, repeatOne, repeatAll,
    lyricsOpen, setVolume,
    hasSonicSimilarity, similarTracksOpen, similarTracksTrackId, similarTracksResults, similarTracksStatus,
    visualizerOpen, audioStatsOpen,
  } from '../lib/stores'
  import { tauriInvoke, tauriFetch } from '../lib/tauri'
  import { fetchAndShowLyrics } from '../lib/playback'
  import { formatDuration, SafeStorage } from '../lib/utils'
  import { Api, Keyring, loadImage } from '../lib/api'
  import { togglePlay, prevTrack, nextTrack, cycleRepeat } from '../lib/playerControls'
  import {
    IconPlay, IconPause, IconLoading, IconPrev, IconNext,
    IconRepeat, IconLyrics, IconVolume, IconMusic, IconHexagon, IconWaveform, IconInfo
  } from '../lib/icons'

  const playIcon = $derived(
    $playbackState === 'loading' ? IconLoading : $playbackState === 'playing' ? IconPause : IconPlay
  )

  const trackInfo = $derived($currentTrack?.trackInfo ?? '')

  // While scrubbing (drag or keyboard), the slider drives the displayed time and
  // thumb position; null means "follow live playback position".
  let scrubPos = $state<number | null>(null)
  const posDisplay = $derived(formatDuration(scrubPos ?? $currentPosition))
  const effectiveDuration = $derived($trackDuration || $currentTrack?.duration || 0)
  const durDisplay = $derived(formatDuration(effectiveDuration))
  const seekMax = $derived(effectiveDuration || 100)
  const seekValue = $derived(effectiveDuration ? $currentPosition : 0)
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

  async function fetchLastfmSimilarTracks(artist: string, title: string, apiKey: string): Promise<import('../lib/types/tauri-commands').SimilarMatch[]> {
    const url = `https://ws.audioscrobbler.com/2.0/?method=track.getSimilar&artist=${encodeURIComponent(artist)}&track=${encodeURIComponent(title)}&api_key=${encodeURIComponent(apiKey)}&limit=15&format=json`
    const res = await tauriFetch(url)
    const data = await res.json()
    const tracks: { name: string; artist: { name: string }; match: string }[] = data?.similartracks?.track ?? []
    if (tracks.length === 0) return []
    const results: import('../lib/types/tauri-commands').SimilarMatch[] = []
    for (const t of tracks.slice(0, 10)) {
      try {
        const found = await Api.search(`${t.artist.name} ${t.name}`)
        const match = found.songs.find(s =>
          s.title.toLowerCase() === t.name.toLowerCase() &&
          (s.artist ?? '').toLowerCase() === t.artist.name.toLowerCase()
        )
        if (match) results.push({ song: match, similarity: parseFloat(t.match) })
      } catch { /* skip unresolvable tracks */ }
    }
    return results
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
      const stale = () => get(similarTracksTrackId) !== track.id

      // 1. sonicSimilarity (server extension)
      if (get(hasSonicSimilarity)) {
        try {
          const sonic = await Api.getSonicSimilarTracks(track.id)
          if (stale()) return
          if (sonic.length > 0) { similarTracksResults.set(sonic); similarTracksStatus.set(''); return }
        } catch { /* fall through */ }
      }

      // 2. Last.fm track.getSimilar
      const lastfmEnabled = SafeStorage.getItem('firmium_lastfm') === 'true'
      if (lastfmEnabled && track.artist && track.title) {
        const apiKey = lastfmEnabled ? ((await Keyring.load('lastfm_api_key').catch(() => '')) as string) || '' : ''
        if (apiKey) {
          try {
            const lfm = await fetchLastfmSimilarTracks(track.artist, track.title, apiKey)
            if (stale()) return
            if (lfm.length > 0) { similarTracksResults.set(lfm); similarTracksStatus.set(''); return }
          } catch { /* fall through */ }
        }
      }

      // 3. Genre/artist fallback
      const fallback = await Api.getSimilarTracksFallback(track.id, track.artistId, firstGenre(track), 10)
      if (stale()) return
      similarTracksResults.set(fallback)
      similarTracksStatus.set('')
    } catch (e) {
      if (get(similarTracksTrackId) === track.id) {
        similarTracksStatus.set('Failed to load similar tracks')
        console.error('Similar tracks error:', e)
      }
    }
  }

  function handleVolumeInput(e: Event) {
    // setVolume updates the store, localStorage, and calls set_queue_volume Rust command.
    setVolume(Number((e.target as HTMLInputElement).value))
  }

  function startSeek() { isSeeking.set(true) }
  // Fires on drag-move and on keyboard arrow keys; keeps the thumb and time label
  // tracking the in-progress seek without committing it yet.
  function onScrub(e: Event) {
    isSeeking.set(true)
    scrubPos = Number((e.target as HTMLInputElement).value)
  }
  async function endSeek(e: Event) {
    const target = Number((e.target as HTMLInputElement).value)
    currentPosition.set(target)
    scrubPos = null
    try { await tauriInvoke('seek_queue', { position: target }) } catch (err) { console.error('Seek failed:', err) }
    // Keep ignoring position updates briefly: a stale "playback-position" event
    // (reflecting the pre-seek position) may already be in flight from Rust.
    setTimeout(() => isSeeking.set(false), 300)
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
        <img bind:this={npCoverImg} class="np-cover-img" alt="{$currentTrack.album ? `${$currentTrack.album} — ${$currentTrack.artist}` : $currentTrack.artist ?? 'Album art'}" />
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
      value={scrubPos ?? seekValue}
      onmousedown={startSeek}
      ontouchstart={startSeek}
      oninput={onScrub}
      onchange={endSeek}
    />
    <span class="time right">{durDisplay}</span>
  </div>

  <div class="controls">
    <button class="ctrl-btn prev-ctrl" onclick={prevTrack} title="Previous" aria-label="Previous track">
      <span class="icon" style="width:15px;height:15px">{@html IconPrev}</span>
    </button>
    <button class="ctrl-btn main-ctrl" onclick={togglePlay} title="Play/Pause" aria-label={$playbackState === 'playing' ? 'Pause' : 'Play'}>
      <span class="icon" style="width:20px;height:20px">{@html playIcon}</span>
    </button>
    <button class="ctrl-btn" onclick={nextTrack} title="Next" aria-label="Next track">
      <span class="icon" style="width:15px;height:15px">{@html IconNext}</span>
    </button>
    <button
      class="ctrl-btn secondary-ctrl repeat-btn"
      class:active={$repeatOne || $repeatAll}
      onclick={cycleRepeat}
      title={$repeatOne ? 'Repeat One' : $repeatAll ? 'Repeat All' : 'Repeat Off'}
      aria-label={$repeatOne ? 'Repeat one track' : $repeatAll ? 'Repeat all tracks' : 'Repeat off'}
    >
      <span class="icon" style="width:16px;height:16px">{@html IconRepeat}</span>
      {#if $repeatOne}<span class="repeat-badge">1</span>{/if}
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$lyricsOpen} onclick={toggleLyrics} title="Lyrics" aria-label={$lyricsOpen ? 'Hide lyrics' : 'Show lyrics'}>
      <span class="icon" style="width:16px;height:16px">{@html IconLyrics}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$similarTracksOpen} onclick={toggleSimilarTracks} title="Similar Tracks" aria-label={$similarTracksOpen ? 'Hide similar tracks' : 'Show similar tracks'}>
      <span class="icon" style="width:16px;height:16px">{@html IconHexagon}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$visualizerOpen} onclick={() => visualizerOpen.update(v => !v)} title="Visualizer" aria-label={$visualizerOpen ? 'Hide visualizer' : 'Show visualizer'}>
      <span class="icon" style="width:16px;height:16px">{@html IconWaveform}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$audioStatsOpen} onclick={() => audioStatsOpen.update(v => !v)} title="Audio Stats" aria-label={$audioStatsOpen ? 'Hide audio stats' : 'Show audio stats'}>
      <span class="icon" style="width:16px;height:16px">{@html IconInfo}</span>
    </button>
  </div>
</div>
