<script lang="ts">
  import { onMount } from 'svelte'
  import { IconWaveform, IconPlay, IconLoading } from '../lib/icons'
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
    <select class="genre-select" bind:value={genre}>
      <option value="">Any genre</option>
      {#each genres as g}
        <option value={g.name}>{g.name}</option>
      {/each}
    </select>
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
    border: 1px solid var(--border); border-radius: 10px; background: var(--surface);
    cursor: pointer; text-align: left; transition: border-color 0.15s, background 0.15s;
  }
  .energy-card:hover { border-color: var(--accent_dim); }
  .energy-card.active { border-color: var(--accent); background: var(--surface2); }
  .energy-label { font-size: 16px; font-weight: 600; }
  .energy-desc { font-size: 13px; color: var(--muted); }
  .genre-select {
    width: 100%; padding: 10px 12px; border-radius: 8px;
    border: 1px solid var(--border); background: var(--surface); color: var(--text); font-size: 14px;
  }
  .start-btn {
    display: inline-flex; align-items: center; gap: 8px; padding: 12px 24px;
    border: none; border-radius: 999px; background: var(--accent); color: #fff;
    font-size: 15px; font-weight: 600; cursor: pointer;
  }
  .start-btn:disabled { opacity: 0.6; cursor: default; }
  .start-btn .icon { width: 18px; height: 18px; display: inline-flex; }
  .mix-message { margin-top: 16px; color: var(--muted); }
</style>
