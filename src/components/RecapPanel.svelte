<script lang="ts">
  import { recapOpen } from '../lib/stores'
  import { loadImage } from '../lib/api'
  import { IconClose, IconChevronRight, IconBack, IconShare, IconDownload } from '../lib/icons'
  import {
    getRecapStats, rangeToBounds, RANGE_OPTIONS, formatDuration, dayOfWeekLabel,
    type RecapStats, type RangeId,
  } from '../lib/stats'
  import { saveNodeAsImage } from '../lib/exportFile'

  let range = $state<RangeId>('30d')
  let customFrom = $state('')
  let customTo = $state('')
  let stats = $state<RecapStats | null>(null)
  let loading = $state(false)
  let error = $state('')
  let cardIndex = $state(0)
  let deck = $state<HTMLDivElement | undefined>()
  let summaryCard = $state<HTMLDivElement | undefined>()
  let saving = $state(false)

  // (Re)load stats whenever the panel opens or the range changes.
  $effect(() => {
    if (!$recapOpen) return
    void load(range, customFrom, customTo)
  })

  async function load(r: RangeId, cf: string, ct: string) {
    loading = true
    error = ''
    try {
      let from: number, to: number
      if (r === 'custom') {
        const f = cf ? Math.floor(new Date(cf).getTime() / 1000) : 0
        const t = ct ? Math.floor(new Date(ct).getTime() / 1000) + 86400 : Math.floor(Date.now() / 1000)
        ;[from, to] = [f, t]
      } else {
        ;[from, to] = rangeToBounds(r)
      }
      stats = await getRecapStats(from, to)
      cardIndex = 0
      deck?.scrollTo({ left: 0 })
    } catch (e) {
      error = String(e)
      stats = null
    } finally {
      loading = false
    }
  }

  function close() { recapOpen.set(false) }

  // Cover-art loader action — same path as the rest of the app (api.ts::loadImage).
  function cover(node: HTMLImageElement, coverId: string | null) {
    if (coverId) loadImage(node, coverId)
    return { update(id: string | null) { if (id) loadImage(node, id) } }
  }

  // ── Horizontal pager ──────────────────────────────────────────────────────
  const cardCount = 9
  function onScroll() {
    if (!deck) return
    cardIndex = Math.round(deck.scrollLeft / deck.clientWidth)
  }
  function goTo(i: number) {
    if (!deck) return
    const clamped = Math.max(0, Math.min(cardCount - 1, i))
    deck.scrollTo({ left: clamped * deck.clientWidth, behavior: 'smooth' })
  }

  async function saveCurrentCard() {
    const card = deck?.querySelectorAll<HTMLElement>('.recap-card')[cardIndex]
    if (!card) return
    saving = true
    try { await saveNodeAsImage(card, `firmium-recap-card-${cardIndex + 1}.png`) }
    catch (e) { error = String(e) }
    finally { saving = false }
  }

  async function saveFullRecap() {
    if (!summaryCard) return
    saving = true
    try { await saveNodeAsImage(summaryCard, 'firmium-recap.png') }
    catch (e) { error = String(e) }
    finally { saving = false }
  }

  function dateStr(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
  }

  const tod = $derived(stats?.byTimeOfDay)
  const todMax = $derived(tod ? Math.max(tod.morning, tod.afternoon, tod.evening, tod.night, 1) : 1)
  const dowMax = $derived(stats ? Math.max(...stats.byDayOfWeek, 1) : 1)
</script>

