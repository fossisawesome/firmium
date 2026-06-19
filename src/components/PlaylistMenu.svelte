<script lang="ts">
  import { get } from 'svelte/store'
  import { playlistMenuState, hidePlaylistMenu, switchToList, switchToCreate } from '../lib/playlistMenu'
  import { playlists, serverPlaylists, mergePlaylists } from '../lib/stores'
  import { Api } from '../lib/api'
  import { IconCloud, IconWaveform, IconList } from '../lib/icons'
  import { dataSource } from '../lib/dataSource'
  import { startRadio } from '../lib/radio'
  import type { Song } from '../lib/types/tauri-commands'

  const unified = $derived(mergePlaylists($playlists, $serverPlaylists))

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

  // Lazily fetch server playlists when the popup opens, so server-only playlists
  // (not created by this client) can be offered as add targets too.
  $effect(() => {
    if ($playlistMenuState.visible && $playlistMenuState.mode !== 'create' && get(serverPlaylists).length === 0) {
      Api.getPlaylists().then(fetched => serverPlaylists.set(fetched)).catch(console.error)
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
    if (target.closest('.track-add-btn') || target.closest('.album-add-btn') || target.closest('.artist-add-btn')) return
    hidePlaylistMenu()
  }

  // Resolves a representative seed track for the pending item, then starts a
  // radio queue from it (shared seeding cascade in radio.ts).
  async function startRadioFromPending() {
    const pending = $playlistMenuState.pending
    hidePlaylistMenu()
    if (!pending) return
    try {
      let seed: Song | undefined
      if (pending.type === 'tracks') {
        seed = pending.tracks[0]
      } else if (pending.type === 'album') {
        const { tracks } = await $dataSource.getAlbumTracks(pending.albumId)
        seed = tracks[0]
      } else if (pending.type === 'artist') {
        const { albums } = await $dataSource.getArtistDetails(pending.artistId)
        if (albums[0]) {
          const { tracks } = await $dataSource.getAlbumTracks(albums[0].id)
          seed = tracks[0]
        }
      }
      if (seed) await startRadio(seed)
    } catch (err) {
      console.error('Start Radio failed:', err)
    }
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

  // Adds tracks directly to a server-only playlist (no local entry exists for it).
  async function addTracksToServerPlaylist(serverId: string, tracks: Song[]): Promise<void> {
    await Api.updatePlaylist(serverId, { songIdsToAdd: tracks.map(t => t.id) })
  }

  async function addTo(playlistId: string) {
    const pending = $playlistMenuState.pending
    hidePlaylistMenu()
    if (!pending) return
    const item = unified.find(u => u.id === playlistId)
    const isServerOnly = item?.source === 'server-only' && item.serverId
    if (pending.type === 'tracks') {
      if (isServerOnly) {
        await addTracksToServerPlaylist(item.serverId!, pending.tracks).catch(console.error)
      } else {
        await syncAddTracks(playlistId, pending.tracks)
      }
    } else if (pending.type === 'album') {
      try {
        const { tracks } = await $dataSource.getAlbumTracks(pending.albumId)
        if (isServerOnly) {
          await addTracksToServerPlaylist(item.serverId!, tracks).catch(console.error)
        } else {
          await syncAddTracks(playlistId, tracks)
        }
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

  // Auto-focus the first item when the "add to playlist" popup opens.
  $effect(() => {
    if ($playlistMenuState.visible && $playlistMenuState.mode !== 'create' && popupEl) {
      requestAnimationFrame(() => {
        popupEl?.querySelector<HTMLElement>('[role="button"]')?.focus()
      })
    }
  })

  function handleItemKeydown(e: KeyboardEvent, onActivate: () => void) {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onActivate(); return }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
    e.preventDefault()
    if (!popupEl) return
    const items = Array.from(popupEl.querySelectorAll<HTMLElement>('[role="button"]'))
    const idx = items.indexOf(e.currentTarget as HTMLElement)
    if (idx === -1) return
    const next = e.key === 'ArrowDown' ? (idx + 1) % items.length : (idx - 1 + items.length) % items.length
    items[next].focus()
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
  {:else if $playlistMenuState.mode === 'actions'}
    <div
      class="pl-popup-item"
      role="button"
      tabindex="0"
      onclick={startRadioFromPending}
      onkeydown={e => handleItemKeydown(e, startRadioFromPending)}
    >
      <span class="pl-popup-name"><span class="icon" aria-hidden="true" style="width:15px;height:15px;margin-right:8px;vertical-align:-2px;display:inline-flex">{@html IconWaveform}</span>Start Radio</span>
    </div>
    <div
      class="pl-popup-item"
      role="button"
      tabindex="0"
      onclick={switchToList}
      onkeydown={e => handleItemKeydown(e, switchToList)}
    >
      <span class="pl-popup-name"><span class="icon" aria-hidden="true" style="width:15px;height:15px;margin-right:8px;vertical-align:-2px;display:inline-flex">{@html IconList}</span>Add to playlist</span>
    </div>
  {:else}
    <div class="pl-popup-header">Add to playlist</div>
    {#if unified.length === 0}
      <div class="pl-popup-empty">No playlists yet</div>
    {:else}
      {#each unified as pl}
        <div
          class="pl-popup-item"
          role="button"
          tabindex="0"
          onclick={() => addTo(pl.id)}
          onkeydown={e => handleItemKeydown(e, () => addTo(pl.id))}
        >
          <span class="pl-popup-name">
            {pl.name}
            {#if pl.source === 'synced' || pl.source === 'server-only'}
              <span class="pl-popup-cloud" title={pl.source === 'synced' ? 'Synced to server' : 'Server playlist'}><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>
            {/if}
          </span>
          <span class="pl-popup-count">{pl.trackCount}</span>
        </div>
      {/each}
    {/if}
    <div class="pl-popup-divider"></div>
    <div
      class="pl-popup-item pl-popup-create"
      role="button"
      tabindex="0"
      onclick={switchToCreate}
      onkeydown={e => handleItemKeydown(e, switchToCreate)}
    >+ Create New</div>
  {/if}
</div>
