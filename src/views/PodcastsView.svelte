<script lang="ts">
  import { onMount } from 'svelte'
  import { Api } from '../lib/api'
  import { navTo } from '../lib/stores'
  import { errorMessage } from '../lib/ipc'
  import { cover } from '../lib/cover'
  import { IconPlus, IconPodcast } from '../lib/icons'
  import type { PodcastChannel } from '../lib/types'

  let channels = $state<PodcastChannel[]>([])
  let loading = $state(true)
  let showAdd = $state(false)
  let url = $state('')
  let adding = $state(false)
  let addError = $state('')

  onMount(async () => {
    channels = await Api.podcastChannels().catch(() => [])
    loading = false
  })

  async function add() {
    if (!url.trim() || adding) return
    adding = true
    addError = ''
    try {
      channels = [...channels, await Api.addPodcast(url)]
      showAdd = false
      url = ''
    } catch (e) { addError = errorMessage(e) }
    finally { adding = false }
  }
</script>

<div class="pl-list-header">
  <span class="section-header" style="margin:0">Podcasts ({channels.length})</span>
  <button class="pl-new-btn" onclick={() => { showAdd = true; addError = '' }}>
    <span class="icon" style="width:12px;height:12px;display:inline-flex;margin-right:6px">{@html IconPlus}</span>Add podcast
  </button>
</div>

{#if showAdd}
  <div class="podcast-add">
    <!-- svelte-ignore a11y_autofocus -->
    <input type="url" placeholder="RSS feed URL…" bind:value={url} autofocus
      onkeydown={e => { if (e.key === 'Enter') add(); if (e.key === 'Escape') showAdd = false }} />
    <button class="debug-btn" onclick={add} disabled={adding || !url.trim()}>{adding ? 'Adding…' : 'Add'}</button>
    <button class="debug-btn" onclick={() => (showAdd = false)}>Cancel</button>
    {#if addError}<div class="error-msg">{addError}</div>{/if}
  </div>
{/if}

{#if !loading && channels.length === 0}
  <div class="pl-empty-state">
    <div class="pl-empty-icon"><span class="icon" style="width:48px;height:48px;color:var(--muted);opacity:0.4">{@html IconPodcast}</span></div>
    <div>No podcasts yet. Add one by RSS feed URL.</div>
  </div>
{/if}

{#each channels as ch (ch.id)}
  <div class="pl-card" role="button" tabindex="0" onclick={() => navTo('podcast', ch.id)}
    onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navTo('podcast', ch.id)}>
    <div class="pl-card-art">
      {#if ch.imageUrl}<img use:cover={ch.imageUrl} alt="" />
      {:else}<div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconPodcast}</span></div>{/if}
    </div>
    <div class="pl-card-info">
      <div class="pl-card-name">{ch.title}</div>
      <div class="pl-card-meta">{(ch.description ?? '').replace(/<[^>]+>/g, '').slice(0, 90)}</div>
    </div>
  </div>
{/each}

<style>
  .podcast-add { display: flex; flex-wrap: wrap; gap: 8px; margin: 0 0 16px; align-items: center; }
  .podcast-add input { flex: 1; min-width: 220px; background: var(--surface); border: 1px solid var(--border); color: var(--text); padding: 8px 10px; border-radius: 3px; font-family: var(--font); }
  .podcast-add .error-msg { flex-basis: 100%; }
</style>
