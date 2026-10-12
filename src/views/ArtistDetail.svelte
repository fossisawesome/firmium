<script lang="ts">
  import { IconPlay, IconLoading, IconShuffle, IconHeart, IconHeartOutline } from '../lib/icons'
  import LoadingState from '../components/LoadingState.svelte'
  import AlbumRow from '../components/AlbumRow.svelte'
  import VirtualList from '../lib/VirtualList.svelte'
  import { Api } from '../lib/api'
  import { artistPhoto, DEFAULT_AVATAR } from '../lib/cover'
  import { errorMessage } from '../lib/ipc'
  import { showMenu } from '../lib/menu'
  import { navTo, dataVersion, isAuthed, toast, toastError } from '../lib/stores'
  import type { Album, Artist } from '../lib/types'

  const ROW_HEIGHT = 60
  let { id }: { id: string } = $props()

  let name = $state('')
  let albums = $state<Album[]>([])
  let loading = $state(true)
  let error = $state('')
  let bio = $state('Fetching biography…')
  let image = $state<string | null>(null)
  let recommendations = $state<Artist[]>([])
  let busy = $state<'play' | 'shuffle' | null>(null)
  let starred = $state(false)

  const stripTags = (s: string) => s.replace(/<[^>]+>/g, '').trim()

  const groups = $derived.by(() => {
    const g: Record<'Albums' | 'EPs' | 'Singles', Album[]> = { Albums: [], EPs: [], Singles: [] }
    for (const a of albums) {
      const type = (a.releaseType || '').toLowerCase()
      if (type === 'single') g.Singles.push(a)
      else if (type === 'ep') g.EPs.push(a)
      else g.Albums.push(a)
    }
    return g
  })

  $effect(() => {
    $dataVersion
    const artistId = id
    let live = true
    loading = true
    error = ''
    bio = 'Fetching biography…'
    image = null
    recommendations = []
    ;(async () => {
      try {
        const d = await Api.artistDetails(artistId)
        if (!live) return
        name = d.name
        albums = d.albums
        Api.artistInfo(artistId, d.name).then(info => {
          if (!live) return
          image = info?.image ?? null
          bio = info?.bio ? stripTags(info.bio) : 'Biography not available.'
        }).catch(() => { if (live) bio = 'Biography not available.' })
        Api.artistRecommendations(artistId, d.name).then(r => { if (live) recommendations = r }).catch(() => {})
      } catch (e) { if (live) error = errorMessage(e) }
      finally { if (live) loading = false }
    })()
    return () => { live = false }
  })

  async function playAll(shuffle: boolean) {
    busy = shuffle ? 'shuffle' : 'play'
    try {
      if ((await Api.playArtist(id, shuffle)) === 0) toast('No playable tracks found for this artist.')
    } catch (e) { toastError(e) }
    finally { busy = null }
  }

  async function toggleStar() {
    starred = !starred
    try { await Api.setStarred(id, 'artist', starred) } catch (e) { starred = !starred; toastError(e) }
  }
</script>

<div class="artist-page-header">
  <img class="artist-img-circle" src={image ?? DEFAULT_AVATAR} alt={name} />
  <div class="artist-page-info">
    <div class="artist-page-name">{name}</div>
    <div class="artist-page-bio">{bio}</div>
    <div class="artist-page-actions">
      <button class="play-all-btn" onclick={() => playAll(false)} disabled={busy !== null}>
        <span class="icon" class:icon-spin={busy === 'play'} style="width:12px;height:12px;margin-right:6px">{@html busy === 'play' ? IconLoading : IconPlay}</span>{busy === 'play' ? 'Loading Queue…' : 'Play All Songs'}
      </button>
      <button class="play-all-btn shuffle-all-btn" onclick={() => playAll(true)} disabled={busy !== null}>
        <span class="icon" class:icon-spin={busy === 'shuffle'} style="width:12px;height:12px;margin-right:6px">{@html busy === 'shuffle' ? IconLoading : IconShuffle}</span>{busy === 'shuffle' ? 'Loading Queue…' : 'Shuffle'}
      </button>
      {#if $isAuthed}
        <button class="play-all-btn shuffle-all-btn" onclick={toggleStar} aria-label={starred ? 'Remove from favorites' : 'Add to favorites'}>
          <span class="icon" style="width:12px;height:12px">{@html starred ? IconHeart : IconHeartOutline}</span>
        </button>
      {/if}
      <button class="play-all-btn artist-add-btn" title="More options" aria-label="More options"
        onclick={e => { e.stopPropagation(); showMenu(e.currentTarget, { type: 'artist', artistId: id, artistName: name }) }}>+</button>
    </div>
  </div>
</div>

<LoadingState {loading} {error} empty={albums.length === 0} loadingMessage="Loading artist profile…" emptyMessage="No releases found.">
  {#each ['Albums', 'EPs', 'Singles'] as const as category}
    {#if groups[category].length > 0}
      <div class="release-group-title">{category}</div>
      <VirtualList items={groups[category]} itemHeight={ROW_HEIGHT}>
        {#snippet children(album)}<AlbumRow {album} />{/snippet}
      </VirtualList>
    {/if}
  {/each}

  {#if recommendations.length > 0}
    <div class="release-group-title">You might also like</div>
    <div class="reco-row">
      {#each recommendations as artist (artist.id)}
        <button class="reco-card" onclick={() => navTo('artist', artist.id)} title={artist.name}>
          <img class="reco-img" use:artistPhoto={artist.id} alt={artist.name} />
          <span class="reco-name">{artist.name}</span>
        </button>
      {/each}
    </div>
  {/if}
</LoadingState>

<style>
  .reco-row { display: flex; flex-wrap: wrap; gap: 16px; padding: 8px 0 24px; }
  .reco-card { display: flex; flex-direction: column; align-items: center; gap: 8px; width: 96px; background: none; border: none; cursor: pointer; color: var(--text); }
  .reco-img { width: 72px; height: 72px; border-radius: 50%; object-fit: cover; background: var(--surface2); border: 1px solid var(--border); }
  .reco-name { font-size: 13px; text-align: center; line-height: 1.3; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
  .reco-card:hover .reco-name { color: var(--accent); }
</style>
