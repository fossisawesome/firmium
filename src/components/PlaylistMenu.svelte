<script lang="ts">
  import { menuState, hideMenu, menuList, menuCreate } from '../lib/menu'
  import { localPlaylists, serverOnlyPlaylists, loadPlaylists } from '../lib/playlists'
  import { isAuthed, toastError, toast } from '../lib/stores'
  import { Api } from '../lib/api'
  import { IconCloud, IconWaveform, IconList } from '../lib/icons'
  import type { Song } from '../lib/types'

  let newName = $state('')
  let popupEl: HTMLDivElement | undefined = $state()

  // Anchor the popup beside the button, keeping it on screen.
  $effect(() => {
    if (!$menuState.visible || !$menuState.rect || !popupEl) return
    const { rect } = $menuState
    popupEl.style.left = `${rect.right + 6}px`
    popupEl.style.top = `${rect.top}px`
    requestAnimationFrame(() => {
      if (!popupEl) return
      const pr = popupEl.getBoundingClientRect()
      if (pr.right > window.innerWidth - 8) popupEl.style.left = `${rect.left - pr.width - 6}px`
      if (pr.bottom > window.innerHeight - 8) popupEl.style.top = `${Math.max(8, window.innerHeight - pr.height - 8)}px`
    })
  })

  // Refresh the playlist list each time the picker opens.
  $effect(() => {
    if ($menuState.visible && $menuState.mode === 'list') loadPlaylists($isAuthed).catch(() => {})
  })

  // Capture phase so outside clicks dismiss the popup regardless of stopPropagation.
  $effect(() => {
    const onClick = (e: MouseEvent) => {
      if (!$menuState.visible) return
      const target = e.target as HTMLElement
      if (popupEl?.contains(target)) return
      if (target.closest('.track-add-btn, .album-add-btn, .artist-add-btn')) return
      hideMenu()
    }
    document.addEventListener('click', onClick, true)
    return () => document.removeEventListener('click', onClick, true)
  })

  // The tracks the pending item stands for (an album/artist expands to its tracks).
  async function pendingTracks(): Promise<Song[]> {
    const p = $menuState.pending
    if (!p) return []
    if (p.type === 'tracks') return p.tracks
    if (p.type === 'album') return (await Api.albumTracks(p.albumId)).tracks
    const { albums } = await Api.artistDetails(p.artistId)
    return albums[0] ? (await Api.albumTracks(albums[0].id)).tracks : []
  }

  async function startRadio() {
    hideMenu()
    try {
      const seed = (await pendingTracks())[0]
      if (seed) await Api.startRadio(seed)
    } catch (e) { toastError(e) }
  }

  async function addToLocal(localId: string) {
    hideMenu()
    try {
      let lists
      for (const song of await pendingTracks()) lists = await Api.addToPlaylist(localId, song)
      if (lists) localPlaylists.set(lists)
    } catch (e) { toastError(e) }
  }

  async function addToServerOnly(serverId: string) {
    hideMenu()
    try { await Api.addToServerPlaylist(serverId, (await pendingTracks()).map(t => t.id)) } catch (e) { toastError(e) }
  }

  async function confirmCreate() {
    const name = newName.trim()
    if (!name) return
    hideMenu()
    newName = ''
    try {
      const tracks = await pendingTracks()
      let lists = await Api.createPlaylist(name, tracks[0])
      const created = lists[0]
      for (const t of tracks.slice(1)) lists = await Api.addToPlaylist(created.id, t)
      localPlaylists.set(lists)
      toast(`Added to "${name}"`)
    } catch (e) { toastError(e) }
  }

  function onItemKey(e: KeyboardEvent, act: () => void) {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); act(); return }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' || !popupEl) return
    e.preventDefault()
    const items = Array.from(popupEl.querySelectorAll<HTMLElement>('[role="button"]'))
    const idx = items.indexOf(e.currentTarget as HTMLElement)
    if (idx >= 0) items[(idx + (e.key === 'ArrowDown' ? 1 : items.length - 1)) % items.length].focus()
  }

  $effect(() => {
    if ($menuState.visible && $menuState.mode !== 'create' && popupEl) {
      requestAnimationFrame(() => popupEl?.querySelector<HTMLElement>('[role="button"]')?.focus())
    }
  })
</script>

<div class="pl-popup" class:pl-popup--visible={$menuState.visible} bind:this={popupEl}>
  {#if $menuState.mode === 'create'}
    <div class="pl-popup-header">New playlist</div>
    <!-- svelte-ignore a11y_autofocus -->
    <input class="pl-popup-input" type="text" placeholder="Playlist name…" maxlength="100" bind:value={newName} autofocus
      onkeydown={e => { if (e.key === 'Enter') { e.preventDefault(); confirmCreate() } if (e.key === 'Escape') { e.stopPropagation(); hideMenu() } }} />
    <div class="pl-popup-actions">
      <button class="pl-popup-btn pl-popup-cancel" onclick={hideMenu}>Cancel</button>
      <button class="pl-popup-btn pl-popup-confirm" onclick={confirmCreate}>Create</button>
    </div>
  {:else if $menuState.mode === 'actions'}
    {#if $isAuthed}
      <div class="pl-popup-item" role="button" tabindex="0" onclick={startRadio} onkeydown={e => onItemKey(e, startRadio)}>
        <span class="pl-popup-name"><span class="icon" aria-hidden="true" style="width:15px;height:15px;margin-right:8px;vertical-align:-2px;display:inline-flex">{@html IconWaveform}</span>Start Radio</span>
      </div>
    {/if}
    <div class="pl-popup-item" role="button" tabindex="0" onclick={menuList} onkeydown={e => onItemKey(e, menuList)}>
      <span class="pl-popup-name"><span class="icon" aria-hidden="true" style="width:15px;height:15px;margin-right:8px;vertical-align:-2px;display:inline-flex">{@html IconList}</span>Add to playlist</span>
    </div>
  {:else}
    <div class="pl-popup-header">Add to playlist</div>
    {#if $localPlaylists.length + $serverOnlyPlaylists.length === 0}
      <div class="pl-popup-empty">No playlists yet</div>
    {/if}
    {#each $localPlaylists as pl}
      <div class="pl-popup-item" role="button" tabindex="0" onclick={() => addToLocal(pl.id)} onkeydown={e => onItemKey(e, () => addToLocal(pl.id))}>
        <span class="pl-popup-name">{pl.name}
          {#if pl.serverId}<span class="pl-popup-cloud" title="Synced to server"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>{/if}
        </span>
        <span class="pl-popup-count">{pl.tracks.length}</span>
      </div>
    {/each}
    {#each $serverOnlyPlaylists as pl}
      <div class="pl-popup-item" role="button" tabindex="0" onclick={() => addToServerOnly(pl.id)} onkeydown={e => onItemKey(e, () => addToServerOnly(pl.id))}>
        <span class="pl-popup-name">{pl.name}
          <span class="pl-popup-cloud" title="Server playlist"><span class="icon" style="width:10px;height:10px">{@html IconCloud}</span></span>
        </span>
        <span class="pl-popup-count">{pl.songCount ?? ''}</span>
      </div>
    {/each}
    <div class="pl-popup-divider"></div>
    <div class="pl-popup-item pl-popup-create" role="button" tabindex="0" onclick={menuCreate} onkeydown={e => onItemKey(e, menuCreate)}>+ Create New</div>
  {/if}
</div>
