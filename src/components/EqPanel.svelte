<script lang="ts">
  import { openPanel } from '../lib/stores'
  import { IconClose } from '../lib/icons'
  import EqualizerSettings from './EqualizerSettings.svelte'
</script>

<div class="similar-tracks-panel eq-panel" class:open={$openPanel === 'eq'}>
  <div class="similar-tracks-header">
    <span class="similar-tracks-header-title">Equalizer</span>
    <button class="similar-tracks-close" onclick={() => openPanel.set(null)} aria-label="Close equalizer">
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">
  <div class="similar-tracks-body">
    {#if $openPanel === 'eq'}<EqualizerSettings />{/if}
  </div>
</div>
