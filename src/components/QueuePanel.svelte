<script lang="ts">
  import { app, toastError } from '../lib/stores.svelte'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { formatDuration } from '../lib/utils'
  import { IconClose } from '../lib/icons'

  const open = $derived(app.openPanel === 'queue')
  const play = (i: number) => Api.playIndex(i).catch(toastError)
</script>

<div class={["similar-tracks-panel", { open }]}>
  <div class="similar-tracks-header">
    <span class="similar-tracks-header-title">Queue</span>
    <button class="similar-tracks-close" onclick={() => (app.openPanel = null)} aria-label="Close queue">
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">

  <div class="similar-tracks-body">
    {#if app.queue.length === 0}
      <div class="similar-tracks-status">Queue is empty</div>
    {:else}
      <div class="track-list">
        {#each app.queue as song, i (i + song.id)}
          <div class={["track-row", { playing: i === app.queueIdx }]}
            role="button"
            tabindex="0"
            onclick={() => play(i)}
            onkeydown={e => (e.key === 'Enter' || e.key === ' ') && play(i)}>
            <div class="track-thumb">{#if song.coverArtId}<img {@attach cover(song.coverArtId)} alt="" />{/if}</div>
            <div class="track-info">
              <div class="track-title">{song.title}</div>
              <div class="track-artist">{song.artist}</div>
            </div>
            <div class="track-duration">{formatDuration(song.duration)}</div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
