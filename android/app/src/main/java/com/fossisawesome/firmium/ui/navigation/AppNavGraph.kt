package com.fossisawesome.firmium.ui.navigation
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.foundation.background
import androidx.navigation.NavBackStackEntry
import androidx.compose.foundation.clickable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Brush
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.*
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.automirrored.filled.PlaylistPlay
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import coil.imageLoader
import com.fossisawesome.firmium.BuildConfig
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.screens.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.ui.theme.LocalUiTheme
import com.fossisawesome.firmium.viewmodel.*
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

private data class NavDest(val route: String, val label: String, val icon: androidx.compose.ui.graphics.vector.ImageVector)

private val detailEnterTransition: AnimatedContentTransitionScope<NavBackStackEntry>.() -> EnterTransition = {
    slideInHorizontally(initialOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
        fadeIn(tween(250))
}
private val detailExitTransition: AnimatedContentTransitionScope<NavBackStackEntry>.() -> ExitTransition = {
    slideOutHorizontally(targetOffsetX = { -it / 4 }, animationSpec = tween(250)) +
        fadeOut(tween(200))
}
private val detailPopEnterTransition: AnimatedContentTransitionScope<NavBackStackEntry>.() -> EnterTransition = {
    slideInHorizontally(initialOffsetX = { -it / 4 }, animationSpec = tween(250)) +
        fadeIn(tween(200))
}
private val detailPopExitTransition: AnimatedContentTransitionScope<NavBackStackEntry>.() -> ExitTransition = {
    slideOutHorizontally(targetOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
        fadeOut(tween(250))
}

// Search and settings are top-bar icons, not bottom tabs — matches the old mobile layout.
private val bottomDests = listOf(
    NavDest("home", "Home", Icons.Default.Home),
    NavDest("music", "Music", Icons.Default.Album),
    NavDest("artists", "Artists", Icons.Default.People),
    NavDest("mix", "Mix", Icons.Default.Radio),
    NavDest("playlists", "Playlists", Icons.AutoMirrored.Filled.PlaylistPlay),
    NavDest("podcasts", "Podcasts", Icons.Default.Mic),
)

// Maps any route (including sub-pages) to its root bottom-nav section.
private fun routeSection(route: String?): String? = when {
    route == null -> null
    route.startsWith("artist/") -> "artists"
    route.startsWith("album/") -> "music"
    route.startsWith("playlist/") -> "playlists"
    route.startsWith("podcast/") -> "podcasts"
    else -> route
}

@Composable
fun AppNavGraph(
    auth: AuthManager,
    authViewModel: AuthViewModel,
    playerViewModel: PlayerViewModel,
    libraryViewModel: LibraryViewModel,
    searchViewModel: SearchViewModel,
    playlistViewModel: PlaylistViewModel,
    podcastsViewModel: PodcastsViewModel,
    currentThemeId: String,
    onThemeSelected: (String) -> Unit,
    currentUiThemeId: String,
    onUiThemeSelected: (String) -> Unit,
    currentFontFamily: String,
    onFontSelected: (String) -> Unit,
    onAccountClick: () -> Unit,
) {
    val context = LocalContext.current
    val app = context.applicationContext as FirmiumApplication
    val scope = rememberCoroutineScope()
    val colors = LocalFirmiumColors.current

    val navController = rememberNavController()
    val playerState by playerViewModel.state.collectAsStateWithLifecycle()
    val lyricsState by playerViewModel.lyricsState.collectAsStateWithLifecycle()
    val playlistsState by playlistViewModel.state.collectAsStateWithLifecycle()
    val podcastChannels by podcastsViewModel.channels.collectAsStateWithLifecycle()
    val podcastEpisodes by podcastsViewModel.episodes.collectAsStateWithLifecycle()
    val podcastAddError by podcastsViewModel.addError.collectAsStateWithLifecycle()
    val podcastPlayingEpisodeId by podcastsViewModel.playingEpisodeId.collectAsStateWithLifecycle()

    val lrclibEnabled by app.prefs.lrclibEnabled.collectAsStateWithLifecycle(initialValue = true)
    val lyricsWordFillEnabled by app.prefs.lyricsWordFillEnabled.collectAsStateWithLifecycle(initialValue = true)
    val lastfmEnabled by app.prefs.lastfmEnabled.collectAsStateWithLifecycle(initialValue = false)
    val autoLoginEnabled by app.prefs.autoLoginEnabled.collectAsStateWithLifecycle(initialValue = true)
    val downloadFormat by app.prefs.downloadFormat.collectAsStateWithLifecycle(initialValue = "original")

    // Genre names for the Mood Mix filter (server only; empty in local-library mode).
    var mixGenres by remember { mutableStateOf<List<String>>(emptyList()) }
    LaunchedEffect(auth.isAuthenticated) {
        mixGenres = if (auth.isAuthenticated) try { app.api.getGenres() } catch (_: Exception) { emptyList() } else emptyList()
    }

    // Download callbacks — only offered when connected to a server (local-library tracks are
    // already on disk). Returned as suspend lambdas so DownloadButton can drive its own state.
    val onDownloadTrack: ((Song) -> suspend () -> Result<Unit>)? = if (auth.isAuthenticated) {
        // In server mode allow re-downloading a track even if a local copy already exists.
        { song -> { app.downloadManager.downloadTrack(song, downloadFormat, allowRedownload = true) } }
    } else null
    val onDownloadAlbum: ((Album) -> suspend () -> Result<Unit>)? = if (auth.isAuthenticated) {
        { album ->
            {
                try {
                    val full = app.api.getAlbumDetail(album.id)
                    app.downloadManager.downloadAlbum(full, downloadFormat)
                } catch (e: Exception) {
                    Result.failure(e)
                }
            }
        }
    } else null

    var lastfmApiKey by remember { mutableStateOf("") }
    var lastfmSecret by remember { mutableStateOf("") }
    LaunchedEffect(Unit) {
        lastfmApiKey = app.secureStorage.get("lastfm", "api_key") ?: ""
        lastfmSecret = app.secureStorage.get("lastfm", "secret") ?: ""
    }
    // Fetch server playlists up front so "Add to playlist" dialogs can offer
    // server-only playlists (not just locally-created ones).
    LaunchedEffect(Unit) { playlistViewModel.refreshServerPlaylists() }

    // Weekly Recap auto-show: surface once every 7 days on app open.
    LaunchedEffect(Unit) {
        val last = app.prefs.recapLastShown.first()
        if (System.currentTimeMillis() - last > 7L * 86400 * 1000) {
            app.prefs.setRecapLastShown(System.currentTimeMillis())
            navController.navigate("recap")
        }
    }

    var showFullPlayer by remember { mutableStateOf(false) }
    var showQueue by remember { mutableStateOf(false) }
    var showSimilarTracks by remember { mutableStateOf(false) }
    val similarTracksState by playerViewModel.similarTracksState.collectAsStateWithLifecycle()

    // Pending album-add-to-playlist: load tracks on demand, then show the dialog.
    var pendingAddAlbumId by remember { mutableStateOf<String?>(null) }
    var pendingAddAlbumTracks by remember { mutableStateOf<List<Song>?>(null) }
    LaunchedEffect(pendingAddAlbumId) {
        val id = pendingAddAlbumId ?: return@LaunchedEffect
        val tracks = try { app.api.getAlbumDetail(id).tracks } catch (_: Exception) { emptyList() }
        pendingAddAlbumTracks = tracks
    }

    val coverUrl: (String?) -> String? = { id ->
        if (!auth.isAuthenticated) null
        else id?.let { if (it.startsWith("file://")) it else auth.coverArtUrl(it, 300) }
    }

    val currentRoute = navController.currentBackStackEntryAsState().value?.destination?.route
    // Main tab routes — those that show the shared top bar and bottom nav.
    val mainRoutes = setOf("home", "music", "artists", "playlists")
    val routeTitle = mapOf("home" to "Home", "music" to "Music", "artists" to "Artists", "playlists" to "Playlists")
    // Section of the current route for highlighting the correct bottom nav item.
    val currentSection = routeSection(currentRoute)

    // Use a side navigation rail on medium/large screens (≥600dp width — tablets, foldables open).
    val configuration = LocalConfiguration.current
    val useRailNav = configuration.screenWidthDp >= 600

    val onNavigate: (String) -> Unit = { destRoute ->
        when {
            currentRoute == destRoute -> Unit
            currentSection == destRoute -> {
                // Pop back to the section root. If the route isn't in the back stack (e.g. an
                // artist page opened from the home tab), popBackStack returns false — fall through
                // to a normal navigate so the button still works.
                val popped = navController.popBackStack(destRoute, inclusive = false)
                if (!popped) navController.navigate(destRoute) {
                    popUpTo("home") { saveState = true }
                    launchSingleTop = true
                    restoreState = true
                }
            }
            else -> navController.navigate(destRoute) {
                popUpTo("home") { saveState = true }
                launchSingleTop = true
                restoreState = true
            }
        }
    }
    val onSearchClick: () -> Unit = {
        navController.navigate("search") {
            popUpTo("home") { saveState = true }
            launchSingleTop = true
            restoreState = true
        }
    }
    val onSettingsClick: () -> Unit = {
        navController.navigate("settings") {
            popUpTo("home") { saveState = true }
            launchSingleTop = true
            restoreState = true
        }
    }

    Row(modifier = Modifier.fillMaxSize()) {
        // Side navigation rail for wide screens (replaces bottom bar).
        if (useRailNav) {
            FirmiumNavRail(
                currentSection = currentSection,
                destinations = bottomDests,
                onNavigate = onNavigate,
                onSearchClick = onSearchClick,
                onSettingsClick = onSettingsClick,
                onAccountClick = onAccountClick,
            )
        }

    Column(modifier = Modifier.weight(1f).fillMaxHeight()) {
        // Top bar — shown on main tab routes; on rail nav the search/settings are in the rail.
        if (currentRoute in mainRoutes) {
            FirmiumPageHeader(
                title = routeTitle[currentRoute] ?: routeTitle[currentSection] ?: "",
                onSearchClick = if (useRailNav) null else onSearchClick,
                onSettingsClick = if (useRailNav) null else onSettingsClick,
                onAccountClick = if (useRailNav) null else onAccountClick,
            )
        }

        // Gradient background from header color (bg) to nav bar color (surface)
        Box(modifier = Modifier.weight(1f).background(
            Brush.verticalGradient(listOf(colors.bg, colors.surface))
        )) {
            val homeState by libraryViewModel.homeState.collectAsStateWithLifecycle()
            val albumListState by libraryViewModel.albumListState.collectAsStateWithLifecycle()
            val artistListState by libraryViewModel.artistListState.collectAsStateWithLifecycle()
            val albumDetailState by libraryViewModel.albumDetailState.collectAsStateWithLifecycle()
            val artistDetailState by libraryViewModel.artistDetailState.collectAsStateWithLifecycle()
            val searchState by searchViewModel.state.collectAsStateWithLifecycle()

            // Default tab transitions: simple fade. Detail screens slide in from the right.
            NavHost(
                navController = navController,
                startDestination = "home",
                enterTransition = { fadeIn(animationSpec = tween(220)) },
                exitTransition = { fadeOut(animationSpec = tween(180)) },
                popEnterTransition = { fadeIn(animationSpec = tween(220)) },
                popExitTransition = { fadeOut(animationSpec = tween(180)) },
            ) {
                composable("home") {
                    HomeScreen(
                        state = homeState,
                        username = auth.credentials?.username ?: "",
                        coverUrlFor = coverUrl,
                        onAlbumClick = { navController.navigate("album/$it") },
                        onArtistClick = { navController.navigate("artist/$it") },
                        onRefresh = { libraryViewModel.loadHome() },
                    )
                }
                composable("mix") {
                    MixScreen(
                        genres = mixGenres,
                        onStartMix = { energy, genre -> playerViewModel.playMoodMix(energy, genre) },
                    )
                }
                composable("music") {
                    AlbumListScreen(
                        state = albumListState,
                        coverUrlFor = coverUrl,
                        playlistItems = playlistsState.items,
                        onAlbumClick = { navController.navigate("album/$it") },
                        onLoad = { libraryViewModel.loadAlbums() },
                        onAddAlbumToPlaylist = { item, albumId ->
                            scope.launch {
                                val tracks = try { app.api.getAlbumDetail(albumId).tracks } catch (_: Exception) { emptyList() }
                                if (tracks.isNotEmpty()) playlistViewModel.addTracksTo(item, tracks)
                            }
                        },
                        onCreatePlaylistAndAddAlbum = { name, albumId ->
                            scope.launch {
                                val tracks = try { app.api.getAlbumDetail(albumId).tracks } catch (_: Exception) { emptyList() }
                                if (tracks.isNotEmpty()) playlistViewModel.createAndAdd(name, tracks)
                            }
                        },
                        onDownloadAlbum = onDownloadAlbum,
                    )
                }
                composable(
                    "album/{albumId}",
                    enterTransition = detailEnterTransition,
                    exitTransition = detailExitTransition,
                    popEnterTransition = detailPopEnterTransition,
                    popExitTransition = detailPopExitTransition,
                ) { back ->
                    val id = back.arguments?.getString("albumId") ?: return@composable
                    AlbumDetailScreen(
                        albumId = id,
                        state = albumDetailState,
                        coverUrlFor = coverUrl,
                        playlistItems = playlistsState.items,
                        onLoad = { libraryViewModel.loadAlbumDetail(it) },
                        onPlayAll = { songs, idx -> playerViewModel.playAt(songs, idx) },
                        onAddToPlaylist = { item, songs -> playlistViewModel.addTracksTo(item, songs) },
                        onCreatePlaylistAndAdd = { name, songs ->
                            playlistViewModel.createAndAdd(name, songs)
                        },
                        // Wrap downloads so the album/track downloaded marks refresh on success.
                        onDownloadTrack = onDownloadTrack?.let { base ->
                            { song -> { base(song)().also { if (it.isSuccess) libraryViewModel.refreshAlbumDownloaded() } } }
                        },
                        onDownloadAlbum = onDownloadAlbum?.let { base ->
                            { album -> { base(album)().also { if (it.isSuccess) libraryViewModel.refreshAlbumDownloaded() } } }
                        },
                        onArtistClick = { navController.navigate("artist/$it") },
                        onBack = { navController.popBackStack() },
                    )
                }
                composable("artists") {
                    ArtistListScreen(
                        state = artistListState,
                        coverUrlFor = coverUrl,
                        onArtistClick = { navController.navigate("artist/$it") },
                        onLoad = { libraryViewModel.loadArtists() },
                    )
                }
                composable(
                    "artist/{artistId}",
                    enterTransition = detailEnterTransition,
                    exitTransition = detailExitTransition,
                    popEnterTransition = detailPopEnterTransition,
                    popExitTransition = detailPopExitTransition,
                ) { back ->
                    val id = back.arguments?.getString("artistId") ?: return@composable
                    ArtistDetailScreen(
                        artistId = id,
                        state = artistDetailState,
                        coverUrlFor = coverUrl,
                        onLoad = { libraryViewModel.loadArtistDetail(it) },
                        onAlbumClick = { navController.navigate("album/$it") },
                        onPlayAlbum = { album ->
                            scope.launch {
                                val tracks = try { app.api.getAlbumDetail(album.id).tracks } catch (_: Exception) { emptyList() }
                                if (tracks.isNotEmpty()) playerViewModel.playAt(tracks, 0)
                            }
                        },
                        onPlaySongs = { songs, idx -> playerViewModel.playAt(songs, idx) },
                        onBack = { navController.popBackStack() },
                        recommendations = artistDetailState.recommendations,
                        onArtistClick = { navController.navigate("artist/$it") },
                        onStartRadio = artistDetailState.detail?.albums?.firstOrNull()?.let { firstAlbum ->
                            {
                                scope.launch {
                                    val tracks = try { app.api.getAlbumDetail(firstAlbum.id).tracks } catch (_: Exception) { emptyList() }
                                    tracks.firstOrNull()?.let { playerViewModel.startRadio(it) }
                                }
                            }
                        },
                    )
                }
                composable("playlists") {
                    PlaylistsScreen(
                        state = playlistsState,
                        coverUrlFor = coverUrl,
                        onPlaylistClick = { navController.navigate("playlist/$it") },
                        onCreate = { playlistViewModel.create(it) },
                        onDelete = { playlistViewModel.delete(it) },
                        onSync = { playlistViewModel.syncNow(it) },
                        onRefreshServer = { playlistViewModel.refreshServerPlaylists() },
                    )
                }
                composable(
                    "playlist/{playlistId}",
                    enterTransition = detailEnterTransition,
                    exitTransition = detailExitTransition,
                    popEnterTransition = detailPopEnterTransition,
                    popExitTransition = detailPopExitTransition,
                ) { back ->
                    val id = back.arguments?.getString("playlistId") ?: return@composable
                    if (id.startsWith("server-")) {
                        val serverId = id.removePrefix("server-")
                        val server = playlistsState.serverPlaylists.find { it.id == serverId }
                        val serverTracksMap by playlistViewModel.serverTracks.collectAsStateWithLifecycle()
                        val cached = serverTracksMap[serverId]
                        LaunchedEffect(serverId) { playlistViewModel.loadServerPlaylistTracks(serverId) }
                        if (server != null) {
                            val serverTracks = cached?.tracks ?: emptyList()
                            var dlIds by remember(serverId) { mutableStateOf(emptySet<String>()) }
                            LaunchedEffect(serverTracks) { dlIds = app.localLibrary.downloadedIds(serverTracks) }
                            PlaylistDetailScreen(
                                title = server.name,
                                tracks = serverTracks,
                                isServerOnly = true,
                                serverLoading = cached == null,
                                onPlayAll = { songs, idx -> playerViewModel.playAt(songs, idx) },
                                onRemoveTrack = { _, index -> playlistViewModel.removeServerTrack(serverId, index) },
                                onMoveTrack = { from, to -> playlistViewModel.moveServerTrack(serverId, from, to) },
                                onDownloadTrack = onDownloadTrack,
                                downloadedSongIds = dlIds,
                                onBack = { navController.popBackStack() },
                            )
                        }
                    } else {
                        val playlist = playlistsState.playlists.find { it.id == id }
                        if (playlist != null) {
                            var dlIds by remember(id) { mutableStateOf(emptySet<String>()) }
                            LaunchedEffect(playlist.tracks) { dlIds = app.localLibrary.downloadedIds(playlist.tracks) }
                            PlaylistDetailScreen(
                                title = playlist.name,
                                tracks = playlist.tracks,
                                onPlayAll = { songs, idx -> playerViewModel.playAt(songs, idx) },
                                onRemoveTrack = { trackId, _ -> playlistViewModel.removeTrack(id, trackId) },
                                onMoveTrack = { from, to -> playlistViewModel.moveTrack(id, from, to) },
                                onDownloadTrack = onDownloadTrack,
                                downloadedSongIds = dlIds,
                                onBack = { navController.popBackStack() },
                            )
                        }
                    }
                }
                composable("podcasts") {
                    LaunchedEffect(Unit) { podcastsViewModel.loadChannels() }
                    PodcastsScreen(
                        channels = podcastChannels,
                        addError = podcastAddError,
                        onChannelClick = { navController.navigate("podcast/$it") },
                        onAddChannel = { podcastsViewModel.addChannel(it) },
                    )
                }
                composable(
                    "podcast/{channelId}",
                    enterTransition = detailEnterTransition,
                    exitTransition = detailExitTransition,
                    popEnterTransition = detailPopEnterTransition,
                    popExitTransition = detailPopExitTransition,
                ) { back ->
                    val channelId = back.arguments?.getString("channelId") ?: return@composable
                    val channel = podcastChannels.find { it.id == channelId }
                    LaunchedEffect(channelId) { podcastsViewModel.loadEpisodes(channelId) }
                    if (channel != null) {
                        PodcastDetailScreen(
                            title = channel.title,
                            episodes = podcastEpisodes,
                            playingEpisodeId = podcastPlayingEpisodeId,
                            onRefresh = { podcastsViewModel.refreshChannel(channelId, channel.feedUrl) },
                            onUnsubscribe = {
                                podcastsViewModel.unsubscribe(channelId)
                                navController.popBackStack()
                            },
                            onPlayEpisode = { podcastsViewModel.playEpisode(it) },
                        )
                    }
                }
                composable("search") {
                    SearchScreen(
                        state = searchState,
                        coverUrlFor = coverUrl,
                        playlistItems = playlistsState.items,
                        onBack = { navController.popBackStack() },
                        onQueryChange = { searchViewModel.onQueryChanged(it) },
                        onSearch = { searchViewModel.onQueryChanged(searchState.query) },
                        onPlaySong = { songs, idx -> playerViewModel.playAt(songs, idx) },
                        onAlbumClick = { navController.navigate("album/$it") },
                        onAddSongToPlaylist = { item, song -> playlistViewModel.addTracksTo(item, listOf(song)) },
                        onCreatePlaylistAndAddSong = { name, song -> playlistViewModel.createAndAdd(name, listOf(song)) },
                        onRatingFilterChange = { searchViewModel.setRatingFilter(it) },
                        onSetRating = { id, rating -> searchViewModel.setRating(id, rating) },
                        onAddAlbum = { albumId -> pendingAddAlbumId = albumId },
                        onDownloadAlbum = onDownloadAlbum,
                        onDownloadTrack = onDownloadTrack,
                    )
                }
                composable("settings") {
                    SettingsScreen(
                        playerState = playerState,
                        serverUrl = auth.credentials?.server ?: "",
                        username = auth.credentials?.username ?: "",
                        appVersion = BuildConfig.VERSION_NAME,
                        currentThemeId = currentThemeId,
                        currentUiThemeId = currentUiThemeId,
                        currentFontFamily = currentFontFamily,
                        lrclibEnabled = lrclibEnabled,
                        lyricsWordFillEnabled = lyricsWordFillEnabled,
                        lastfmEnabled = lastfmEnabled,
                        lastfmApiKey = lastfmApiKey,
                        lastfmSecret = lastfmSecret,
                        autoLoginEnabled = autoLoginEnabled,
                        downloadFormat = downloadFormat,
                        onCrossfadeToggle = { playerViewModel.setCrossfadeEnabled(it) },
                        onCrossfadeDurationChange = { playerViewModel.setCrossfadeDuration(it) },
                        onCrossfadeCurveChange = { playerViewModel.setCrossfadeCurve(it) },
                        onGaplessToggle = { playerViewModel.setGaplessEnabled(it) },
                        onReplayGainToggle = { playerViewModel.setReplayGainEnabled(it) },
                        onThemeSelected = onThemeSelected,
                        onUiThemeSelected = onUiThemeSelected,
                        onFontSelected = onFontSelected,
                        onVisualizerToggle = { playerViewModel.setVisualizerEnabled(it) },
                        onVisualizerTypeSelected = { playerViewModel.setVisualizerType(it) },
                        onLrclibToggle = { scope.launch { app.prefs.setLrclibEnabled(it) } },
                        onLyricsWordFillToggle = { scope.launch { app.prefs.setLyricsWordFillEnabled(it) } },
                        onLastfmToggle = { scope.launch { app.prefs.setLastfmEnabled(it) } },
                        onLastfmApiKeyChange = { key ->
                            lastfmApiKey = key
                            app.secureStorage.save("lastfm", "api_key", key)
                        },
                        onLastfmSecretChange = { secret ->
                            lastfmSecret = secret
                            app.secureStorage.save("lastfm", "secret", secret)
                        },
                        onAutoLoginToggle = { scope.launch { app.prefs.setAutoLoginEnabled(it) } },
                        onDownloadFormatSelected = { scope.launch { app.prefs.setDownloadFormat(it) } },
                        onWipeCache = {
                            context.imageLoader.diskCache?.clear()
                            context.imageLoader.memoryCache?.clear()
                        },
                        onClearCache = {
                            context.cacheDir.listFiles()?.forEach { it.deleteRecursively() }
                        },
                        onResetSettings = {
                            scope.launch {
                                app.prefs.clear()
                                app.secureStorage.delete("lastfm", "api_key")
                                app.secureStorage.delete("lastfm", "secret")
                                lastfmApiKey = ""
                                lastfmSecret = ""
                            }
                        },
                        onLogout = { authViewModel.logout() },
                        onViewRecap = { navController.navigate("recap") },
                    )
                }
                composable("recap") {
                    RecapScreen(
                        repository = app.playHistory,
                        coverUrlFor = coverUrl,
                        onClose = { navController.popBackStack() },
                    )
                }
            }
        }

        // Player bar slides up from below when playback starts, slides away when it stops.
        AnimatedVisibility(
            visible = playerState.currentTrack != null,
            enter = slideInVertically(initialOffsetY = { it }) + fadeIn(tween(300)),
            exit = slideOutVertically(targetOffsetY = { it }) + fadeOut(tween(200)),
        ) {
            PlayerBar(
                state = playerState,
                coverUrl = coverUrl(playerState.currentTrack?.coverArt),
                onBarClick = { showFullPlayer = true },
                onPlayPause = { playerViewModel.togglePlayPause() },
                onNext = { playerViewModel.skipToNext() },
                onShuffleToggle = { playerViewModel.toggleShuffle() },
                onRepeatCycle = {
                    // Cycle: none → all (repeat forever) → one (repeat once) → none
                    playerViewModel.setRepeatMode(when (playerState.repeatMode) {
                        "none" -> "all"; "all" -> "one"; else -> "none"
                    })
                },
            )
        }

        // Bottom bar only on narrow screens; wide screens use the rail nav instead.
        if (!useRailNav) {
            if (LocalUiTheme.current == "spotify") {
                SpotifyBottomBar(
                    currentSection = currentSection,
                    destinations = bottomDests,
                    onNavigate = onNavigate,
                )
            } else {
                FirmiumBottomBar(
                    currentSection = currentSection,
                    destinations = bottomDests,
                    onNavigate = onNavigate,
                )
            }
        }
    } // end inner Column
    } // end outer Row

    // Full-screen player slides up from the bottom; swipe-down (BackHandler inside) dismisses it.
    AnimatedVisibility(
        visible = showFullPlayer && playerState.currentTrack != null,
        enter = slideInVertically(
            initialOffsetY = { it },
            animationSpec = tween(durationMillis = 380, easing = FastOutSlowInEasing),
        ) + fadeIn(tween(280)),
        exit = slideOutVertically(
            targetOffsetY = { it },
            animationSpec = tween(durationMillis = 300),
        ) + fadeOut(tween(220)),
    ) {
        FullScreenPlayer(
            state = playerState,
            coverUrl = coverUrl(playerState.currentTrack?.coverArt),
            visualizerProcessor = playerState.visualizerProcessor,
            playlistItems = playlistsState.items,
            lyricsState = lyricsState,
            wordFillEnabled = lyricsWordFillEnabled,
            onDismiss = { showFullPlayer = false },
            onPlayPause = { playerViewModel.togglePlayPause() },
            onNext = { playerViewModel.skipToNext() },
            onPrevious = { playerViewModel.skipToPrevious() },
            onSeek = { fraction ->
                playerViewModel.setSeekingFlag(true)
                playerViewModel.seek(fraction * playerState.trackDuration)
            },
            onSeekStart = { playerViewModel.setSeekingFlag(true) },
            onSeekEnd = { playerViewModel.setSeekingFlag(false) },
            onVolumeChange = { playerViewModel.setVolume(it) },
            onRepeatCycle = {
                // Cycle: none → all (repeat forever) → one (repeat once) → none
                playerViewModel.setRepeatMode(when (playerState.repeatMode) {
                    "none" -> "all"; "all" -> "one"; else -> "none"
                })
            },
            onShuffleToggle = { playerViewModel.toggleShuffle() },
            onQueueOpen = { showQueue = true },
            onSimilarTracksOpen = {
                playerViewModel.fetchSimilarTracks()
                showSimilarTracks = true
            },
            onLyricsOpen = { playerViewModel.openLyrics() },
            onLyricsClose = { playerViewModel.closeLyrics() },
            onAddToPlaylist = { item ->
                playerState.currentTrack?.let { playlistViewModel.addTracksTo(item, listOf(it)) }
            },
            onCreatePlaylistAndAdd = { name ->
                playerState.currentTrack?.let { playlistViewModel.createAndAdd(name, listOf(it)) }
            },
            onStartRadio = { playerState.currentTrack?.let { playerViewModel.startRadio(it) } },
            onRate = { songId, rating -> playerViewModel.setRating(songId, rating) },
            onAddToQueue = { playerState.currentTrack?.let { playerViewModel.addToQueue(it) } },
            onViewArtist = {
                playerState.currentTrack?.artistId?.takeIf { it.isNotBlank() }?.let {
                    showFullPlayer = false
                    navController.navigate("artist/$it")
                }
            },
            onEqualizer = {
                showFullPlayer = false
                navController.navigate("settings")
            },
            onDownloadTrack = if (auth.isAuthenticated) {
                {
                    playerState.currentTrack?.let { t ->
                        scope.launch { app.downloadManager.downloadTrack(t, downloadFormat, allowRedownload = true) }
                    }
                }
            } else null,
        )
    }

    if (showQueue) {
        QueueSheet(
            queue = playerState.queue,
            currentIndex = playerState.queueIndex,
            onDismiss = { showQueue = false },
            onPlayAt = { idx -> playerViewModel.skipToIndex(idx); showQueue = false },
        )
    }

    if (showSimilarTracks) {
        SimilarTracksSheet(
            state = similarTracksState,
            onDismiss = {
                showSimilarTracks = false
                playerViewModel.clearSimilarTracks()
            },
            onPlayAt = { songs, idx ->
                playerViewModel.playAt(songs, idx)
                showSimilarTracks = false
                playerViewModel.clearSimilarTracks()
            },
        )
    }

    // Album-add-to-playlist dialog — shows once tracks finish loading.
    val tracks = pendingAddAlbumTracks
    if (pendingAddAlbumId != null && tracks != null) {
        AddToPlaylistDialog(
            items = playlistsState.items,
            onAddTo = { item ->
                playlistViewModel.addTracksTo(item, tracks)
                pendingAddAlbumId = null; pendingAddAlbumTracks = null
            },
            onCreateAndAdd = { name ->
                playlistViewModel.createAndAdd(name, tracks)
                pendingAddAlbumId = null; pendingAddAlbumTracks = null
            },
            onDismiss = { pendingAddAlbumId = null; pendingAddAlbumTracks = null },
            onStartRadio = tracks.firstOrNull()?.let { seed -> { playerViewModel.startRadio(seed); pendingAddAlbumId = null; pendingAddAlbumTracks = null } },
        )
    }
}

// Page header matching .mobile-page-header: title left, account + search + settings icons right, border-bottom.
// Pass null for onSearchClick/onSettingsClick/onAccountClick to hide those icons (used on rail-nav wide screens).
@Composable
private fun FirmiumPageHeader(
    title: String,
    onSearchClick: (() -> Unit)?,
    onSettingsClick: (() -> Unit)?,
    onAccountClick: (() -> Unit)?,
) {
    val colors = LocalFirmiumColors.current
    val spotify = LocalUiTheme.current == "spotify"

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(colors.bg)
            .windowInsetsPadding(WindowInsets.statusBars),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 16.dp, end = 4.dp, top = 10.dp, bottom = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = title,
                fontSize = if (spotify) 24.sp else 18.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = LocalAppFontFamily.current,
                color = colors.text,
                modifier = Modifier.weight(1f),
                maxLines = 1,
            )
            if (onSearchClick != null) {
                FirmiumIconButton(onClick = onSearchClick, modifier = Modifier.size(44.dp)) {
                    FirmiumIcon(Icons.Default.Search, contentDescription = "Search", tint = colors.muted)
                }
            }
            if (onSettingsClick != null) {
                FirmiumIconButton(onClick = onSettingsClick, modifier = Modifier.size(44.dp)) {
                    FirmiumIcon(Icons.Default.Settings, contentDescription = "Settings", tint = colors.muted)
                }
            }
            if (onAccountClick != null) {
                FirmiumIconButton(onClick = onAccountClick, modifier = Modifier.size(44.dp)) {
                    FirmiumIcon(Icons.Default.AccountCircle, contentDescription = "Account", tint = colors.muted)
                }
            }
        }
        FirmiumDivider()
    }
}

