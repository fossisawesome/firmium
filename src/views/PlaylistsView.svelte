<script>
  import { IconMusic, IconList, IconPlay } from '../lib/icons.js'
  import { playlists, navToPlaylist } from '../lib/stores.js'
  import { loadImage } from '../lib/api.js'
  import { lazyLoad } from '../lib/lazyLoad.js'

  let showDialog = false
  let nameInput = ''
  let inputEl

  function createNew() {
    nameInput = ''
    showDialog = true
    // Focus the input after the DOM updates
    setTimeout(() => inputEl?.focus(), 0)
  }

  function confirm() {
    if (nameInput.trim()) {
      const pl = playlists.create(nameInput.trim())
      navToPlaylist(pl.id)
    }
    showDialog = false
  }

  function cancel() {
    showDialog = false
  }

  function onKeydown(e) {
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

{#if $playlists.length === 0}
  <div class="section-header">Playlists</div>
  <div class="pl-empty-state">
    <div class="pl-empty-icon"><span class="icon" style="width:48px;height:48px;color:var(--muted);opacity:0.4">{@html IconList}</span></div>
    <div>No playlists yet</div>
    <button class="pl-new-btn" onclick={createNew}>New Playlist</button>
  </div>
{:else}
  <div class="pl-list-header">
    <span class="section-header" style="margin:0">Playlists</span>
    <button class="pl-new-btn" onclick={createNew}>+ New</button>
  </div>
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
        <div class="pl-card-name">{pl.name}</div>
        <div class="pl-card-meta">
          {pl.tracks.length} track{pl.tracks.length !== 1 ? 's' : ''}
          {#if pl.description} · {pl.description.slice(0, 60)}{/if}
        </div>
      </div>
    </div>
  {/each}
{/if}

<style>
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
