<script lang="ts">
  import { app } from '../lib/stores.svelte'
  import { IconClose } from '../lib/icons'
  import type { ReplayGain } from '../lib/types'

  function close() { (app.openPanel = null) }

  const rg = $derived((app.currentTrack?.replayGain ?? {}) as ReplayGain)

  function db(v: number | undefined): string {
    return v == null ? '—' : `${v > 0 ? '+' : ''}${v.toFixed(2)} dB`
  }
  function peak(v: number | undefined): string {
    return v == null ? '—' : v.toFixed(4)
  }

  const hasRg = $derived(
    rg.trackGain != null || rg.albumGain != null || rg.trackPeak != null || rg.albumPeak != null
  )
</script>

<div class={["audio-stats-panel", { open: app.openPanel === 'stats' }]}>
  <div class="audio-stats-header">
    <span class="audio-stats-header-title">Audio Stats</span>
    <button class="audio-stats-close" onclick={close}>
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">

  <div class="audio-stats-body">
    {#if !app.currentTrack}
      <div class="audio-stats-status">No track playing</div>
    {:else}
      <div class="audio-stats-group">
        <div class="audio-stats-label">Format</div>
        <div class="audio-stats-value">{app.currentTrack.trackInfo ?? '—'}</div>
      </div>

      <div class="audio-stats-group">
        <div class="audio-stats-label">Tempo</div>
        <div class="audio-stats-value">{app.currentTrack.bpm != null ? `${Math.round(app.currentTrack.bpm)} BPM` : '—'}</div>
      </div>

      <div class="audio-stats-group">
        <div class="audio-stats-label">ReplayGain</div>
        {#if hasRg}
          <table class="audio-stats-table">
            <tbody>
              <tr><td>Track gain</td><td>{db(rg.trackGain)}</td></tr>
              <tr><td>Track peak</td><td>{peak(rg.trackPeak)}</td></tr>
              <tr><td>Album gain</td><td>{db(rg.albumGain)}</td></tr>
              <tr><td>Album peak</td><td>{peak(rg.albumPeak)}</td></tr>
              {#if rg.baseGain != null}<tr><td>Base gain</td><td>{db(rg.baseGain)}</td></tr>{/if}
              {#if rg.fallbackGain != null}<tr><td>Fallback gain</td><td>{db(rg.fallbackGain)}</td></tr>{/if}
            </tbody>
          </table>
        {:else}
          <div class="audio-stats-value">Not provided by server</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style lang="scss">
  .audio-stats-group { margin-bottom: 18px; }
  .audio-stats-label {
    font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px;
    color: var(--muted); margin-bottom: 6px;
  }
  .audio-stats-value { font-size: 13px; color: var(--text); }
  .audio-stats-status { color: var(--muted); font-size: 12px; text-align: center; padding: 40px 0; }
  .audio-stats-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;

  td {
    padding: 4px 0;

    &:first-child {
      color: var(--muted);
    }

    &:last-child {
      text-align: right;
      font-variant-numeric: tabular-nums;
    }
  }
}
</style>