// Side navigation rail for medium/large screens (tablets, foldables in open state).
// Shows icons + labels vertically on the left; search and settings at the bottom.
@Composable
private fun FirmiumNavRail(
    currentSection: String?,
    destinations: List<NavDest>,
    onNavigate: (String) -> Unit,
    onSearchClick: () -> Unit,
    onSettingsClick: () -> Unit,
    onAccountClick: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier
            .fillMaxHeight()
            .width(72.dp)
            .background(colors.surface)
            .windowInsetsPadding(WindowInsets.systemBars),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Spacer(Modifier.height(12.dp))
        destinations.forEach { dest ->
            RailNavItem(dest = dest, selected = currentSection == dest.route, onNavigate = onNavigate)
            Spacer(Modifier.height(4.dp))
        }
        Spacer(Modifier.weight(1f))
        FirmiumIconButton(onClick = onSearchClick, modifier = Modifier.size(48.dp)) {
            FirmiumIcon(Icons.Default.Search, contentDescription = "Search", tint = colors.muted)
        }
        Spacer(Modifier.height(4.dp))
        FirmiumIconButton(onClick = onSettingsClick, modifier = Modifier.size(48.dp)) {
            FirmiumIcon(Icons.Default.Settings, contentDescription = "Settings", tint = colors.muted)
        }
        Spacer(Modifier.height(4.dp))
        FirmiumIconButton(onClick = onAccountClick, modifier = Modifier.size(48.dp)) {
            FirmiumIcon(Icons.Default.AccountCircle, contentDescription = "Account", tint = colors.muted)
        }
        Spacer(Modifier.height(12.dp))
    }
}

