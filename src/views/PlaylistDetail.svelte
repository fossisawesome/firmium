<script lang="ts">
  import { onMount } from 'svelte'
  import { IconList, IconPlay, IconCloud, IconShuffle } from '../lib/icons'
  import { playlists, currentTrack, navToView, serverPlaylists, type Playlist } from '../lib/stores'
  import { Api, loadImage } from '../lib/api'
  import { setQueueSeamless, shufflePlay } from '../lib/playback'
  import { showPlaylistMenu } from '../lib/playlistMenu'
  import { lazyLoad } from '../lib/lazyLoad'
  import { formatDuration } from '../lib/utils'
  import VirtualList from '../lib/VirtualList.svelte'
  import TrackRow from '../components/TrackRow.svelte'
  import type { Song } from '../lib/types/tauri-commands'

  const TRACK_ROW_HEIGHT = 56

  type DetailPlaylist = (Playlist & { isServerOnly?: false }) | {
    id: string
    name: string
    description: string
    coverArtId: string | null
    coverDataUrl: string | null
    tracks: Song[]
    serverId: string
    isServerOnly: true
  }

  let { id }: { id: string } = $props()

  // Detect whether this is a server-only playlist (id prefixed with 'server-').
  const isServerOnly = id.startsWith('server-')
  const serverId = isServerOnly ? id.slice('server-'.length) : null

  let editingName = $state(false)
  let editingDesc = $state(false)
  let nameValue = $state('')
  let descValue = $state('')
  let showCoverPicker = $state(false)
  let fileInput = $state<HTMLInputElement>()

  // For server-only playlists, tracks are loaded on mount.
  let serverTracks = $state<Song[] | null>(null)
  let serverLoading = $state(false)

  onMount(async () => {
    if (isServerOnly && serverId) {
      serverLoading = true
      try {
        const result = await Api.getPlaylistTracks(serverId)
        serverTracks = result.tracks ?? []
      } catch (e: any) {
        console.error('Failed to load server playlist tracks:', e)
        serverTracks = []
      } finally {
        serverLoading = false
      }
    }
  })

  // For local playlists: look up from the local store.
  // For server-only: build a synthetic object from serverPlaylists metadata + loaded tracks.
  const localPl = $derived(isServerOnly ? null : ($playlists.find(p => p.id === id) ?? null))
  const serverMeta = $derived(isServerOnly ? ($serverPlaylists.find(sp => sp.id === serverId) ?? null) : null)

  const pl = $derived<DetailPlaylist | null>((() => {
    if (!isServerOnly) return localPl
    if (!serverMeta) return null
    return {
      id,
      name: serverMeta.name ?? 'Server Playlist',
      description: serverMeta.comment ?? '',
      coverArtId: (serverMeta.coverArt as string | undefined) ?? null,
      coverDataUrl: null,
      tracks: serverTracks ?? [],
      serverId: serverMeta.id,
      isServerOnly: true
    }
  })())

  const totalDuration = $derived(pl ? pl.tracks.reduce((s, t) => s + (t.duration || 0), 0) : 0)
  const uniqueCovers = $derived(pl ? (() => {
    const seen = new Set<string>()
    return pl.tracks.filter(t => t.coverArtId && !seen.has(t.coverArtId) && seen.add(t.coverArtId))
  })() : [])

  function startEditName() {
    if (!pl || pl.isServerOnly) return
    nameValue = pl.name
    editingName = true
  }

  function commitName() {
    const val = nameValue.trim() || pl?.name
    if (val && pl && !pl.isServerOnly) {
      playlists.updatePlaylist(id, { name: val })
      // Sync to server if this local playlist is linked.
      if (pl.serverId) Api.updatePlaylist(pl.serverId, { name: val }).catch(console.error)
    }
    editingName = false
  }

  function startEditDesc() {
    if (!pl || pl.isServerOnly) return
    descValue = pl.description
    editingDesc = true
  }

  function commitDesc() {
    if (pl && !pl.isServerOnly) {
      playlists.updatePlaylist(id, { description: descValue.trim() })
      // Sync comment to server if linked.
      if (pl.serverId) Api.updatePlaylist(pl.serverId, { comment: descValue.trim() }).catch(console.error)
    }
    editingDesc = false
  }

  function handleNameKey(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); commitName() }
    if (e.key === 'Escape') { editingName = false }
  }

  function handleDescKey(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); commitDesc() }
    if (e.key === 'Escape') { editingDesc = false }
  }

  function playAll() {
    if (!pl || !pl.tracks.length) return
    setQueueSeamless(pl.tracks, 0)
  }

  function shuffleAll() {
    if (!pl || !pl.tracks.length) return
    shufflePlay(pl.tracks)
  }

  function deletePl() {
    if (!pl) return
    if (confirm(`Delete "${pl.name}"? This cannot be undone.`)) {
      if (!pl.isServerOnly) playlists.delete(id)
      // Delete from server if linked.
      const sid = pl.serverId
      if (sid) Api.deletePlaylist(sid).catch(console.error)
      navToView('playlists')
    }
  }

  function removeTrack(track: Song, trackIdx: number) {
    if (!pl) return
    if (pl.isServerOnly) {
      // Server-only: remove from local serverTracks state and sync index to server.
      serverTracks = (serverTracks ?? []).filter((_, i) => i !== trackIdx)
      Api.updatePlaylist(serverId!, { songIndicesToRemove: [trackIdx] }).catch(console.error)
    } else {
      const removedIdx = playlists.removeTrack(id, track.id)
      if (pl.serverId && removedIdx >= 0) {
        Api.updatePlaylist(pl.serverId, { songIndicesToRemove: [removedIdx] }).catch(console.error)
      }
    }
  }

  function setCover(coverId: string) {
    if (pl && !pl.isServerOnly) {
      playlists.updatePlaylist(id, { coverArtId: coverId, coverDataUrl: null })
    }
    showCoverPicker = false
  }

  function handleFileUpload(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (!file || pl?.isServerOnly) return
    const reader = new FileReader()
    reader.onload = ev => {
      playlists.updatePlaylist(id, { coverDataUrl: ev.target?.result as string, coverArtId: null })
    }
    reader.readAsDataURL(file)
  }

  function isPlaying(track: Song) {
    return $currentTrack?.id === track.id
  }

  function playTrack(idx: number) {
    if (!pl) return
    setQueueSeamless(pl.tracks, idx)
  }

  // Pushes the playlist's new track order to the server by removing every
  // original index and re-adding the song IDs in the new order (OpenSubsonic
  // updatePlaylist has no native "move" operation).
  function syncOrderToServer(serverIdToUse: string, newTracks: Song[]) {
    const ids = newTracks.map(t => t.id)
    Api.updatePlaylist(serverIdToUse, { songIndicesToRemove: ids.map((_, i) => i), songIdsToAdd: ids }).catch(console.error)
  }

  function moveTrack(idx: number, direction: -1 | 1) {
    if (!pl) return
    const to = idx + direction
    if (to < 0 || to >= pl.tracks.length) return
    if (pl.isServerOnly) {
      const tracks = [...(serverTracks ?? [])]
      const [moved] = tracks.splice(idx, 1)
      tracks.splice(to, 0, moved)
      serverTracks = tracks
      syncOrderToServer(serverId!, tracks)
    } else {
      const newTracks = playlists.moveTrack(id, idx, to)
      if (newTracks && pl.serverId) syncOrderToServer(pl.serverId, newTracks)
    }
  }
