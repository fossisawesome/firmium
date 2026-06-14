<script lang="ts">
  import { get } from 'svelte/store'
  import { playlistMenuState, hidePlaylistMenu, switchToCreate } from '../lib/playlistMenu'
  import { playlists } from '../lib/stores'
  import { Api } from '../lib/api'
  import { dataSource } from '../lib/dataSource'
  import type { Song } from '../lib/types/tauri-commands'

  let newPlaylistName = $state('')
  let popupEl: HTMLDivElement | undefined = $state()

  // Position the popup relative to the anchor button rect, keeping it on screen.
  $effect(() => {
    if ($playlistMenuState.visible && $playlistMenuState.rect && popupEl) {
      const { rect } = $playlistMenuState
      popupEl.style.left = `${rect.right + 6}px`
      popupEl.style.top = `${rect.top}px`
      // Adjust if it goes off-screen (checked after paint via rAF).
      requestAnimationFrame(() => {
        if (!popupEl) return
        const pr = popupEl.getBoundingClientRect()
        if (pr.right > window.innerWidth - 8) popupEl.style.left = `${rect.left - pr.width - 6}px`
        if (pr.bottom > window.innerHeight - 8) popupEl.style.top = `${Math.max(8, window.innerHeight - pr.height - 8)}px`
      })
    }
  })

  // Capture-phase click listener so clicks outside the popup dismiss it regardless of stopPropagation.
  $effect(() => {
    document.addEventListener('click', handleDocumentClick, true)
    return () => document.removeEventListener('click', handleDocumentClick, true)
  })

  function handleDocumentClick(e: MouseEvent) {
    if (!$playlistMenuState.visible) return
    const target = e.target as HTMLElement
    if (popupEl && popupEl.contains(target)) return
    if (target.closest('.track-add-btn') || target.closest('.album-add-btn')) return
    hidePlaylistMenu()
  }

  // Adds tracks to a local playlist and syncs new track IDs to server if the playlist is linked.
  async function syncAddTracks(playlistId: string, tracks: Song[]) {
    const { newTracks } = playlists.addTracks(playlistId, tracks)
    if (newTracks.length) {
      const pl = get(playlists).find(p => p.id === playlistId)
      if (pl?.serverId) {
        Api.updatePlaylist(pl.serverId, { songIdsToAdd: newTracks.map(t => t.id) }).catch(console.error)
      }
    }
  }

  async function addTo(playlistId: string) {
    const pending = $playlistMenuState.pending
    hidePlaylistMenu()
    if (!pending) return
    if (pending.type === 'tracks') {
      await syncAddTracks(playlistId, pending.tracks)
    } else if (pending.type === 'album') {
      try {
        const { tracks } = await $dataSource.getAlbumTracks(pending.albumId)
        await syncAddTracks(playlistId, tracks)
      } catch (err) {
        console.error('Failed to add album to playlist:', err)
      }
    }
  }

  async function confirmCreate() {
    const name = newPlaylistName.trim()
    if (!name) return
    const pending = $playlistMenuState.pending
    const newPl = playlists.create(name)
    hidePlaylistMenu()
    newPlaylistName = ''
    // Create on server and record the server ID, then add tracks.
    try {
      const serverPl = await Api.createPlaylist(name)
      if (serverPl.id) playlists.setServerId(newPl.id, serverPl.id)
    } catch (e) {
      console.error('Failed to create playlist on server:', e)
    }
    if (!pending) return
    if (pending.type === 'tracks') {
      await syncAddTracks(newPl.id, pending.tracks)
    } else if (pending.type === 'album') {
      try {
        const { tracks } = await $dataSource.getAlbumTracks(pending.albumId)
        await syncAddTracks(newPl.id, tracks)
      } catch (err) {
        console.error('Failed to add album to new playlist:', err)
      }
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); confirmCreate() }
    if (e.key === 'Escape') { e.stopPropagation(); hidePlaylistMenu() }
  }
</script>

<div
  class="pl-popup"
  class:pl-popup--visible={$playlistMenuState.visible}
  bind:this={popupEl}
>
  {#if $playlistMenuState.mode === 'create'}
    <div class="pl-popup-header">New playlist</div>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="pl-popup-input"
      type="text"
      placeholder="Playlist name…"
      maxlength="100"
      bind:value={newPlaylistName}
      onkeydown={handleKeydown}
      autofocus
    />
    <div class="pl-popup-actions">
      <button class="pl-popup-btn pl-popup-cancel" onclick={hidePlaylistMenu}>Cancel</button>
      <button class="pl-popup-btn pl-popup-confirm" onclick={confirmCreate}>Create</button>
    </div>
  {:else}
    <div class="pl-popup-header">Add to playlist</div>
    {#if $playlists.length === 0}
      <div class="pl-popup-empty">No playlists yet</div>
    {:else}
      {#each $playlists as pl}
        <div
          class="pl-popup-item"
          role="button"
          tabindex="0"
          onclick={() => addTo(pl.id)}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && addTo(pl.id)}
        >
          <span class="pl-popup-name">{pl.name}</span>
          <span class="pl-popup-count">{pl.tracks.length}</span>
        </div>
      {/each}
    {/if}
    <div class="pl-popup-divider"></div>
    <div
      class="pl-popup-item pl-popup-create"
      role="button"
      tabindex="0"
      onclick={switchToCreate}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && switchToCreate()}
    >+ Create New</div>
  {/if}
</div>
