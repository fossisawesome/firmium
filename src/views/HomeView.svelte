<script lang="ts">
  import { IconMusic, IconUser, IconHeart, IconRecap } from '../lib/icons'
  import { auth, navTo, dataVersion } from '../lib/stores'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import type { Album, Genre, RecentPlay } from '../lib/types'

  const hour = new Date().getHours()
  const timeOfDay = hour >= 5 && hour < 12 ? 'Morning' : hour < 17 && hour >= 12 ? 'Afternoon' : hour >= 17 && hour < 21 ? 'Evening' : 'Night'
  const username = $derived($auth.username ?? 'there')

  let plays = $state<RecentPlay[]>([])
  let randomAlbums = $state<Album[]>([])
  let genres = $state<Genre[]>([])
  let loading = $state(true)

  // Recently played albums / artists, deduplicated from the play history.
  const recentAlbums = $derived.by(() => {
    const seen = new Set<string>()
    return plays.filter(p => p.albumId && !seen.has(p.albumId) && seen.add(p.albumId))
  })
  const recentArtists = $derived.by(() => {
    const seen = new Set<string>()
    return plays.filter(p => p.artistName && !seen.has(p.artistName) && seen.add(p.artistName))
  })

  $effect(() => {
    $dataVersion
    let live = true
    loading = true
    Promise.allSettled([Api.recentPlays(40), Api.albumList('random', 12), Api.genres()]).then(([p, r, g]) => {
      if (!live) return
      plays = p.status === 'fulfilled' ? p.value : []
      randomAlbums = r.status === 'fulfilled' ? r.value : []
      genres = g.status === 'fulfilled' ? g.value.slice(0, 30) : []
      loading = false
    })
    return () => { live = false }
  })
</script>

<div class="home-view">
  <div class="home-greeting">
    <span class="home-greeting-label">Good {timeOfDay},</span>
    <span class="home-greeting-name">{username}</span>
  </div>

  <div class="home-shortcuts">
    <button class="home-shortcut" onclick={() => navTo('favorites')}>
      <span class="icon" style="width:16px;height:16px">{@html IconHeart}</span>Favorites
    </button>
    <button class="home-shortcut" onclick={() => navTo('recap')}>
      <span class="icon" style="width:16px;height:16px">{@html IconRecap}</span>Recap
    </button>
  </div>

  {#if recentAlbums.length > 0}
    <section class="home-section">
      <div class="home-section-title">Recently Played</div>
      <div class="home-scroll-row">
        {#each recentAlbums as p (p.albumId)}
          <button class="home-card" onclick={() => navTo('album', p.albumId)}>
            <div class="home-card-art">
              {#if p.coverArtId}<img use:cover={p.coverArtId} alt="" />
              {:else}<div class="home-card-no-art"><span class="icon" style="width:24px;height:24px;color:var(--muted)">{@html IconMusic}</span></div>{/if}
            </div>
            <div class="home-card-info">
              <div class="home-card-title">{p.albumName}</div>
              <div class="home-card-sub">{p.artistName}</div>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if recentArtists.length > 0}
    <section class="home-section">
      <div class="home-section-title">Recently Played Artists</div>
      <div class="home-scroll-row">
        {#each recentArtists as p (p.artistName)}
          <button class="home-card home-card--artist" onclick={() => (p.artistId ? navTo('artist', p.artistId) : navTo('artists'))}>
            <div class="home-card-art home-card-art--round">
              {#if p.coverArtId}<img use:cover={p.coverArtId} alt="" />
              {:else}<div class="home-card-no-art"><span class="icon" style="width:28px;height:28px;color:var(--muted)">{@html IconUser}</span></div>{/if}
            </div>
            <div class="home-card-info"><div class="home-card-title">{p.artistName}</div></div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <section class="home-section">
    <div class="home-section-title">Random Picks</div>
    {#if loading}
      <div class="home-loading">Loading…</div>
    {:else if randomAlbums.length === 0}
      <div class="home-empty">No albums found.</div>
    {:else}
      <div class="home-scroll-row">
        {#each randomAlbums as album (album.id)}
          <button class="home-card" onclick={() => navTo('album', album.id)}>
            <div class="home-card-art">
              {#if album.coverArtId}<img use:cover={album.coverArtId} alt="" />
              {:else}<div class="home-card-no-art"><span class="icon" style="width:24px;height:24px;color:var(--muted)">{@html IconMusic}</span></div>{/if}
            </div>
            <div class="home-card-info">
              <div class="home-card-title">{album.name}</div>
              <div class="home-card-sub">{album.albumArtist}</div>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="home-section">
    <div class="home-section-title">Genres</div>
    {#if !loading && genres.length === 0}
      <div class="home-empty">No genres found.</div>
    {:else}
      <div class="home-genre-chips">
        {#each genres as genre (genre.name)}
          <button class="home-genre-chip" onclick={() => navTo('genre', genre.name)}>
            {genre.name}<span class="home-genre-count">{genre.albumCount}</span>
          </button>
        {/each}
      </div>
    {/if}
  </section>
</div>
