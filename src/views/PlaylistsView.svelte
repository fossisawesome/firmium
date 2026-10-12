<script lang="ts">
  import { onMount } from 'svelte'
  import { IconList, IconCloud } from '../lib/icons'
  import { isAuthed, navTo, toastError } from '../lib/stores'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { errorMessage } from '../lib/ipc'
  import { localPlaylists, serverOnlyPlaylists, loadPlaylists } from '../lib/playlists'
  import PlaylistMosaic from '../components/PlaylistMosaic.svelte'

  let showDialog = $state(false)
  let nameInput = $state('')
  let inputEl: HTMLInputElement | undefined = $state()
  let loading = $state(false)
  let error = $state('')
  let syncing = $state(new Set<string>())

  onMount(async () => {
    loading = true
    try { await loadPlaylists($isAuthed) } catch (e) { error = errorMessage(e) }
    finally { loading = false }
  })

  function createNew() {
    nameInput = ''
    showDialog = true
    setTimeout(() => inputEl?.focus(), 0)
  }

  async function confirm() {
    const name = nameInput.trim()
    showDialog = false
    if (!name) return
    try {
      const lists = await Api.createPlaylist(name)
      localPlaylists.set(lists)
      navTo('playlist', lists[0].id)
    } catch (e) { toastError(e) }
  }

  async function syncNow(e: MouseEvent, localId: string) {
    e.stopPropagation()
    if (syncing.has(localId)) return
    syncing = new Set(syncing).add(localId)
    try { localPlaylists.set(await Api.syncPlaylistNow(localId)) } catch (err) { toastError(err) }
    finally { const n = new Set(syncing); n.delete(localId); syncing = n }
  }

  const total = $derived($localPlaylists.length + $serverOnlyPlaylists.length)
</script>

{#if showDialog}
  <div class="dialog-backdrop" onclick={() => (showDialog = false)} onkeydown={e => e.key === 'Escape' && (showDialog = false)} role="presentation">
    <div class="dialog" onclick={e => e.stopPropagation()} onkeydown={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="New Playlist" tabindex="-1">
      <div class="dialog-title">New Playlist</div>
      <input bind:this={inputEl} bind:value={nameInput} class="dialog-input" placeholder="Playlist name" maxlength="100"
        onkeydown={e => { if (e.key === 'Enter') confirm(); if (e.key === 'Escape') showDialog = false }} />
      <div class="dialog-actions">
        <button class="dialog-btn cancel" onclick={() => (showDialog = false)}>Cancel</button>
        <button class="dialog-btn confirm" onclick={confirm} disabled={!nameInput.trim()}>Create</button>
      </div>
    </div>
  </div>
{/if}

<div class="pl-list-header">
  <span class="section-header" style="margin:0">Playlists</span>
  <button class="pl-new-btn" onclick={createNew}>+ New</button>
</div>

{#if loading}<div class="pl-server-loading">Loading playlists…</div>
{:else if error}<div class="pl-server-error">{error}</div>{/if}

{#if total === 0}
  <div class="pl-empty-state">
    <div class="pl-empty-icon"><span class="icon" style="width:48px;height:48px;color:var(--muted);opacity:0.4">{@html IconList}</span></div>
    <div>No playlists yet</div>
  </div>
{:else}
  {#each $localPlaylists as pl (pl.id)}
    <div class="pl-card" role="button" tabindex="0" onclick={() => navTo('playlist', pl.id)}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navTo('playlist', pl.id)}>
      <div class="pl-card-art">
        {#if pl.tracks.length}<PlaylistMosaic covers={pl.tracks.map(t => t.coverArtId)} />
        {:else}<div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconList}</span></div>{/if}
      </div>
      <div class="pl-card-info">
        <div class="pl-card-name">{pl.name}
          {#if pl.serverId}<span class="pl-synced-badge" title="Synced to server"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>{/if}
        </div>
        <div class="pl-card-meta">{pl.tracks.length} track{pl.tracks.length !== 1 ? 's' : ''}</div>
      </div>
      {#if $isAuthed && !pl.serverId}
        <button class="pl-sync-btn" title="Sync to server" disabled={syncing.has(pl.id)} onclick={e => syncNow(e, pl.id)}>
          <span class="icon" style="width:14px;height:14px">{@html IconCloud}</span>{syncing.has(pl.id) ? 'Syncing…' : 'Sync'}
        </button>
      {/if}
    </div>
  {/each}
  {#each $serverOnlyPlaylists as sp (sp.id)}
    <div class="pl-card" role="button" tabindex="0" onclick={() => navTo('playlist', `server-${sp.id}`)}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navTo('playlist', `server-${sp.id}`)}>
      <div class="pl-card-art">
        {#if sp.coverArt}<img use:cover={sp.coverArt} alt="" />
        {:else}<div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconList}</span></div>{/if}
      </div>
      <div class="pl-card-info">
        <div class="pl-card-name">{sp.name}
          <span class="pl-server-badge" title="Server playlist"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>
        </div>
        <div class="pl-card-meta">{sp.songCount ?? 0} track{sp.songCount !== 1 ? 's' : ''}{#if sp.comment} · {sp.comment.slice(0, 60)}{/if}</div>
      </div>
    </div>
  {/each}
{/if}

<style>
  .pl-sync-btn { display: flex; align-items: center; gap: 5px; flex-shrink: 0; padding: 5px 10px; font-size: 0.8rem; color: var(--accent); background: transparent; border: 1px solid var(--accent); border-radius: 3px; cursor: pointer; font-family: var(--font); }
  .pl-sync-btn:hover { background: var(--accent); color: var(--bg); }
  .pl-sync-btn:disabled { opacity: 0.5; cursor: default; }
  .pl-synced-badge, .pl-server-badge { display: inline-flex; align-items: center; margin-left: 5px; color: var(--accent); opacity: 0.8; vertical-align: middle; }
  .pl-server-loading, .pl-server-error { font-size: 0.8rem; color: var(--muted); padding: 8px 0; }
  .pl-server-error { color: var(--error); }
  .dialog-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.55); display: flex; align-items: center; justify-content: center; z-index: 1000; backdrop-filter: blur(2px); }
  .dialog { background: var(--surface); border: 1px solid var(--border); border-radius: 4px; padding: 24px; width: 320px; display: flex; flex-direction: column; gap: 16px; box-shadow: 0 8px 32px rgba(0,0,0,0.4); }
  .dialog-title { font-size: 1rem; font-weight: 600; color: var(--text); }
  .dialog-input { background: var(--surface2); border: 1px solid var(--border); border-radius: 3px; color: var(--text); font-family: var(--font); font-size: 0.9rem; padding: 9px 12px; outline: none; transition: border-color 0.15s; }
  .dialog-input:focus { border-color: var(--accent); }
  .dialog-actions { display: flex; justify-content: flex-end; gap: 10px; }
  .dialog-btn { border: none; border-radius: 3px; padding: 8px 18px; font-family: var(--font); font-size: 0.85rem; font-weight: 500; cursor: pointer; transition: opacity 0.15s; }
  .dialog-btn.cancel { background: var(--surface2); color: var(--text); }
  .dialog-btn.confirm { background: var(--accent); color: var(--bg); }
  .dialog-btn.confirm:disabled { opacity: 0.4; cursor: default; }
</style>
