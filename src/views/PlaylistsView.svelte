<script lang="ts">
  import { onMount } from 'svelte'
  import { IconList, IconCloud } from '../lib/icons'
  import { playlists, navToPlaylist, serverPlaylists, mergePlaylists } from '../lib/stores'
  import { Api, loadImage } from '../lib/api'
  import { lazyLoad } from '../lib/lazyLoad'

  let showDialog = $state(false)
  let nameInput = $state('')
  let inputEl: HTMLInputElement | undefined = $state()
  let serverLoading = $state(false)
  let serverError = $state('')

  // On mount, fetch server playlists and retry any local playlists that
  // haven't been created on the server yet (e.g. created while offline).
  onMount(async () => {
    serverLoading = true
    serverError = ''
    try {
      const fetched = await Api.getPlaylists()
      serverPlaylists.set(fetched)

      for (const p of $playlists) {
        if (p.serverId || p.createPending === false || (p.createAttempts ?? 0) >= 3) continue
        // Adopt an existing server playlist with the same name instead of creating a duplicate.
        const existing = fetched.find(sp => sp.name === p.name)
        if (existing) { playlists.markCreateAttempt(p.id, true, existing.id); continue }
        try {
          const serverPl = await Api.createPlaylist(p.name)
          playlists.markCreateAttempt(p.id, true, serverPl.id)
        } catch (e) {
          console.error('Retry: failed to create playlist on server:', e)
          playlists.markCreateAttempt(p.id, false)
        }
      }
    } catch (e: any) {
      serverError = e.message ?? 'Failed to load server playlists'
    } finally {
      serverLoading = false
    }
  })

  const unified = $derived(mergePlaylists($playlists, $serverPlaylists))

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
      playlists.markCreateAttempt(pl.id, true, serverPl.id)
    } catch (e) {
      console.error('Failed to create playlist on server:', e)
      playlists.markCreateAttempt(pl.id, false)
    }
  }

  function cancel() {
    showDialog = false
  }

  let syncingIds = $state(new Set<string>())

  async function syncToServer(e: MouseEvent, item: typeof unified[number]) {
    e.stopPropagation()
    if (!item.local || syncingIds.has(item.local.id)) return
    const localId = item.local.id
    syncingIds = new Set(syncingIds).add(localId)
    try {
      const serverPl = await Api.createPlaylist(item.name)
      playlists.setServerId(localId, serverPl.id)
    } catch (e) {
      console.error('Failed to sync playlist to server:', e)
    } finally {
      const next = new Set(syncingIds)
      next.delete(localId)
      syncingIds = next
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') confirm()
    if (e.key === 'Escape') cancel()
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

<div class="pl-list-header">
  <span class="section-header" style="margin:0">Playlists</span>
  <button class="pl-new-btn" onclick={createNew}>+ New</button>
</div>

{#if serverLoading}
  <div class="pl-server-loading">Loading server playlists…</div>
{:else if serverError}
  <div class="pl-server-error">{serverError}</div>
{/if}

{#if unified.length === 0}
  <div class="pl-empty-state">
    <div class="pl-empty-icon"><span class="icon" style="width:48px;height:48px;color:var(--muted);opacity:0.4">{@html IconList}</span></div>
    <div>No playlists yet</div>
  </div>
{:else}
  {#each unified as item}
    <div
      class="pl-card"
      role="button"
      tabindex="0"
      onclick={() => navToPlaylist(item.id)}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToPlaylist(item.id)}
    >
      <div class="pl-card-art">
        {#if item.coverDataUrl}
          <img src={item.coverDataUrl} alt="" />
        {:else if item.coverArtId}
          <img use:lazyLoad={img => loadImage(img, item.coverArtId, null)} alt="" />
        {:else}
          <div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconList}</span></div>
        {/if}
      </div>
      <div class="pl-card-info">
        <div class="pl-card-name">
          {item.name}
          {#if item.source === 'synced'}
            <span class="pl-synced-badge" title="Synced to server"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>
          {:else if item.source === 'server-only'}
            <span class="pl-server-badge" title="Server playlist"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>
          {/if}
        </div>
        <div class="pl-card-meta">
          {item.trackCount} track{item.trackCount !== 1 ? 's' : ''}
          {#if item.description} · {item.description.slice(0, 60)}{/if}
        </div>
      </div>
      {#if item.source === 'local' && item.local}
        <button
          class="pl-sync-btn"
          title="Sync to server"
          disabled={syncingIds.has(item.local.id)}
          onclick={e => syncToServer(e, item)}
        >
          <span class="icon" style="width:14px;height:14px">{@html IconCloud}</span>
          {syncingIds.has(item.local.id) ? 'Syncing…' : 'Sync'}
        </button>
      {/if}
    </div>
  {/each}
{/if}

<style>
  .pl-sync-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
    padding: 5px 10px;
    font-size: 0.8rem;
    color: var(--accent);
    background: transparent;
    border: 1px solid var(--accent);
    border-radius: 6px;
    cursor: pointer;
  }

  .pl-sync-btn:hover {
    background: var(--accent);
    color: var(--bg);
  }

  .pl-sync-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .pl-synced-badge, .pl-server-badge {
    display: inline-flex;
    align-items: center;
    margin-left: 5px;
    color: var(--accent);
    opacity: 0.8;
    vertical-align: middle;
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
