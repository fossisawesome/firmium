<script lang="ts">
  import { app, togglePanel, toastError } from '../lib/stores.svelte'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { formatDuration } from '../lib/utils'
  import {
    IconPlay, IconPause, IconLoading, IconPrev, IconNext, IconRepeat, IconShuffle, IconLyrics, IconVolume,
    IconMusic, IconHexagon, IconInfo, IconQueue, IconMore, IconHeart, IconHeartOutline,
  } from '../lib/icons'

  const run = (p: Promise<unknown>) => p.catch(toastError)

  const playIcon = $derived(app.playbackState === 'loading' ? IconLoading : app.playbackState === 'playing' ? IconPause : IconPlay)
  const effectiveDuration = $derived(app.duration || app.currentTrack?.duration || 0)
  const seekMax = $derived(effectiveDuration || 100)

  // While dragging the seek bar, show the dragged value instead of live position.
  let dragging = $state<number | null>(null)
  const seekValue = $derived(dragging ?? (effectiveDuration ? app.position : 0))
  const progressPct = $derived((seekValue / seekMax) * 100)

  async function endSeek(e: Event) {
    const target = Number((e.target as HTMLInputElement).value)
    await run(Api.seek(target))
    setTimeout(() => (dragging = null), 300)
  }

  function cycleRepeat() {
    const { repeatOne, repeatAll } = app.queueState
    if (!repeatOne && !repeatAll) run(Api.setRepeat(false, true))
    else if (repeatAll) run(Api.setRepeat(true, false))
    else run(Api.setRepeat(false, false))
  }

  let starred = $state(false)
  $effect(() => { starred = app.currentTrack?.starred ?? false })
  async function toggleStar() {
    const t = app.currentTrack
    if (!t) return
    starred = !starred
    try { await Api.setStarred(t.id, 'song', starred) } catch (e) { starred = !starred; toastError(e) }
  }

  let moreOpen = $state(false)
  function pickPanel(p: 'similar' | 'stats') { togglePanel(p); moreOpen = false }
</script>
<svelte:window onclick={() => (moreOpen = false)} onkeydown={e => e.key === 'Escape' && (moreOpen = false)} />

<div class="player-bar">
  <div class="now-playing">
    <div class="np-art">
      {#if app.currentTrack?.coverArtId}
        <img {@attach cover(app.currentTrack.coverArtId)} class="np-cover-img" alt="{app.currentTrack.album} by {app.currentTrack.artist}" />
      {:else}
        <span class="icon" style="width:20px;height:20px;color:var(--muted)">{@html IconMusic}</span>
      {/if}
    </div>
    <div class="np-info">
      <div class="np-title">{app.currentTrack?.title ?? '—'}</div>
      <div class="np-artist">{app.currentTrack?.artist ?? 'No track selected'}</div>
      {#if app.currentTrack?.trackInfo}<div class="np-format">{app.currentTrack.trackInfo}</div>{/if}
    </div>
    {#if app.currentTrack && !app.currentTrack.id.startsWith('podcast:') && !app.currentTrack.id.startsWith('local:')}
      <button class={["ctrl-btn secondary-ctrl np-heart", { active: starred }]} onclick={toggleStar} aria-label={starred ? 'Remove from favorites' : 'Add to favorites'} title="Favorite">
        <span class="icon" style="width:16px;height:16px">{@html starred ? IconHeart : IconHeartOutline}</span>
      </button>
    {/if}
    <div class="vol-row">
      <span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconVolume}</span>
      <input type="range" class="volume-slider" min="0" max="1" step="0.01" value={app.volume}
        oninput={e => Api.setSetting('volume', Number((e.target as HTMLInputElement).value)).then(s => (app.settings = s)).catch(() => {})} />
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
    <button class={["ctrl-btn secondary-ctrl", { active: app.queueState.shuffleEnabled && app.queue.length > 0 }]} onclick={() => run(Api.toggleShuffle())} title="Shuffle" aria-label="Toggle shuffle">
      <span class="icon" style="width:16px;height:16px">{@html IconShuffle}</span>
    </button>
    <button class="ctrl-btn prev-ctrl" onclick={() => run(Api.prev())} title="Previous" aria-label="Previous track">
      <span class="icon" style="width:15px;height:15px">{@html IconPrev}</span>
    </button>
    <button class="ctrl-btn main-ctrl" onclick={() => run(Api.togglePlay())} title="Play/Pause" aria-label={app.playbackState === 'playing' ? 'Pause' : 'Play'}>
      <span class="icon" style="width:20px;height:20px">{@html playIcon}</span>
    </button>
    <button class="ctrl-btn" onclick={() => run(Api.next())} title="Next" aria-label="Next track">
      <span class="icon" style="width:15px;height:15px">{@html IconNext}</span>
    </button>
    <button class={["ctrl-btn secondary-ctrl repeat-btn", { active: app.queueState.repeatOne || app.queueState.repeatAll }]}
      onclick={cycleRepeat}
      title={app.queueState.repeatOne ? 'Repeat One' : app.queueState.repeatAll ? 'Repeat All' : 'Repeat Off'}
      aria-label="Cycle repeat mode">
      <span class="icon" style="width:16px;height:16px">{@html IconRepeat}</span>
      {#if app.queueState.repeatOne}<span class="repeat-badge">1</span>{/if}
    </button>
    <button class={["ctrl-btn secondary-ctrl", { active: app.openPanel === 'queue' }]} onclick={() => togglePanel('queue')} title="Queue" aria-label="Toggle queue">
      <span class="icon" style="width:16px;height:16px">{@html IconQueue}</span>
    </button>
    <button class={["ctrl-btn secondary-ctrl", { active: app.openPanel === 'lyrics' }]} onclick={() => togglePanel('lyrics')} title="Lyrics" aria-label="Toggle lyrics">
      <span class="icon" style="width:16px;height:16px">{@html IconLyrics}</span>
    </button>
    <div class="more-wrap">
      <button class={["ctrl-btn secondary-ctrl", { active: moreOpen || app.openPanel === 'similar' || app.openPanel === 'stats' }]}
        onclick={e => { e.stopPropagation(); moreOpen = !moreOpen }}
        title="More"
        aria-label="More options"
        aria-haspopup="menu"
        aria-expanded={moreOpen}>
        <span class="icon" style="width:16px;height:16px">{@html IconMore}</span>
      </button>
      {#if moreOpen}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="pl-popup pl-popup--visible more-menu" role="menu" tabindex="-1" onclick={e => e.stopPropagation()}>
          <button class="pl-popup-item" role="menuitem" onclick={() => pickPanel('similar')}>
            <span class="icon" style="width:14px;height:14px">{@html IconHexagon}</span><span class="pl-popup-name">Similar Tracks</span>
          </button>
          <button class="pl-popup-item" role="menuitem" onclick={() => pickPanel('stats')}>
            <span class="icon" style="width:14px;height:14px">{@html IconInfo}</span><span class="pl-popup-name">Audio Stats</span>
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>