@Composable
private fun RailNavItem(dest: NavDest, selected: Boolean, onNavigate: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    val iconTint by animateColorAsState(
        targetValue = if (selected) colors.accent else colors.muted,
        animationSpec = tween(durationMillis = 200),
        label = "railTint${dest.route}",
    )
    val interactionSource = remember { MutableInteractionSource() }
    val isPressed by interactionSource.collectIsPressedAsState()
    val scale by animateFloatAsState(
        targetValue = if (isPressed) 0.82f else 1f,
        animationSpec = if (isPressed) tween(80) else spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessMedium),
        label = "railScale${dest.route}",
    )
    Box(
        modifier = Modifier
            .size(48.dp)
            .scale(scale)
            .clip(RoundedCornerShape(12.dp))
            .background(if (selected) colors.accent.copy(alpha = 0.15f) else androidx.compose.ui.graphics.Color.Transparent)
            .clickable(interactionSource = interactionSource, indication = null) { onNavigate(dest.route) },
        contentAlignment = Alignment.Center,
    ) {
        FirmiumIcon(
            imageVector = dest.icon,
            contentDescription = dest.label,
            tint = iconTint,
            modifier = Modifier.size(24.dp),
        )
    }
}

// Firmium-style bottom navigation: icon-only, flat with border-top, accent underline on active.
@Composable
private fun FirmiumBottomBar(
    currentSection: String?,
    destinations: List<NavDest>,
    onNavigate: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(colors.surface),
    ) {
        FirmiumDivider()
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.navigationBars)
                .height(56.dp),
        ) {
            // Each item wrapped in a weight Box so FirmiumNavItem has no RowScope in scope,
            // which lets AnimatedVisibility resolve to the BoxScope overload correctly.
            destinations.forEach { dest ->
                Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
                    FirmiumNavItem(
                        dest = dest,
                        selected = currentSection == dest.route,
                        onNavigate = onNavigate,
                    )
                }
            }
        }
    }
}

