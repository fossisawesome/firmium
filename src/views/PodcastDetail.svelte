<script lang="ts">
  import { onMount } from 'svelte'
  import { Api } from '../lib/api'
  import { navTo, currentTrack, toastError, toast } from '../lib/stores'
  import { formatDuration } from '../lib/utils'
  import { IconPlay, IconRefresh } from '../lib/icons'
  import type { PodcastChannel, PodcastEpisode } from '../lib/types'

  let { id }: { id: string } = $props()

  let channel = $state<PodcastChannel | null>(null)
  let episodes = $state<PodcastEpisode[]>([])
  let refreshing = $state(false)

  async function loadEpisodes() {
    episodes = await Api.podcastEpisodes(id).catch(() => [])
  }

  onMount(async () => {
    channel = (await Api.podcastChannels().catch(() => [])).find(c => c.id === id) ?? null
    await loadEpisodes()
  })

  async function refresh() {
    if (!channel || refreshing) return
    refreshing = true
    try {
      const n = await Api.refreshPodcast(channel.id, channel.feedUrl)
      await loadEpisodes()
      toast(n > 0 ? `${n} new episode${n === 1 ? '' : 's'}` : 'Up to date')
    } catch (e) { toastError(e) }
    finally { refreshing = false }
  }

  async function unsubscribe() {
    if (!channel || !confirm(`Unsubscribe from "${channel.title}"?`)) return
    try { await Api.unsubscribePodcast(channel.id); navTo('podcasts') } catch (e) { toastError(e) }
  }

  const play = (ep: PodcastEpisode) => Api.playPodcastEpisode(ep, channel?.title ?? '', channel?.imageUrl).catch(toastError)
</script>

<div class="pl-list-header">
  <span class="section-header" style="margin:0">{channel?.title ?? 'Podcast'}</span>
  <div class="pl-detail-actions">
    <button class="debug-btn" onclick={refresh} disabled={refreshing}>
      <span class="icon" class:icon-spin={refreshing} style="width:12px;height:12px;display:inline-flex;margin-right:6px">{@html IconRefresh}</span>Refresh
    </button>
    <button class="debug-btn debug-btn--danger" onclick={unsubscribe}>Unsubscribe</button>
  </div>
</div>

{#if episodes.length === 0}
  <div class="loading-msg">No episodes found.</div>
{:else}
  <div class="track-list">
    {#each episodes as ep (ep.id)}
      <div class="track-row" class:playing={$currentTrack?.id === `podcast:${ep.id}`} role="button" tabindex="0"
        onclick={() => play(ep)} onkeydown={e => (e.key === 'Enter' || e.key === ' ') && play(ep)}>
        <div class="track-num"><span class="icon" style="width:12px;height:12px;display:inline-flex">{@html IconPlay}</span></div>
        <div class="track-info">
          <div class="track-title">{ep.title}</div>
          <div class="track-artist">{ep.positionMs > 0 ? `Resume at ${formatDuration(ep.positionMs / 1000)}` : ''}</div>
        </div>
        <div class="track-duration">{ep.durationSeconds ? formatDuration(ep.durationSeconds) : ''}</div>
      </div>
    {/each}
  </div>
{/if}
