<script lang="ts">
  import { onMount } from 'svelte'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { toast, toastError, navTo } from '../lib/stores'
  import { errorMessage } from '../lib/ipc'
  import { IconBack, IconChevronRight } from '../lib/icons'
  import type { RecapRange, RecapStats } from '../lib/types'

  const RANGES: { id: RecapRange; label: string }[] = [
    { id: 'week', label: '7 days' }, { id: 'month', label: '30 days' }, { id: 'threeMonths', label: '3 months' },
    { id: 'year', label: '1 year' }, { id: 'all', label: 'All time' }, { id: 'custom', label: 'Custom' },
  ]
  const CARDS = 9
  const DAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']

  let range = $state<RecapRange>('month')
  let customFrom = $state('')
  let customTo = $state('')
  let stats = $state<RecapStats | null>(null)
  let card = $state(0)
  let error = $state('')

  const rangeLabel = $derived(RANGES.find(r => r.id === range)?.label.toLowerCase() ?? '')

  async function load() {
    error = ''
    card = 0
    try {
      if (range === 'custom') {
        if (!customFrom || !customTo) { stats = null; return }
        stats = await Api.recap('custom', Math.floor(new Date(customFrom).getTime() / 1000), Math.floor(new Date(customTo + 'T23:59:59').getTime() / 1000))
      } else {
        stats = await Api.recap(range)
      }
    } catch (e) { stats = null; error = errorMessage(e) }
  }

  onMount(load)

  const hours = (secs: number) => `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`
  const next = () => (card = Math.min(CARDS - 1, card + 1))
  const prev = () => (card = Math.max(0, card - 1))

  // Plain-text version of the current card, used to render the shareable PNG.
  const lines = $derived.by((): string[] => {
    const s = stats
    if (!s) return []
    switch (card) {
      case 0: return ['OVERVIEW', `${s.totalPlays} plays`, `${hours(s.totalSeconds)} of listening`, `over the last ${rangeLabel}`]
      case 1: return ['TOP TRACKS', ...s.topTracks.slice(0, 5).map((t, i) => `${i + 1}. ${t.title} (${t.count}x)`)]
      case 2: return ['TOP ARTISTS', ...s.topArtists.slice(0, 5).map((t, i) => `${i + 1}. ${t.name} (${t.count}x)`)]
      case 3: return ['TOP ALBUMS', ...s.topAlbums.slice(0, 5).map((t, i) => `${i + 1}. ${t.name} (${t.count}x)`)]
      case 4: return ['TOP GENRE', s.topGenre ? `${s.topGenre.genre} (${s.topGenre.count} plays)` : 'No genre data']
      case 5: return ['TIME OF DAY', `Morning ${s.byTimeOfDay.morning}`, `Afternoon ${s.byTimeOfDay.afternoon}`, `Evening ${s.byTimeOfDay.evening}`, `Night ${s.byTimeOfDay.night}`]
      case 6: return ['DAY OF WEEK', ...DAYS.map((d, i) => `${d} ${s.byDayOfWeek[i]}`)]
      case 7: return ['BIGGEST DISCOVERY', s.biggestDiscovery ? `${s.biggestDiscovery.title} (${s.biggestDiscovery.count} plays)` : 'No standout discovery']
      default: return ['STREAK', `${s.streak.daysActive} days active`, `Longest streak: ${s.streak.longestStreak} days`]
    }
  })

  async function saveCard() {
    const css = getComputedStyle(document.documentElement)
    const c = document.createElement('canvas')
    c.width = 1080; c.height = 1350
    const g = c.getContext('2d')!
    g.fillStyle = css.getPropertyValue('--bg').trim() || '#0f0f0f'
    g.fillRect(0, 0, c.width, c.height)
    g.fillStyle = css.getPropertyValue('--muted').trim() || '#a0a0a0'
    g.font = '36px monospace'
    g.fillText('FIRMIUM RECAP', 80, 120)
    lines.forEach((text, i) => {
      g.fillStyle = css.getPropertyValue(i === 0 ? '--muted' : i === 1 ? '--accent' : '--text').trim() || '#fff'
      g.font = i === 1 ? 'bold 72px monospace' : '44px monospace'
      g.fillText(text, 80, 260 + i * 100)
    })
    try {
      if (await Api.saveImage(`firmium-recap-${card + 1}.png`, c.toDataURL('image/png'))) toast('Recap card saved')
    } catch (e) { toastError(e) }
  }
</script>