// Spotify-style bottom navigation: icon + label per tab, active tab tinted, no underline —
// mirrors Spotify's mobile tab bar instead of Firmium's icon-only + accent-underline style.
@Composable
private fun SpotifyBottomBar(
    currentSection: String?,
    destinations: List<NavDest>,
    onNavigate: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(colors.bg),
    ) {
        FirmiumDivider()
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.navigationBars)
                .height(64.dp),
        ) {
            destinations.forEach { dest ->
                Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
                    SpotifyNavItem(dest = dest, selected = currentSection == dest.route, onNavigate = onNavigate)
                }
            }
        }
    }
}

@Composable
private fun SpotifyNavItem(dest: NavDest, selected: Boolean, onNavigate: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    val tint by animateColorAsState(
        targetValue = if (selected) colors.text else colors.muted,
        animationSpec = tween(durationMillis = 200),
        label = "${dest.route}SpotifyTint",
    )
    val interactionSource = remember { MutableInteractionSource() }
    Column(
        modifier = Modifier
            .fillMaxSize()
            .clickable(interactionSource = interactionSource, indication = null) { onNavigate(dest.route) },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        FirmiumIcon(
            imageVector = dest.icon,
            contentDescription = dest.label,
            tint = tint,
            modifier = Modifier.size(24.dp),
        )
        Spacer(Modifier.height(3.dp))
        Text(
            text = dest.label,
            fontSize = 10.sp,
            fontFamily = LocalAppFontFamily.current,
            color = tint,
            maxLines = 1,
        )
    }
}

