<script>
  import { playlists, queue, currentTrack, navToView } from '../lib/stores.js'
  import { Api, loadImage } from '../lib/api.js'
  import { playAt } from '../lib/playback.js'
  import { showPlaylistMenu } from '../lib/playlistMenu.js'
  import { lazyLoad } from '../lib/lazyLoad.js'
  import { formatDuration } from '../lib/utils.js'

  let { id } = $props()

  let editingName = $state(false)
  let editingDesc = $state(false)
  let nameValue = $state('')
  let descValue = $state('')
  let showCoverPicker = $state(false)
  let fileInput = $state()

  const pl = $derived($playlists.find(p => p.id === id) ?? null)
  const totalDuration = $derived(pl ? pl.tracks.reduce((s, t) => s + (t.duration || 0), 0) : 0)
  const uniqueCovers = $derived(pl ? (() => {
    const seen = new Set()
    return pl.tracks.filter(t => t.coverArtId && !seen.has(t.coverArtId) && seen.add(t.coverArtId))
  })() : [])

  function startEditName() {
    if (!pl) return
    nameValue = pl.name
    editingName = true
  }

  function commitName() {
    const val = nameValue.trim() || pl?.name
    if (val && pl) playlists.updatePlaylist(id, { name: val })
    editingName = false
  }

  function startEditDesc() {
    if (!pl) return
    descValue = pl.description
    editingDesc = true
  }

  function commitDesc() {
    if (pl) playlists.updatePlaylist(id, { description: descValue.trim() })
    editingDesc = false
  }

  function handleNameKey(e) {
    if (e.key === 'Enter') { e.preventDefault(); commitName() }
    if (e.key === 'Escape') { editingName = false }
  }

  function handleDescKey(e) {
    if (e.key === 'Enter') { e.preventDefault(); commitDesc() }
    if (e.key === 'Escape') { editingDesc = false }
  }

  function playAll() {
    if (!pl || !pl.tracks.length) return
    queue.set(pl.tracks)
    playAt(0)
  }

  function deletePl() {
    if (!pl) return
    if (confirm(`Delete "${pl.name}"? This cannot be undone.`)) {
      playlists.delete(id)
      navToView('playlists')
    }
  }

  function removeTrack(trackId) {
    playlists.removeTrack(id, trackId)
  }

  function setCover(coverId) {
    playlists.updatePlaylist(id, { coverArtId: coverId, coverDataUrl: null })
    showCoverPicker = false
  }

  function handleFileUpload(e) {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = ev => {
      playlists.updatePlaylist(id, { coverDataUrl: ev.target.result, coverArtId: null })
    }
    reader.readAsDataURL(file)
  }

  function isPlaying(track) {
    return $currentTrack?.id === track.id
  }

  function playTrack(idx) {
    if (!pl) return
    queue.set(pl.tracks)
    playAt(idx)
  }
</script>

{#if !pl}
  <div class="loading-msg">Playlist not found.</div>
{:else}
  <div class="pl-detail-header">
    <div
      class="pl-detail-art"
      role="button"
      tabindex="0"
      onclick={() => showCoverPicker = !showCoverPicker}
      onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (showCoverPicker = !showCoverPicker)}
      title="Change cover"
    >
      {#if pl.coverDataUrl}
        <img src={pl.coverDataUrl} alt="" />
      {:else if pl.coverArtId}
        <img use:lazyLoad={img => loadImage(img, pl.coverArtId, null)} alt="" />
      {:else}
        ♫
      {/if}
      <div class="pl-detail-art-overlay">Change<br>Cover</div>
    </div>

    <div class="pl-detail-info">
      {#if editingName}
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
          role="button"
          tabindex="0"
          onclick={startEditName}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && startEditName()}
          title="Click to rename"
        >{pl.name}</div>
      {/if}

      {#if editingDesc}
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
          role="button"
          tabindex="0"
          onclick={startEditDesc}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && startEditDesc()}
          title="Click to edit description"
        >
          {#if pl.description}
            {pl.description}
          {:else}
            <span class="pl-detail-desc-hint">Add description…</span>
          {/if}
        </div>
      {/if}

      <div class="pl-detail-meta">
        {pl.tracks.length} track{pl.tracks.length !== 1 ? 's' : ''} · {formatDuration(totalDuration)}
      </div>

      <div class="pl-detail-actions">
        <button class="play-all-btn" onclick={playAll} disabled={!pl.tracks.length}>▶ Play All</button>
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
              onclick={() => setCover(t.coverArtId)}
              onkeydown={e => (e.key === 'Enter' || e.key === ' ') && setCover(t.coverArtId)}
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
    <div class="loading-msg">No tracks yet — use the + button on any song or album.</div>
  {:else}
    <div class="track-list">
      {#each pl.tracks as track, idx}
        <div
          class="track-row"
          class:playing={isPlaying(track)}
          role="button"
          tabindex="0"
          onclick={() => playTrack(idx)}
          onkeydown={e => (e.key === 'Enter' || e.key === ' ') && playTrack(idx)}
        >
          <div class="track-num">{idx + 1}</div>
          <div class="track-thumb">
            {#if track.coverArtId}
              <img use:lazyLoad={img => loadImage(img, track.coverArtId, null)} alt="" />
            {/if}
          </div>
          <div class="track-info">
            <div class="track-title">{track.title}</div>
            <div class="track-artist">{track.artist}</div>
          </div>
          <div class="track-duration">{formatDuration(track.duration)}</div>
          <button
            class="track-add-btn"
            title="Add to playlist"
            onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'tracks', tracks: [track] }) }}
          >+</button>
          <button
            class="track-remove-btn"
            title="Remove from playlist"
            onclick={e => { e.stopPropagation(); removeTrack(track.id) }}
          >×</button>
        </div>
      {/each}
    </div>
  {/if}
{/if}
