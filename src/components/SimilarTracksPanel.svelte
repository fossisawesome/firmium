<script lang="ts">
  import { app, toastError } from '../lib/stores.svelte'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { formatDuration } from '../lib/utils'
  import { IconClose } from '../lib/icons'
  import type { SimilarMatch, Song } from '../lib/types'

  let results = $state<SimilarMatch[]>([])
  let status = $state('')
  let loadedFor: string | null = null

  const open = $derived(app.openPanel === 'similar')

  async function load(song: Song) {
    loadedFor = song.id
    results = []
    status = 'Loading similar tracks…'
    try {
      const r = await Api.similarTracks(song, 10)
      if (loadedFor !== song.id) return
      results = r
      status = ''
    } catch {
      if (loadedFor === song.id) status = 'Failed to load similar tracks'
    }
  }

  $effect(() => {
    const t = app.currentTrack
    if (open && t && t.id !== loadedFor) load(t)
  })

  const play = (idx: number) => Api.setQueueSeamless(results.map(m => m.song), idx).catch(toastError)
</script>

<div class={["similar-tracks-panel", { open }]}>
  <div class="similar-tracks-header">
    <span class="similar-tracks-header-title">Similar Tracks</span>
    <button class="similar-tracks-close" onclick={() => (app.openPanel = null)} aria-label="Close similar tracks">
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">

  <div class="similar-tracks-body">
    {#if status}
      <div class="similar-tracks-status">{status}</div>
    {:else if results.length === 0}
      <div class="similar-tracks-status">{app.currentTrack ? 'No similar tracks found' : 'No track playing'}</div>
    {:else}
      <div class="track-list">
        {#each results as match, idx}
          <div class={["track-row", { playing: app.currentTrack?.id === match.song.id }]}
            role="button"
            tabindex="0"
            onclick={() => play(idx)}
            onkeydown={e => (e.key === 'Enter' || e.key === ' ') && play(idx)}>
            <div class="track-thumb">{#if match.song.coverArtId}<img {@attach cover(match.song.coverArtId)} alt="" />{/if}</div>
            <div class="track-info">
              <div class="track-title">{match.song.title}</div>
              <div class="track-artist">{match.song.artist}</div>
            </div>
            <div class="similar-tracks-match">{Math.round(match.similarity * 100)}%</div>
            <div class="track-duration">{formatDuration(match.song.duration)}</div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