<div class="recap-panel" class:open={$recapOpen}>
  <div class="recap-safe-top"></div>

  <div class="recap-header">
    <span class="recap-title">Firmium Recap</span>
    <button class="recap-icon-btn" onclick={close} aria-label="Close recap">
      <span class="icon" style="width:14px;height:14px">{@html IconClose}</span>
    </button>
  </div>

  <div class="recap-ranges">
    {#each RANGE_OPTIONS as opt}
      <button
        class="recap-range-btn"
        class:active={range === opt.id}
        onclick={() => range = opt.id}
      >{opt.label}</button>
    {/each}
  </div>
  {#if range === 'custom'}
    <div class="recap-custom">
      <input type="date" bind:value={customFrom} aria-label="From date" />
      <span>to</span>
      <input type="date" bind:value={customTo} aria-label="To date" />
    </div>
  {/if}

  {#if loading}
    <div class="recap-status">Crunching your listening…</div>
  {:else if error}
    <div class="recap-status">{error}</div>
  {:else if !stats || stats.totalPlays === 0}
    <div class="recap-status">No plays recorded in this range yet. Listen to some music and check back.</div>
  {:else}
    <div class="recap-deck" bind:this={deck} onscroll={onScroll}>
      <!-- 1: total time -->
      <section class="recap-card">
        <div class="recap-kicker">You listened for</div>
        <div class="recap-hero">{formatDuration(stats.totalSeconds)}</div>
        <div class="recap-sub">{stats.totalPlays} tracks played</div>
      </section>

      <!-- 2: top tracks -->
      <section class="recap-card">
        <div class="recap-card-title">Top Tracks</div>
        <ol class="recap-list">
          {#each stats.topTracks.slice(0, 5) as t, i}
            <li>
              <span class="recap-rank">{i + 1}</span>
              <img class="recap-thumb" use:cover={t.coverArtId} alt="" />
              <span class="recap-names">
                <span class="recap-name">{t.title}</span>
                <span class="recap-meta">{t.artist ?? ''}</span>
              </span>
              <span class="recap-count">{t.count}</span>
            </li>
          {/each}
        </ol>
      </section>

      <!-- 3: top artists -->
      <section class="recap-card">
        <div class="recap-card-title">Top Artists</div>
        <ol class="recap-list">
          {#each stats.topArtists.slice(0, 5) as a, i}
            <li>
              <span class="recap-rank">{i + 1}</span>
              <span class="recap-names">
                <span class="recap-name">{a.name}</span>
              </span>
              <span class="recap-count">{a.count}</span>
            </li>
          {/each}
        </ol>
      </section>

      <!-- 4: top albums -->
      <section class="recap-card">
        <div class="recap-card-title">Top Albums</div>
        <ol class="recap-list">
          {#each stats.topAlbums.slice(0, 5) as a, i}
            <li>
              <span class="recap-rank">{i + 1}</span>
              <img class="recap-thumb" use:cover={a.coverArtId} alt="" />
              <span class="recap-names">
                <span class="recap-name">{a.name}</span>
                <span class="recap-meta">{a.artist ?? ''}</span>
              </span>
              <span class="recap-count">{a.count}</span>
            </li>
          {/each}
        </ol>
      </section>

      <!-- 5: top genre -->
      <section class="recap-card">
        <div class="recap-kicker">Your sound was</div>
        {#if stats.topGenre}
          <div class="recap-hero recap-hero--accent">{stats.topGenre.genre}</div>
          <div class="recap-sub">{stats.topGenre.count} plays</div>
        {:else}
          <div class="recap-sub">No genre data</div>
        {/if}
      </section>

      <!-- 6: time of day -->
      <section class="recap-card">
        <div class="recap-card-title">By Time of Day</div>
        <div class="recap-bars">
          {#each [['Morning', tod?.morning ?? 0], ['Afternoon', tod?.afternoon ?? 0], ['Evening', tod?.evening ?? 0], ['Night', tod?.night ?? 0]] as [label, val]}
            <div class="recap-bar-row">
              <span class="recap-bar-label">{label}</span>
              <div class="recap-bar-track">
                <div class="recap-bar-fill" style="width: {(Number(val) / todMax) * 100}%"></div>
              </div>
              <span class="recap-bar-val">{val}</span>
            </div>
          {/each}
        </div>
      </section>

      <!-- 7: day of week -->
      <section class="recap-card">
        <div class="recap-card-title">By Day of Week</div>
        <div class="recap-dow">
          {#each stats.byDayOfWeek as val, i}
            <div class="recap-dow-col">
              <div class="recap-dow-bar" style="height: {(val / dowMax) * 100}%"></div>
              <span class="recap-dow-label">{dayOfWeekLabel(i)}</span>
            </div>
          {/each}
        </div>
      </section>

      <!-- 8: biggest discovery -->
      <section class="recap-card">
        <div class="recap-card-title">Biggest Discovery</div>
        {#if stats.biggestDiscovery}
          <img class="recap-discovery-art" use:cover={stats.biggestDiscovery.coverArtId} alt="" />
          <div class="recap-name recap-name--big">{stats.biggestDiscovery.title}</div>
          <div class="recap-meta">{stats.biggestDiscovery.artist ?? ''}</div>
          <div class="recap-sub">{stats.biggestDiscovery.count} plays · first heard {dateStr(stats.biggestDiscovery.firstHeard)}</div>
        {:else}
          <div class="recap-sub">Not enough plays yet</div>
        {/if}
      </section>

      <!-- 9: streak -->
      <section class="recap-card">
        <div class="recap-kicker">Longest streak</div>
        <div class="recap-hero">{stats.streak.longestStreak}<span class="recap-hero-unit"> days</span></div>
        <div class="recap-sub">{stats.streak.daysActive} days with music in this range</div>
      </section>
    </div>

    <div class="recap-footer">
      <button class="recap-icon-btn" onclick={() => goTo(cardIndex - 1)} disabled={cardIndex === 0} aria-label="Previous card">
        <span class="icon" style="width:16px;height:16px">{@html IconBack}</span>
      </button>
      <div class="recap-dots">
        {#each Array(cardCount) as _, i}
          <button class="recap-dot" class:active={i === cardIndex} onclick={() => goTo(i)} aria-label="Card {i + 1}"></button>
        {/each}
      </div>
      <button class="recap-icon-btn" onclick={() => goTo(cardIndex + 1)} disabled={cardIndex === cardCount - 1} aria-label="Next card">
        <span class="icon" style="width:16px;height:16px">{@html IconChevronRight}</span>
      </button>
    </div>

    <div class="recap-actions">
      <button class="recap-action" onclick={saveCurrentCard} disabled={saving}>
        <span class="icon" style="width:14px;height:14px">{@html IconDownload}</span> Save card
      </button>
      <button class="recap-action" onclick={saveFullRecap} disabled={saving}>
        <span class="icon" style="width:14px;height:14px">{@html IconShare}</span> Save recap
      </button>
    </div>

    <!-- Off-screen shareable summary captured by "Save recap". -->
    <div class="recap-summary" bind:this={summaryCard} aria-hidden="true">
      <div class="recap-summary-brand">Firmium Recap</div>
      <div class="recap-summary-time">{formatDuration(stats.totalSeconds)}</div>
      <div class="recap-summary-sub">{stats.totalPlays} tracks · {stats.streak.daysActive} active days</div>
      <div class="recap-summary-grid">
        {#if stats.topTracks[0]}
          <div class="recap-summary-item"><span>Top track</span><strong>{stats.topTracks[0].title}</strong></div>
        {/if}
        {#if stats.topArtists[0]}
          <div class="recap-summary-item"><span>Top artist</span><strong>{stats.topArtists[0].name}</strong></div>
        {/if}
        {#if stats.topAlbums[0]}
          <div class="recap-summary-item"><span>Top album</span><strong>{stats.topAlbums[0].name}</strong></div>
        {/if}
        {#if stats.topGenre}
          <div class="recap-summary-item"><span>Top genre</span><strong>{stats.topGenre.genre}</strong></div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .recap-panel {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: var(--bg);
    color: var(--text);
    display: flex;
    flex-direction: column;
    transform: translateY(100%);
    transition: transform var(--timing) ease;
    pointer-events: none;
  }
  .recap-panel.open { transform: translateY(0); pointer-events: auto; }

  .recap-safe-top { height: env(safe-area-inset-top, 0); }

  .recap-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 8px;
  }
  .recap-title { font-size: 18px; font-weight: 700; letter-spacing: 0.5px; }

  .recap-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }
  .recap-icon-btn:disabled { opacity: 0.35; cursor: default; }

  .recap-ranges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 20px 8px;
    justify-content: center;
  }
  .recap-range-btn {
    padding: 5px 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
  }
  .recap-range-btn.active { background: var(--accent); color: var(--bg); border-color: var(--accent); }

  .recap-custom {
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: center;
    padding: 0 20px 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .recap-custom input {
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 6px;
    padding: 4px 8px;
  }

  .recap-status {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 40px;
    color: var(--muted);
  }

  .recap-deck {
    flex: 1;
    display: flex;
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
  }
  .recap-deck::-webkit-scrollbar { display: none; }

  .recap-card {
    flex: 0 0 100%;
    scroll-snap-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px 28px;
    box-sizing: border-box;
    text-align: center;
  }

  .recap-kicker { font-size: 14px; color: var(--muted); text-transform: uppercase; letter-spacing: 1px; }
  .recap-hero {
    font-size: clamp(48px, 14vw, 96px);
    font-weight: 800;
    line-height: 1;
    background: linear-gradient(135deg, var(--accent), var(--text));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .recap-hero--accent { background: linear-gradient(135deg, var(--accent), #863bff); -webkit-background-clip: text; background-clip: text; }
  .recap-hero-unit { font-size: 0.35em; font-weight: 600; color: var(--muted); -webkit-text-fill-color: var(--muted); }
  .recap-sub { font-size: 14px; color: var(--muted); }
  .recap-card-title { font-size: 22px; font-weight: 700; margin-bottom: 8px; }

  .recap-list { list-style: none; margin: 0; padding: 0; width: 100%; max-width: 460px; display: flex; flex-direction: column; gap: 12px; }
  .recap-list li { display: flex; align-items: center; gap: 12px; text-align: left; }
  .recap-rank { font-size: 20px; font-weight: 800; color: var(--accent); width: 24px; flex: 0 0 auto; }
  .recap-thumb { width: 44px; height: 44px; border-radius: 6px; object-fit: cover; background: var(--surface2); flex: 0 0 auto; }
  .recap-names { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .recap-name { font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .recap-name--big { font-size: 20px; margin-top: 12px; }
  .recap-meta { font-size: 12px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .recap-count { font-variant-numeric: tabular-nums; color: var(--muted); flex: 0 0 auto; }

  .recap-bars { width: 100%; max-width: 460px; display: flex; flex-direction: column; gap: 14px; }
  .recap-bar-row { display: flex; align-items: center; gap: 10px; }
  .recap-bar-label { width: 80px; text-align: left; font-size: 13px; flex: 0 0 auto; }
  .recap-bar-track { flex: 1; height: 12px; background: var(--surface2); border-radius: 999px; overflow: hidden; }
  .recap-bar-fill { height: 100%; background: var(--accent); border-radius: 999px; }
  .recap-bar-val { width: 36px; text-align: right; font-variant-numeric: tabular-nums; color: var(--muted); flex: 0 0 auto; }

  .recap-dow { display: flex; align-items: flex-end; gap: 10px; height: 220px; width: 100%; max-width: 460px; }
  .recap-dow-col { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: flex-end; height: 100%; gap: 6px; }
  .recap-dow-bar { width: 100%; min-height: 3px; background: var(--accent); border-radius: 6px 6px 0 0; }
  .recap-dow-label { font-size: 11px; color: var(--muted); }

  .recap-discovery-art { width: 160px; height: 160px; border-radius: 12px; object-fit: cover; background: var(--surface2); }

  .recap-footer { display: flex; align-items: center; justify-content: center; gap: 16px; padding: 8px 20px; }
  .recap-dots { display: flex; gap: 8px; }
  .recap-dot { width: 8px; height: 8px; border-radius: 50%; border: none; background: var(--surface2); cursor: pointer; padding: 0; }
  .recap-dot.active { background: var(--accent); }

  .recap-actions { display: flex; gap: 10px; justify-content: center; padding: 4px 20px calc(16px + env(safe-area-inset-bottom, 0)); }
  .recap-action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
  }
  .recap-action:disabled { opacity: 0.5; cursor: default; }

  /* Off-screen capture target for the shareable summary image. */
  .recap-summary {
    position: absolute;
    left: -9999px;
    top: 0;
    width: 540px;
    box-sizing: border-box;
    padding: 48px 40px;
    background: var(--bg);
    color: var(--text);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .recap-summary-brand { font-size: 16px; letter-spacing: 1px; color: var(--accent); text-transform: uppercase; }
  .recap-summary-time { font-size: 64px; font-weight: 800; line-height: 1.1; }
  .recap-summary-sub { color: var(--muted); margin-bottom: 20px; }
  .recap-summary-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .recap-summary-item { display: flex; flex-direction: column; gap: 2px; }
  .recap-summary-item span { font-size: 12px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .recap-summary-item strong { font-size: 18px; }
</style>
