<script lang="ts">
  import { onMount } from 'svelte'
  import { IconList, IconPlay, IconCloud, IconShuffle } from '../lib/icons'
  import { currentTrack, isAuthed, navTo, toastError } from '../lib/stores'
  import { Api } from '../lib/api'
  import { formatDuration } from '../lib/utils'
  import { localPlaylists, loadPlaylists } from '../lib/playlists'
  import VirtualList from '../lib/VirtualList.svelte'
  import TrackRow from '../components/TrackRow.svelte'
  import PlaylistMosaic from '../components/PlaylistMosaic.svelte'
  import type { Song } from '../lib/types'

  const ROW_HEIGHT = 56
  let { id }: { id: string } = $props()

  // Server-only playlists are navigated to as `server-<id>`.
  const serverOnlyId = id.startsWith('server-') ? id.slice('server-'.length) : null

  let serverName = $state('Server Playlist')
  let serverTracks = $state<Song[]>([])
  let serverLoading = $state(false)
  let editingName = $state(false)
  let nameValue = $state('')
  let bpm = $state(0)

  onMount(async () => {
    if (serverOnlyId) {
      serverLoading = true
      try {
        const r = await Api.serverPlaylistTracks(serverOnlyId)
        serverName = r.name
        serverTracks = r.tracks
      } catch (e) { toastError(e) }
      finally { serverLoading = false }
    } else if ($localPlaylists.length === 0) {
      loadPlaylists(false).catch(() => {})
    }
  })

  const local = $derived(serverOnlyId ? null : ($localPlaylists.find(p => p.id === id) ?? null))
  const name = $derived(serverOnlyId ? serverName : (local?.name ?? ''))
  const tracks = $derived<Song[]>(serverOnlyId ? serverTracks : (local?.tracks ?? []))
  const linked = $derived(!!serverOnlyId || !!local?.serverId)
  const totalDuration = $derived(tracks.reduce((s, t) => s + (t.duration || 0), 0))

  const BPM_RANGES = [
    { label: 'All', min: 0, max: Infinity }, { label: '<80', min: 0, max: 79 },
    { label: '80-120', min: 80, max: 120 }, { label: '120+', min: 121, max: Infinity },
  ]
  const hasBpm = $derived(tracks.some(t => t.bpm && t.bpm > 0))
  const visible = $derived(bpm === 0 ? tracks : tracks.filter(t => t.bpm && t.bpm >= BPM_RANGES[bpm].min && t.bpm <= BPM_RANGES[bpm].max))

  const play = (idx: number) => Api.setQueueSeamless(tracks, idx).catch(toastError)

  async function commitName() {
    editingName = false
    const val = nameValue.trim()
    if (!local || !val || val === local.name) return
    try { localPlaylists.set(await Api.renamePlaylist(local.id, val)) } catch (e) { toastError(e) }
  }

  async function deletePlaylist() {
    if (!local || !confirm(`Delete "${local.name}"? This cannot be undone.`)) return
    try {
      localPlaylists.set(await Api.deletePlaylist(local.id))
      navTo('playlists')
    } catch (e) { toastError(e) }
  }

  async function remove(track: Song, idx: number) {
    try {
      if (serverOnlyId) {
        await Api.removeFromServerPlaylist(serverOnlyId, idx)
        serverTracks = serverTracks.filter((_, i) => i !== idx)
      } else if (local) {
        localPlaylists.set(await Api.removePlaylistTrack(local.id, track.id))
      }
    } catch (e) { toastError(e) }
  }

  async function move(idx: number, dir: -1 | 1) {
    const to = idx + dir
    if (to < 0 || to >= tracks.length) return
    try {
      if (serverOnlyId) {
        const next = [...serverTracks]
        next.splice(to, 0, next.splice(idx, 1)[0])
        serverTracks = next
        await Api.reorderServerPlaylist(serverOnlyId, next.map(t => t.id))
      } else if (local) {
        localPlaylists.set(await Api.movePlaylistTrack(local.id, idx, to))
      }
    } catch (e) { toastError(e) }
  }

  async function rate(track: Song, rating: number) {
    await Api.setRating(track.id, rating).catch(toastError)
    const patch = (list: Song[]) => list.map(t => (t.id === track.id ? { ...t, userRating: rating || undefined } : t))
    if (serverOnlyId) serverTracks = patch(serverTracks)
  }
</script>

{#if serverLoading}
  <div class="loading-msg">Loading playlist from server…</div>
{:else if !serverOnlyId && !local}
  <div class="loading-msg">Playlist not found.</div>
{:else}
  <div class="pl-detail-header">
    <div class="pl-detail-art">
      {#if tracks.length}<PlaylistMosaic covers={tracks.map(t => t.coverArtId)} />
      {:else}<span class="icon" style="width:40px;height:40px;color:var(--muted)">{@html IconList}</span>{/if}
    </div>

    <div class="pl-detail-info">
      {#if editingName && local}
        <!-- svelte-ignore a11y_autofocus -->
        <input class="pl-inline-edit" type="text" bind:value={nameValue} maxlength="100" autofocus onblur={commitName}
          onkeydown={e => { if (e.key === 'Enter') { e.preventDefault(); commitName() } if (e.key === 'Escape') editingName = false }} />
      {:else}
        <div class="pl-detail-name" role={local ? 'button' : 'text'} tabindex={local ? 0 : -1} title={local ? 'Click to rename' : ''}
          onclick={() => { if (local) { nameValue = local.name; editingName = true } }}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && local && (nameValue = local.name, editingName = true)}>
          {name}
          {#if linked}<span class="pl-server-indicator" title="Synced with server"><span class="icon" style="width:11px;height:11px">{@html IconCloud}</span></span>{/if}
        </div>
      {/if}

      <div class="pl-detail-meta">{tracks.length} track{tracks.length !== 1 ? 's' : ''} · {formatDuration(totalDuration)}</div>

      <div class="pl-detail-actions">
        <button class="play-all-btn" onclick={() => play(0)} disabled={!tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconPlay}</span>Play All</button>
        <button class="play-all-btn shuffle-all-btn" onclick={() => Api.shuffleAndPlay(tracks).catch(toastError)} disabled={!tracks.length}><span class="icon" style="width:12px;height:12px;margin-right:6px">{@html IconShuffle}</span>Shuffle</button>
        {#if local}<button class="pl-delete-btn" onclick={deletePlaylist}>Delete</button>{/if}
      </div>
    </div>
  </div>

  {#if tracks.length === 0}
    <div class="loading-msg">{serverOnlyId ? 'No tracks in this playlist.' : 'No tracks yet. Use the + button on any song or album.'}</div>
  {:else}
    {#if hasBpm}
      <div class="filter-bar"><div class="filter-group">
        {#each BPM_RANGES as r, i}<button class="filter-chip" class:active={bpm === i} onclick={() => (bpm = i)}>BPM {r.label}</button>{/each}
      </div></div>
    {/if}
    <div class="track-list">
      <VirtualList items={visible} itemHeight={ROW_HEIGHT}>
        {#snippet children(track, idx)}
          <TrackRow {track} {idx} playing={$currentTrack?.id === track.id} displayNum={idx + 1} showAddButton={!serverOnlyId}
            onPlay={() => play(tracks.indexOf(track))} onRate={$isAuthed ? rate : undefined} onRemove={remove} onMove={move}
            isFirst={idx === 0} isLast={idx === visible.length - 1} />
        {/snippet}
      </VirtualList>
    </div>
  {/if}
{/if}
