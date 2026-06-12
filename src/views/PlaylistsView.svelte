<script lang="ts">
  import { onMount } from 'svelte'
  import { IconList, IconCloud } from '../lib/icons'
  import { playlists, navToPlaylist, serverPlaylists } from '../lib/stores'
  import { Api, loadImage, type ServerPlaylist } from '../lib/api'
  import { lazyLoad } from '../lib/lazyLoad'

  let showDialog = $state(false)
  let nameInput = $state('')
  let inputEl: HTMLInputElement | undefined = $state()
  let serverLoading = $state(false)
  let serverError = $state('')

  // On mount, fetch server playlists and store them for display.
  onMount(async () => {
    serverLoading = true
    serverError = ''
    try {
      const fetched = await Api.getPlaylists()
      serverPlaylists.set(fetched)
    } catch (e: any) {
      serverError = e.message ?? 'Failed to load server playlists'
    } finally {
      serverLoading = false
    }
  })

  // Server playlists that don't already exist locally (matched by serverId).
  const syncedServerIds = $derived(new Set($playlists.map(p => p.serverId).filter(Boolean)))
  const serverOnlyPlaylists = $derived($serverPlaylists.filter(sp => !syncedServerIds.has(sp.id)))

  function createNew() {
    nameInput = ''
    showDialog = true
    // Focus the input after the DOM updates
    setTimeout(() => inputEl?.focus(), 0)
  }

  async function confirm() {
    if (!nameInput.trim()) { showDialog = false; return }
    const name = nameInput.trim()
    showDialog = false
    // Create locally first so the user sees it immediately.
    const pl = playlists.create(name)
    navToPlaylist(pl.id)
    // Then create on the server and record the server ID.
    try {
      const serverPl = await Api.createPlaylist(name)
      if (serverPl.id) playlists.setServerId(pl.id, serverPl.id)
    } catch (e) {
      console.error('Failed to create playlist on server:', e)
    }
  }

  function cancel() {
    showDialog = false
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') confirm()
    if (e.key === 'Escape') cancel()
  }

  // Navigate to a server-only playlist using a prefixed ID so PlaylistDetail can detect it.
  function openServerPlaylist(sp: ServerPlaylist) {
    navToPlaylist('server-' + sp.id)
  }
</script>

{#if showDialog}
  <!-- Custom dialog overlay for creating a new playlist -->
  <div class="dialog-backdrop" onclick={cancel} onkeydown={e => e.key === 'Escape' && cancel()} role="presentation">
    <div class="dialog" onclick={e => e.stopPropagation()} onkeydown={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="New Playlist" tabindex="-1">
      <div class="dialog-title">New Playlist</div>
      <input
        bind:this={inputEl}
        bind:value={nameInput}
        class="dialog-input"
        placeholder="Playlist name"
        onkeydown={onKeydown}
        maxlength="100"
      />
      <div class="dialog-actions">
        <button class="dialog-btn cancel" onclick={cancel}>Cancel</button>
        <button class="dialog-btn confirm" onclick={confirm} disabled={!nameInput.trim()}>Create</button>
      </div>
    </div>
  </div>
{/if}

<!-- Local playlists section -->
<div class="pl-list-header">
  <span class="section-header" style="margin:0">Your Playlists</span>
  <button class="pl-new-btn" onclick={createNew}>+ New</button>
</div>

{#if $playlists.length === 0}
  <div class="pl-empty-state">
    <div class="pl-empty-icon"><span class="icon" style="width:48px;height:48px;color:var(--muted);opacity:0.4">{@html IconList}</span></div>
    <div>No playlists yet</div>
  </div>
{:else}
  {#each $playlists as pl}
    <div
      class="pl-card"
      role="button"
      tabindex="0"
      onclick={() => navToPlaylist(pl.id)}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToPlaylist(pl.id)}
    >
      <div class="pl-card-art">
        {#if pl.coverDataUrl}
          <img src={pl.coverDataUrl} alt="" />
        {:else if pl.coverArtId}
          <img use:lazyLoad={img => loadImage(img, pl.coverArtId, null)} alt="" />
        {:else}
          <div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconList}</span></div>
        {/if}
      </div>
      <div class="pl-card-info">
        <div class="pl-card-name">
          {pl.name}
          {#if pl.serverId}<span class="pl-synced-badge" title="Synced to server"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>{/if}
        </div>
        <div class="pl-card-meta">
          {pl.tracks.length} track{pl.tracks.length !== 1 ? 's' : ''}
          {#if pl.description} · {pl.description.slice(0, 60)}{/if}
        </div>
      </div>
    </div>
  {/each}
{/if}

<!-- Server playlists section (playlists from Navidrome not already in local store) -->
{#if serverLoading}
  <div class="pl-server-loading">Loading server playlists…</div>
{:else if serverError}
  <div class="pl-server-error">{serverError}</div>
{:else if serverOnlyPlaylists.length > 0}
  <div class="section-header pl-server-section-header">From Server</div>
  {#each serverOnlyPlaylists as sp}
    <div
      class="pl-card pl-card--server"
      role="button"
      tabindex="0"
      onclick={() => openServerPlaylist(sp)}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && openServerPlaylist(sp)}
    >
      <div class="pl-card-art">
        {#if sp.coverArt}
          <img use:lazyLoad={img => loadImage(img, sp.coverArt as string, null)} alt="" />
        {:else}
          <div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconList}</span></div>
        {/if}
      </div>
      <div class="pl-card-info">
        <div class="pl-card-name">
          {sp.name}
          <span class="pl-server-badge" title="Server playlist"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>
        </div>
        <div class="pl-card-meta">
          {sp.songCount ?? 0} track{(sp.songCount ?? 0) !== 1 ? 's' : ''}
          {#if sp.comment} · {sp.comment.slice(0, 60)}{/if}
        </div>
      </div>
    </div>
  {/each}
{/if}

<style>
  .pl-synced-badge, .pl-server-badge {
    display: inline-flex;
    align-items: center;
    margin-left: 5px;
    color: var(--accent);
    opacity: 0.8;
    vertical-align: middle;
  }

  .pl-card--server {
    opacity: 0.9;
  }

  .pl-server-section-header {
    margin-top: 16px;
  }

  .pl-server-loading, .pl-server-error {
    font-size: 0.8rem;
    color: var(--muted);
    padding: 8px 0;
  }

  .pl-server-error {
    color: var(--error);
  }

  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .dialog {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .dialog-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text);
  }

  .dialog-input {
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text);
    font-size: 0.9rem;
    padding: 9px 12px;
    outline: none;
    transition: border-color 0.15s;
  }

  .dialog-input:focus {
    border-color: var(--accent);
  }

  .dialog-input::placeholder {
    color: color-mix(in srgb, var(--text) 40%, transparent);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .dialog-btn {
    border: none;
    border-radius: 7px;
    padding: 8px 18px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s, background 0.15s;
  }

  .dialog-btn.cancel {
    background: var(--surface2);
    color: var(--text);
  }

  .dialog-btn.cancel:hover {
    opacity: 0.8;
  }

  .dialog-btn.confirm {
    background: var(--accent);
    color: var(--bg);
  }

  .dialog-btn.confirm:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .dialog-btn.confirm:not(:disabled):hover {
    opacity: 0.85;
  }
</style>
