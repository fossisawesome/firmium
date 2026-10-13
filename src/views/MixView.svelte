<script lang="ts">
  import { onMount } from 'svelte'
  import { IconWaveform, IconPlay, IconLoading } from '../lib/icons'
  import { Api } from '../lib/api'
  import { errorMessage } from '../lib/ipc'
  import type { Energy, Genre } from '../lib/types'

  type Choice = Energy | 'any'
  const ENERGIES: { id: Choice; label: string; desc: string }[] = [
    { id: 'chill', label: 'Chill', desc: 'Under 95 BPM' },
    { id: 'mid', label: 'Mid', desc: '95–130 BPM' },
    { id: 'high', label: 'High', desc: '130+ BPM' },
    { id: 'any', label: 'Any', desc: 'Whole library' },
  ]

  let energy = $state<Choice>('mid')
  let genre = $state('')
  let genres = $state<Genre[]>([])
  let building = $state(false)
  let message = $state('')

  onMount(() => {
    Api.genres().then(g => { genres = g.filter(x => x.songCount > 0).sort((a, b) => a.name.localeCompare(b.name)) }).catch(() => {})
  })

  async function startMix() {
    if (building) return
    building = true
    message = ''
    try {
      if (energy === 'any') {
        if ((await Api.shuffleAll()) === 0) message = 'No songs found to shuffle.'
      } else if ((await Api.generateMix(energy, genre || undefined)) === 0) message = 'No tracks matched that energy level. Try another band or genre.'
    } catch (e) { message = errorMessage(e) }
    finally { building = false }
  }
</script>

<div class="mix-view">
  <header class="mix-header">
    <span class="mix-icon" aria-hidden="true">{@html IconWaveform}</span>
    <div>
      <h1>Mix</h1>
      <p>Generate a shuffled queue tuned to an energy level, or shuffle your whole library.</p>
    </div>
  </header>

  <section class="mix-section">
    <h2>Energy</h2>
    <div class="energy-grid">
      {#each ENERGIES as e}
        <button class={["energy-card", { active: energy === e.id }]} aria-pressed={energy === e.id} onclick={() => (energy = e.id)}>
          <span class="energy-label">{e.label}</span>
          <span class="energy-desc">{e.desc}</span>
        </button>
      {/each}
    </div>
  </section>

  {#if energy !== 'any'}
  <section class="mix-section">
    <h2>Genre <span class="optional">(optional)</span></h2>
    <select class="genre-select" bind:value={genre}>
      <option value="">Any genre</option>
      {#each genres as g}<option value={g.name}>{g.name}</option>{/each}
    </select>
  </section>
  {/if}

  <button class="start-btn" onclick={startMix} disabled={building}>
    <span class="icon" aria-hidden="true">{@html building ? IconLoading : IconPlay}</span>
    {building ? 'Building…' : energy === 'any' ? 'Shuffle all' : 'Start Mix'}
  </button>

  {#if message}<p class="mix-message">{message}</p>{/if}
</div>

<style lang="scss">
  .mix-view { padding: 24px 32px; max-width: 720px; }
  .mix-header { display: flex; align-items: center; gap: 16px; margin-bottom: 28px; }
  .mix-icon { width: 40px; height: 40px; color: var(--accent); flex: none; }
  .mix-header h1 { margin: 0; font-size: 28px; }
  .mix-header p { margin: 4px 0 0; color: var(--muted); }
  .mix-section {
  margin-bottom: 24px;

  h2 {
    font-size: 15px;
    margin: 0 0 12px;
    color: var(--text);
  }
}
  .optional { color: var(--muted); font-weight: 400; }
  .energy-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; }
  .energy-card {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--surface);
  color: var(--text);
  cursor: pointer;
  text-align: left;
  font-family: var(--font);
  transition: border-color 0.15s, background 0.15s;

  &:hover {
    border-color: var(--accent-dim);
  }

  &.active {
    border-color: var(--accent);
    background: var(--surface2);
  }
}
  .energy-label { font-size: 16px; font-weight: 600; }
  .energy-desc { font-size: 13px; color: var(--muted); }
  .genre-select { width: 100%; padding: 10px 12px; border-radius: 3px; border: 1px solid var(--border); background: var(--surface); color: var(--text); font-family: var(--font); font-size: 14px; }
  .start-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 12px 24px;
  border: none;
  border-radius: 3px;
  background: var(--accent);
  color: var(--bg);
  font-family: var(--font);
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;

  &:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .icon {
    width: 18px;
    height: 18px;
    display: inline-flex;
  }
}
  .mix-message { margin-top: 16px; color: var(--muted); }
</style>