<div class="recap-view">
  <div class="recap-header">
    <h1>Firmium Recap</h1>
    <div class="recap-ranges">
      {#each RANGES as r}
        <button class="filter-chip" class:active={range === r.id} onclick={() => { range = r.id; load() }}>{r.label}</button>
      {/each}
    </div>
  </div>

  {#if range === 'custom'}
    <div class="recap-custom">
      <input type="date" bind:value={customFrom} onchange={load} aria-label="From" />
      <span>to</span>
      <input type="date" bind:value={customTo} onchange={load} aria-label="To" />
    </div>
  {/if}

  {#if error}
    <div class="loading-msg error-msg">{error}</div>
  {:else if !stats || stats.totalPlays === 0}
    <div class="loading-msg">No listening history yet. Play some music and check back.</div>
  {:else}
    <div class="recap-card">
      {#if card === 0}
        <div class="recap-label">OVERVIEW</div>
        <div class="recap-big">{stats.totalPlays} plays</div>
        <div class="recap-sub">{hours(stats.totalSeconds)} of listening</div>
        <div class="recap-muted">over the last {rangeLabel}</div>
      {:else if card === 1}
        <div class="recap-label">TOP TRACKS</div>
        {#each stats.topTracks.slice(0, 5) as t, i (t.trackId)}
          <div class="recap-row"><span class="recap-rank">{i + 1}</span>
            <div class="recap-art">{#if t.coverArtId}<img use:cover={t.coverArtId} alt="" />{/if}</div>
            <div class="recap-main"><div>{t.title}</div><div class="recap-muted">{t.artist ?? ''}</div></div>
            <span class="recap-count">{t.count}×</span></div>
        {/each}
      {:else if card === 2}
        <div class="recap-label">TOP ARTISTS</div>
        {#each stats.topArtists.slice(0, 5) as a, i (a.name)}
          <div class="recap-row"><span class="recap-rank">{i + 1}</span><div class="recap-main">{a.name}</div><span class="recap-count">{a.count}×</span></div>
        {/each}
      {:else if card === 3}
        <div class="recap-label">TOP ALBUMS</div>
        {#each stats.topAlbums.slice(0, 5) as a, i (a.name + i)}
          <div class="recap-row"><span class="recap-rank">{i + 1}</span>
            <div class="recap-art">{#if a.coverArtId}<img use:cover={a.coverArtId} alt="" />{/if}</div>
            <div class="recap-main"><div>{a.name}</div><div class="recap-muted">{a.artist ?? ''}</div></div>
            <span class="recap-count">{a.count}×</span></div>
        {/each}
      {:else if card === 4}
        <div class="recap-label">TOP GENRE</div>
        {#if stats.topGenre}<div class="recap-big">{stats.topGenre.genre}</div><div class="recap-muted">{stats.topGenre.count} plays</div>
        {:else}<div class="recap-muted">No genre data</div>{/if}
      {:else if card === 5}
        <div class="recap-label">TIME OF DAY</div>
        <div class="recap-stat"><span>Morning (5–11)</span><b>{stats.byTimeOfDay.morning}</b></div>
        <div class="recap-stat"><span>Afternoon (12–16)</span><b>{stats.byTimeOfDay.afternoon}</b></div>
        <div class="recap-stat"><span>Evening (17–20)</span><b>{stats.byTimeOfDay.evening}</b></div>
        <div class="recap-stat"><span>Night (21–4)</span><b>{stats.byTimeOfDay.night}</b></div>
      {:else if card === 6}
        <div class="recap-label">DAY OF WEEK</div>
        {#each DAYS as d, i}<div class="recap-stat"><span>{d}</span><b>{stats.byDayOfWeek[i]}</b></div>{/each}
      {:else if card === 7}
        <div class="recap-label">BIGGEST DISCOVERY</div>
        {#if stats.biggestDiscovery}
          {@const d = stats.biggestDiscovery}
          <div class="recap-row">
            <div class="recap-art recap-art--lg">{#if d.coverArtId}<img use:cover={d.coverArtId} alt="" />{/if}</div>
            <div class="recap-main"><div class="recap-sub">{d.title}</div><div class="recap-muted">{d.artist ?? ''}</div><div class="recap-count">{d.count} plays since you found it</div></div>
          </div>
        {:else}<div class="recap-muted">No standout discovery this period</div>{/if}
      {:else}
        <div class="recap-label">STREAK</div>
        <div class="recap-big">{stats.streak.daysActive} days active</div>
        <div class="recap-count">Longest streak: {stats.streak.longestStreak} days</div>
      {/if}
    </div>

    <div class="recap-nav">
      <button class="ctrl-btn" onclick={prev} aria-label="Previous card"><span class="icon" style="width:16px;height:16px">{@html IconBack}</span></button>
      <div class="recap-dots">{#each Array(CARDS) as _, i}<span class="recap-dot" class:active={i === card}></span>{/each}</div>
      <button class="ctrl-btn" onclick={next} aria-label="Next card"><span class="icon" style="width:16px;height:16px">{@html IconChevronRight}</span></button>
    </div>
    <div class="recap-actions"><button class="debug-btn" onclick={saveCard}>Save card as image</button></div>
  {/if}
</div>
