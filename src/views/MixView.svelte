<script lang="ts">
  import { onMount } from 'svelte'
  import { IconWaveform, IconPlay, IconLoading, IconChevronDown } from '../lib/icons'
  import { Api, type Genre } from '../lib/api'
  import { tauriInvoke } from '../lib/tauri'
  import { buildMoodMix, type Energy } from '../lib/radio'
  import { createAbortController } from '../lib/utils'

  const ENERGIES: { id: Energy; label: string; desc: string }[] = [
    { id: 'chill', label: 'Chill', desc: 'Under 80 BPM' },
    { id: 'mid',   label: 'Mid',   desc: '80–120 BPM' },
    { id: 'high',  label: 'High',  desc: '120+ BPM' },
  ]

  let energy = $state<Energy>('mid')
  let genre = $state<string>('')
  let genres = $state<Genre[]>([])
  let building = $state(false)
  let message = $state('')
  let genreOpen = $state(false)

  const abortCtrl = createAbortController()

  onMount(() => {
    Api.getGenresList(abortCtrl.renew()).then(g => {
      genres = g.filter(x => x.songCount > 0).sort((a, b) => a.name.localeCompare(b.name))
    }).catch(() => {})
  })

  async function startMix() {
    if (building) return
    building = true
    message = ''
    try {
      const tracks = await buildMoodMix(energy, genre || undefined)
      if (!tracks.length) {
        message = 'No tracks matched that energy level. Try another band or genre.'
        return
      }
      await tauriInvoke('set_queue', { songs: tracks, startIdx: 0 })
    } catch (e) {
      console.error('Mix build failed:', e)
      message = 'Could not build a mix. Check your connection and try again.'
    } finally {
      building = false
    }
  }
</script>

<svelte:document onclick={() => { genreOpen = false }} />

<div class="mix-view">
  <header class="mix-header">
    <span class="mix-icon" aria-hidden="true">{@html IconWaveform}</span>
    <div>
      <h1>Mix</h1>
      <p>Generate a shuffled queue tuned to an energy level.</p>
    </div>
  </header>

  <section class="mix-section">
    <h2>Energy</h2>
    <div class="energy-grid">
      {#each ENERGIES as e}
        <button
          class="energy-card"
          class:active={energy === e.id}
          aria-pressed={energy === e.id}
          onclick={() => energy = e.id}
        >
          <span class="energy-label">{e.label}</span>
          <span class="energy-desc">{e.desc}</span>
        </button>
      {/each}
    </div>
  </section>

  <section class="mix-section">
    <h2>Genre <span class="optional">(optional)</span></h2>
    <div
      class="genre-selector"
      class:open={genreOpen}
      role="button"
      tabindex="0"
      onclick={e => { e.stopPropagation(); genreOpen = !genreOpen }}
      onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && (genreOpen = !genreOpen) }}
    >
      <div class="genre-selector-value">{genre || 'Any genre'}</div>
      <span class="genre-selector-arrow icon" style="width:14px;height:14px">{@html IconChevronDown}</span>
      <div class="genre-selector-dropdown">
        <div
          class="genre-option"
          class:selected={genre === ''}
          role="option"
          tabindex="0"
          aria-selected={genre === ''}
          onclick={e => { e.stopPropagation(); genre = ''; genreOpen = false }}
          onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && (genre = '', genreOpen = false) }}
        >Any genre</div>
        {#each genres as g}
          <div
            class="genre-option"
            class:selected={genre === g.name}
            role="option"
            tabindex="0"
            aria-selected={genre === g.name}
            onclick={e => { e.stopPropagation(); genre = g.name; genreOpen = false }}
            onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && (genre = g.name, genreOpen = false) }}
          >{g.name}</div>
        {/each}
      </div>
    </div>
  </section>

  <button class="start-btn" onclick={startMix} disabled={building}>
    <span class="icon" aria-hidden="true">{@html building ? IconLoading : IconPlay}</span>
    {building ? 'Building…' : 'Start Mix'}
  </button>

  {#if message}<p class="mix-message">{message}</p>{/if}
</div>

<style>
  .mix-view { padding: 24px 32px; max-width: 720px; }
  .mix-header { display: flex; align-items: center; gap: 16px; margin-bottom: 28px; }
  .mix-icon { width: 40px; height: 40px; color: var(--accent); flex: none; }
  .mix-header h1 { margin: 0; font-size: 28px; }
  .mix-header p { margin: 4px 0 0; color: var(--muted); }
  .mix-section { margin-bottom: 24px; }
  .mix-section h2 { font-size: 15px; margin: 0 0 12px; color: var(--text); }
  .optional { color: var(--muted); font-weight: 400; }
  .energy-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
  .energy-card {
    display: flex; flex-direction: column; gap: 4px; padding: 16px;
    border: 1px solid var(--border); border-radius: 4px; background: var(--surface);
    cursor: pointer; text-align: left; transition: border-color 0.15s, background 0.15s;
  }
  .energy-card:hover { border-color: var(--accent_dim); }
  .energy-card.active { border-color: var(--accent); background: var(--surface2); }
  .energy-label { font-size: 16px; font-weight: 600; }
  .energy-desc { font-size: 13px; color: var(--muted); }
  .genre-selector {
    position: relative; width: 280px;
    background: var(--bg); border: 1px solid var(--border); color: var(--text);
    padding: 10px 12px; font-family: var(--font); font-size: 13px;
    border-radius: 2px; cursor: pointer;
    display: flex; align-items: center; justify-content: space-between;
    user-select: none; transition: border-color var(--timing);
  }
  .genre-selector:hover, .genre-selector.open { border-color: var(--accent); }
  .genre-selector-arrow { color: var(--muted); transition: transform var(--timing); }
  .genre-selector.open .genre-selector-arrow { transform: rotate(180deg); }
  .genre-selector-dropdown {
    display: none; position: absolute; top: calc(100% + 4px); left: -1px; right: -1px;
    background: var(--surface); border: 1px solid var(--border); border-radius: 2px;
    z-index: 100; overflow-y: auto; max-height: 240px;
  }
  .genre-selector.open .genre-selector-dropdown { display: block; }
  .genre-option {
    padding: 9px 12px; font-family: var(--font); font-size: 13px;
    color: var(--text); cursor: pointer; transition: background var(--timing);
  }
  .genre-option:hover { background: var(--surface2); }
  .genre-option.selected { color: var(--accent); }
  .start-btn {
    display: inline-flex; align-items: center; gap: 8px; padding: 12px 24px;
    border: none; border-radius: 4px; background: var(--accent); color: #fff;
    font-size: 15px; font-weight: 600; cursor: pointer;
  }
  .start-btn:disabled { opacity: 0.6; cursor: default; }
  .start-btn .icon { width: 18px; height: 18px; display: inline-flex; }
  .mix-message { margin-top: 16px; color: var(--muted); }
</style>