</script>

{#if isServerOnly && serverLoading}
  <div class="loading-msg">Loading playlist from server…</div>
{:else if !pl}
  <div class="loading-msg">Playlist not found.</div>
{:else}
  <div class="pl-detail-header">
    <div
      class="pl-detail-art"
      role="button"
      tabindex="0"
      onclick={() => !pl.isServerOnly && (showCoverPicker = !showCoverPicker)}
      onkeydown={e => !pl.isServerOnly && (e.key === 'Enter' || e.key === ' ') && (showCoverPicker = !showCoverPicker)}
      title={pl.isServerOnly ? '' : 'Change cover'}
    >
      {#if pl.coverDataUrl}
        <img src={pl.coverDataUrl} alt="" />
      {:else if pl.coverArtId}
        <img use:lazyLoad={img => loadImage(img, pl.coverArtId, null)} alt="" />
      {:else}
        <span class="icon" style="width:40px;height:40px;color:var(--muted)">{@html IconList}</span>
      {/if}
      {#if !pl.isServerOnly}<div class="pl-detail-art-overlay">Change<br>Cover</div>{/if}
    </div>

    <div class="pl-detail-info">
      {#if editingName && !pl.isServerOnly}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="pl-inline-edit"
          type="text"
          bind:value={nameValue}
          maxlength="100"
          onblur={commitName}
          onkeydown={handleNameKey}
          autofocus
        />
      {:else}
        <div
          class="pl-detail-name"
          role={pl.isServerOnly ? 'text' : 'button'}
          tabindex={pl.isServerOnly ? -1 : 0}
          onclick={startEditName}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && startEditName()}
          title={pl.isServerOnly ? '' : 'Click to rename'}
        >
          {pl.name}
          {#if pl.serverId || pl.isServerOnly}
            <span class="pl-server-indicator" title="Synced with server"><span class="icon" style="width:11px;height:11px">{@html IconCloud}</span></span>
          {/if}
        </div>
      {/if}

      {#if editingDesc && !pl.isServerOnly}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="pl-inline-edit pl-inline-edit--desc"
          type="text"
          bind:value={descValue}
          placeholder="Add description…"
          maxlength="300"
          onblur={commitDesc}
          onkeydown={handleDescKey}
          autofocus
        />
      {:else}
        <div
          class="pl-detail-desc"
          role={pl.isServerOnly ? 'text' : 'button'}
          tabindex={pl.isServerOnly ? -1 : 0}
          onclick={startEditDesc}
          onkeydown={e => !pl.isServerOnly && (e.key === 'Enter' || e.key === ' ') && startEditDesc()}
          title={pl.isServerOnly ? '' : 'Click to edit description'}
        >
          {#if pl.description}
            {pl.description}
          {:else if !pl.isServerOnly}
            <span class="pl-detail-desc-hint">Add description…</span>
          {/if}
        </div>
      {/if}

      <div class="pl-detail-meta">
        {pl.tracks.length} track{pl.tracks.length !== 1 ? 's' : ''} · {formatDuration(totalDuration)}
      </div>

      <div class="pl-detail-actions">
        <button class="play-all-btn" onclick={playAll} disabled={!pl.tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconPlay}</span>Play All</button>
        <button class="play-all-btn shuffle-all-btn" onclick={shuffleAll} disabled={!pl.tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconShuffle}</span>Shuffle</button>
        <button class="pl-delete-btn" onclick={deletePl}>Delete</button>
      </div>
    </div>
  </div>

  {#if showCoverPicker}
    <div class="pl-cover-picker">
      <div class="pl-cover-picker-header">
        <span>Choose Cover</span>
        <label class="pl-cover-upload-label">
          Upload Image
          <input type="file" accept="image/*" class="pl-cover-file-input" bind:this={fileInput} onchange={handleFileUpload} />
        </label>
      </div>
      <div class="pl-cover-grid">
        {#if uniqueCovers.length === 0}
          <div class="pl-cover-empty">No track art available</div>
        {:else}
          {#each uniqueCovers as t}
            <div
              class="pl-cover-option"
              role="button"
              tabindex="0"
              onclick={() => setCover(t.coverArtId!)}
              onkeydown={e => (e.key === 'Enter' || e.key === ' ') && setCover(t.coverArtId!)}
              title={t.title}
            >
              <img use:lazyLoad={img => loadImage(img, t.coverArtId, null)} alt={t.title} />
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}

  {#if pl.tracks.length === 0}
    <div class="loading-msg">{pl.isServerOnly ? 'No tracks in this playlist.' : 'No tracks yet — use the + button on any song or album.'}</div>
  {:else}
    <div class="track-list">
      <VirtualList items={pl.tracks} itemHeight={TRACK_ROW_HEIGHT}>
        {#snippet children(track, idx)}
          <TrackRow
            {track} {idx}
            playing={isPlaying(track)}
            displayNum={idx + 1}
            showAddButton={!pl.isServerOnly}
            onPlay={playTrack}
            onRemove={removeTrack}
            onMove={moveTrack}
            isFirst={idx === 0}
            isLast={idx === pl.tracks.length - 1}
          />
        {/snippet}
      </VirtualList>
    </div>
  {/if}
{/if}
