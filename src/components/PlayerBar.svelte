<script lang="ts">
  import {
    currentTrack, playbackState, position, duration, queueState, settings, openPanel, togglePanel, toastError, volume,
  } from '../lib/stores'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { formatDuration } from '../lib/utils'
  import {
    IconPlay, IconPause, IconLoading, IconPrev, IconNext, IconRepeat, IconShuffle, IconLyrics, IconVolume,
    IconMusic, IconHexagon, IconInfo, IconQueue, IconEqualizer, IconHeart, IconHeartOutline,
  } from '../lib/icons'

  const run = (p: Promise<unknown>) => p.catch(toastError)

  const playIcon = $derived($playbackState === 'loading' ? IconLoading : $playbackState === 'playing' ? IconPause : IconPlay)
  const effectiveDuration = $derived($duration || $currentTrack?.duration || 0)
  const seekMax = $derived(effectiveDuration || 100)

  // While dragging the seek bar, show the dragged value instead of live position.
  let dragging = $state<number | null>(null)
  const seekValue = $derived(dragging ?? (effectiveDuration ? $position : 0))
  const progressPct = $derived((seekValue / seekMax) * 100)

  async function endSeek(e: Event) {
    const target = Number((e.target as HTMLInputElement).value)
    await run(Api.seek(target))
    setTimeout(() => (dragging = null), 300)
  }

  function cycleRepeat() {
    const { repeatOne, repeatAll } = $queueState
    if (!repeatOne && !repeatAll) run(Api.setRepeat(false, true))
    else if (repeatAll) run(Api.setRepeat(true, false))
    else run(Api.setRepeat(false, false))
  }

  let starred = $state(false)
  $effect(() => { starred = $currentTrack?.starred ?? false })
  async function toggleStar() {
    const t = $currentTrack
    if (!t) return
    starred = !starred
    try { await Api.setStarred(t.id, 'song', starred) } catch (e) { starred = !starred; toastError(e) }
  }
</script>

<div class="player-bar">
  <div class="now-playing">
    <div class="np-art">
      {#if $currentTrack?.coverArtId}
        <img use:cover={$currentTrack.coverArtId} class="np-cover-img" alt="{$currentTrack.album} by {$currentTrack.artist}" />
      {:else}
        <span class="icon" style="width:20px;height:20px;color:var(--muted)">{@html IconMusic}</span>
      {/if}
    </div>
    <div class="np-info">
      <div class="np-title">{$currentTrack?.title ?? '—'}</div>
      <div class="np-artist">{$currentTrack?.artist ?? 'No track selected'}</div>
      {#if $currentTrack?.trackInfo}<div class="np-format">{$currentTrack.trackInfo}</div>{/if}
    </div>
    {#if $currentTrack && !$currentTrack.id.startsWith('podcast:') && !$currentTrack.id.startsWith('local:')}
      <button class="ctrl-btn secondary-ctrl np-heart" class:active={starred} onclick={toggleStar} aria-label={starred ? 'Remove from favorites' : 'Add to favorites'} title="Favorite">
        <span class="icon" style="width:16px;height:16px">{@html starred ? IconHeart : IconHeartOutline}</span>
      </button>
    {/if}
    <div class="vol-row">
      <span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconVolume}</span>
      <input type="range" class="volume-slider" min="0" max="1" step="0.01" value={$volume}
        oninput={e => Api.setSetting('volume', Number((e.target as HTMLInputElement).value)).then(s => settings.set(s)).catch(() => {})} />
    </div>
  </div>

  <div class="progress-row">
    <span class="time">{formatDuration(seekValue)}</span>
    <input
      type="range" id="seekBar" style="--pct: {progressPct}%"
      min="0" max={seekMax} step="0.1" value={seekValue}
      oninput={e => (dragging = Number((e.target as HTMLInputElement).value))}
      onchange={endSeek}
    />
    <span class="time right">{formatDuration(effectiveDuration)}</span>
  </div>

  <div class="controls">
    <button class="ctrl-btn secondary-ctrl" class:active={$queueState.shuffleEnabled} onclick={() => run(Api.toggleShuffle())} title="Shuffle" aria-label="Toggle shuffle">
      <span class="icon" style="width:16px;height:16px">{@html IconShuffle}</span>
    </button>
    <button class="ctrl-btn prev-ctrl" onclick={() => run(Api.prev())} title="Previous" aria-label="Previous track">
      <span class="icon" style="width:15px;height:15px">{@html IconPrev}</span>
    </button>
    <button class="ctrl-btn main-ctrl" onclick={() => run(Api.togglePlay())} title="Play/Pause" aria-label={$playbackState === 'playing' ? 'Pause' : 'Play'}>
      <span class="icon" style="width:20px;height:20px">{@html playIcon}</span>
    </button>
    <button class="ctrl-btn" onclick={() => run(Api.next())} title="Next" aria-label="Next track">
      <span class="icon" style="width:15px;height:15px">{@html IconNext}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl repeat-btn" class:active={$queueState.repeatOne || $queueState.repeatAll} onclick={cycleRepeat}
      title={$queueState.repeatOne ? 'Repeat One' : $queueState.repeatAll ? 'Repeat All' : 'Repeat Off'} aria-label="Cycle repeat mode">
      <span class="icon" style="width:16px;height:16px">{@html IconRepeat}</span>
      {#if $queueState.repeatOne}<span class="repeat-badge">1</span>{/if}
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$openPanel === 'queue'} onclick={() => togglePanel('queue')} title="Queue" aria-label="Toggle queue">
      <span class="icon" style="width:16px;height:16px">{@html IconQueue}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$openPanel === 'lyrics'} onclick={() => togglePanel('lyrics')} title="Lyrics" aria-label="Toggle lyrics">
      <span class="icon" style="width:16px;height:16px">{@html IconLyrics}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$openPanel === 'similar'} onclick={() => togglePanel('similar')} title="Similar Tracks" aria-label="Toggle similar tracks">
      <span class="icon" style="width:16px;height:16px">{@html IconHexagon}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$openPanel === 'eq'} onclick={() => togglePanel('eq')} title="Equalizer" aria-label="Toggle equalizer">
      <span class="icon" style="width:16px;height:16px">{@html IconEqualizer}</span>
    </button>
    <button class="ctrl-btn secondary-ctrl" class:active={$openPanel === 'stats'} onclick={() => togglePanel('stats')} title="Audio Stats" aria-label="Toggle audio stats">
      <span class="icon" style="width:16px;height:16px">{@html IconInfo}</span>
    </button>
  </div>
</div>
