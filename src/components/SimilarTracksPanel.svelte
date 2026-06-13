<script lang="ts">
  import { similarTracksOpen, similarTracksResults, similarTracksStatus, currentTrack, queue } from '../lib/stores'
  import { playAt } from '../lib/playback'
  import { loadImage } from '../lib/api'
  import { lazyLoad } from '../lib/lazyLoad'
  import { formatDuration, createAbortController } from '../lib/utils'
  import { IconClose } from '../lib/icons'

  const abortCtrl = createAbortController()

  function close() {
    similarTracksOpen.set(false)
  }

  function playMatch(idx: number) {
    queue.set($similarTracksResults.map(m => m.song))
    playAt(idx)
  }

  function isPlaying(songId: string) {
    return $currentTrack?.id === songId
  }
</script>

<div class="similar-tracks-panel" class:open={$similarTracksOpen}>
  <div class="similar-tracks-safe-top"></div>
  <div class="similar-tracks-header">
    <span class="similar-tracks-header-title">Similar Tracks</span>
    <button class="similar-tracks-close" onclick={close}>
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">

  <div class="similar-tracks-body">
    {#if $similarTracksStatus}
      <div class="similar-tracks-status">{$similarTracksStatus}</div>
    {:else if $similarTracksResults.length === 0}
      <div class="similar-tracks-status">No similar tracks found</div>
    {:else}
      <div class="track-list">
        {#each $similarTracksResults as match, idx}
          <div
            class="track-row"
            class:playing={isPlaying(match.song.id)}
            role="button"
            tabindex="0"
            onclick={() => playMatch(idx)}
            onkeydown={e => (e.key === 'Enter' || e.key === ' ') && playMatch(idx)}
          >
            <div class="track-thumb">
              {#if match.song.coverArtId}
                <img use:lazyLoad={img => loadImage(img, match.song.coverArtId, abortCtrl.signal)} alt="" />
              {/if}
            </div>
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
