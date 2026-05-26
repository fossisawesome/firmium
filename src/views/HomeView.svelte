<script>
  import { onMount, onDestroy } from 'svelte'
  import { authUsername, navToAlbum, navToArtist, navToView, recentlyPlayedSongs } from '../lib/stores.js'
  import { Api, loadImage } from '../lib/api.js'
  import { lazyLoad } from '../lib/lazyLoad.js'

  // ── Greeting ──────────────────────────────────────────────────────────────
  function getTimeOfDay() {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone
    const raw = new Date().toLocaleString('en-US', { timeZone: tz, hour: 'numeric', hour12: false })
    const h = parseInt(raw, 10)
    const hour = h === 24 ? 0 : h
    if (hour >= 5 && hour < 12) return 'Morning'
    if (hour >= 12 && hour < 17) return 'Afternoon'
    if (hour >= 17 && hour < 21) return 'Evening'
    return 'Night'
  }

  const timeOfDay = getTimeOfDay()
  const username = $derived($authUsername ?? 'there')

  // ── Data ──────────────────────────────────────────────────────────────────
  let recentAlbums = $state([])
  let recentArtists = $state([])
  let randomAlbums = $state([])
  let genres = $state([])
  let loadingRecent = $state(true)
  let loadingRandom = $state(true)
  let loadingGenres = $state(true)

  let ctrl

  // Derives unique artists from recently played albums
  function extractArtists(albums) {
    const seen = new Set()
    const out = []
    for (const a of albums) {
      const key = a.albumArtist
      if (!seen.has(key)) { seen.add(key); out.push({ id: a.artistId, name: key, coverArtId: a.coverArtId }) }
    }
    return out
  }

  onMount(async () => {
    ctrl = new AbortController()
    const sig = ctrl.signal

    // Fetch recent albums + random albums + genres in parallel
    const [recentRes, randomRes, genresRes] = await Promise.allSettled([
      Api.getRecentAlbums(12, sig),
      Api.getRandomAlbums(12, sig),
      Api.getGenresList(sig),
    ])

    if (sig.aborted) return

    if (recentRes.status === 'fulfilled') {
      recentAlbums = recentRes.value
      recentArtists = extractArtists(recentRes.value)
    }
    loadingRecent = false

    if (randomRes.status === 'fulfilled') {
      randomAlbums = randomRes.value
    }
    loadingRandom = false

    if (genresRes.status === 'fulfilled') {
      genres = genresRes.value.slice(0, 30)
    }
    loadingGenres = false

  })

  onDestroy(() => ctrl?.abort())

  // Derives unique albums from recently played songs (deduplicated by albumId)
  const recentAlbumsFromSongs = $derived((() => {
    const seen = new Set()
    const out = []
    for (const s of $recentlyPlayedSongs) {
      const key = s.albumId
      if (key && !seen.has(key)) {
        seen.add(key)
        out.push({ id: s.albumId, name: s.album, artist: s.artist, coverArtId: s.coverArtId })
      }
    }
    return out
  })())

  // ── Navigate to genre search ──────────────────────────────────────────────
  function navToGenre(genre) {
    navToView('search')
    // Dispatch after view switches so SearchView can receive it
    setTimeout(() => {
      window.dispatchEvent(new CustomEvent('firmium:search-genre', { detail: genre }))
    }, 50)
  }
</script>

<div class="home-view">
  <!-- Greeting -->
  <div class="home-greeting">
    <span class="home-greeting-label">Good {timeOfDay},</span>
    <span class="home-greeting-name">{username}</span>
  </div>

  <!-- Recently Played (unique albums from play history) -->
  {#if recentAlbumsFromSongs.length > 0}
    <section class="home-section">
      <div class="home-section-title">Recently Played</div>
      <div class="home-scroll-row">
        {#each recentAlbumsFromSongs as album}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="home-card" onclick={() => navToAlbum(album.id)}>
            <div class="home-card-art">
              {#if album.coverArtId}
                <img use:lazyLoad={img => loadImage(img, album.coverArtId, ctrl?.signal)} alt="" />
              {:else}
                <div class="home-card-no-art">♪</div>
              {/if}
            </div>
            <div class="home-card-info">
              <div class="home-card-title">{album.name}</div>
              <div class="home-card-sub">{album.artist}</div>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Recently Played Artists -->
  <section class="home-section">
    <div class="home-section-title">Recently Played Artists</div>
    {#if loadingRecent}
      <div class="home-loading">Loading…</div>
    {:else if recentArtists.length === 0}
      <div class="home-empty">No recent artists found.</div>
    {:else}
      <div class="home-scroll-row">
        {#each recentArtists as artist}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="home-card home-card--artist" onclick={() => artist.id ? navToArtist(artist.id) : navToView('artists')}>
            <div class="home-card-art home-card-art--round">
              {#if artist.coverArtId}
                <img use:lazyLoad={img => loadImage(img, artist.coverArtId, ctrl?.signal)} alt="" />
              {:else}
                <div class="home-card-no-art">♬</div>
              {/if}
            </div>
            <div class="home-card-info">
              <div class="home-card-title">{artist.name}</div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Random Albums -->
  <section class="home-section">
    <div class="home-section-title">Random Picks</div>
    {#if loadingRandom}
      <div class="home-loading">Loading…</div>
    {:else if randomAlbums.length === 0}
      <div class="home-empty">No albums found.</div>
    {:else}
      <div class="home-scroll-row">
        {#each randomAlbums as album}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="home-card" onclick={() => navToAlbum(album.id)}>
            <div class="home-card-art">
              {#if album.coverArtId}
                <img use:lazyLoad={img => loadImage(img, album.coverArtId, ctrl?.signal)} alt="" />
              {:else}
                <div class="home-card-no-art">♪</div>
              {/if}
            </div>
            <div class="home-card-info">
              <div class="home-card-title">{album.name}</div>
              <div class="home-card-sub">{album.albumArtist}</div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Genres -->
  <section class="home-section">
    <div class="home-section-title">Genres</div>
    {#if loadingGenres}
      <div class="home-loading">Loading…</div>
    {:else if genres.length === 0}
      <div class="home-empty">No genres found.</div>
    {:else}
      <div class="home-genre-chips">
        {#each genres as genre}
          <button class="home-genre-chip" onclick={() => navToGenre(genre.name)}>
            {genre.name}
            <span class="home-genre-count">{genre.albumCount}</span>
          </button>
        {/each}
      </div>
    {/if}
  </section>
</div>