// Single bottom-nav tab — plain composable (no RowScope) so AnimatedVisibility resolves
// to the BoxScope overload inside the inner Box, not RowScope.
@Composable
private fun FirmiumNavItem(
    dest: NavDest,
    selected: Boolean,
    onNavigate: (String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val iconTint by animateColorAsState(
        targetValue = if (selected) colors.accent else colors.muted,
        animationSpec = tween(durationMillis = 200),
        label = "${dest.route}Tint",
    )
    // Bounce scale on tap: shrink fast, spring back with overshoot.
    val interactionSource = remember { MutableInteractionSource() }
    val isPressed by interactionSource.collectIsPressedAsState()
    val iconScale by animateFloatAsState(
        targetValue = if (isPressed) 0.78f else 1f,
        animationSpec = if (isPressed) {
            tween(durationMillis = 80, easing = LinearEasing)
        } else {
            spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessMedium)
        },
        label = "${dest.route}Scale",
    )
    Box(
        modifier = Modifier
            .fillMaxSize()
            .clickable(
                interactionSource = interactionSource,
                indication = null,
            ) { onNavigate(dest.route) },
    ) {
        FirmiumIcon(
            imageVector = dest.icon,
            contentDescription = dest.label,
            tint = iconTint,
            modifier = Modifier.align(Alignment.Center).scale(iconScale),
        )
        // Accent underline fades in/out for the active destination.
        AnimatedVisibility(
            visible = selected,
            enter = fadeIn(tween(200)),
            exit = fadeOut(tween(200)),
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth(),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(2.dp)
                    .background(colors.accent),
            )
        }
    }
}
