package com.fossisawesome.firmium.ui.navigation

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
import androidx.compose.foundation.background
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
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.screens.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.*
import kotlinx.coroutines.launch

private data class NavDest(val route: String, val label: String, val icon: androidx.compose.ui.graphics.vector.ImageVector)

// Search and settings are top-bar icons, not bottom tabs — matches the old mobile layout.
private val bottomDests = listOf(
    NavDest("home", "Home", Icons.Default.Home),
    NavDest("music", "Music", Icons.Default.Album),
    NavDest("artists", "Artists", Icons.Default.People),
    NavDest("playlists", "Playlists", Icons.AutoMirrored.Filled.PlaylistPlay),
)

// Maps any route (including sub-pages) to its root bottom-nav section.
private fun routeSection(route: String?): String? = when {
    route == null -> null
    route.startsWith("artist/") -> "artists"
    route.startsWith("album/") -> "music"
    route.startsWith("playlist/") -> "playlists"
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
    currentThemeId: String,
    onThemeSelected: (String) -> Unit,
) {
    val context = LocalContext.current
    val app = context.applicationContext as FirmiumApplication
    val scope = rememberCoroutineScope()
    val colors = LocalFirmiumColors.current

    val navController = rememberNavController()
    val playerState by playerViewModel.state.collectAsStateWithLifecycle()
    val lyricsState by playerViewModel.lyricsState.collectAsStateWithLifecycle()
    val playlistsState by playlistViewModel.state.collectAsStateWithLifecycle()

    val lrclibEnabled by app.prefs.lrclibEnabled.collectAsStateWithLifecycle(initialValue = true)
    val lastfmEnabled by app.prefs.lastfmEnabled.collectAsStateWithLifecycle(initialValue = false)
    val autoLoginEnabled by app.prefs.autoLoginEnabled.collectAsStateWithLifecycle(initialValue = true)

    var lastfmApiKey by remember { mutableStateOf("") }
    var lastfmSecret by remember { mutableStateOf("") }
    LaunchedEffect(Unit) {
        lastfmApiKey = app.secureStorage.get("lastfm", "api_key") ?: ""
        lastfmSecret = app.secureStorage.get("lastfm", "secret") ?: ""
    }

    var showFullPlayer by remember { mutableStateOf(false) }
    var showQueue by remember { mutableStateOf(false) }
    var showLyrics by remember { mutableStateOf(false) }

    // Pending album-add-to-playlist: load tracks on demand, then show the dialog.
    var pendingAddAlbumId by remember { mutableStateOf<String?>(null) }
    var pendingAddAlbumTracks by remember { mutableStateOf<List<Song>?>(null) }
    LaunchedEffect(pendingAddAlbumId) {
        val id = pendingAddAlbumId ?: return@LaunchedEffect
        val tracks = try { app.api.getAlbumDetail(id).tracks } catch (_: Exception) { emptyList() }
        pendingAddAlbumTracks = tracks
    }

    val coverUrl: (String?) -> String? = { id -> id?.let { auth.coverArtUrl(it, 300) } }

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
            )
        }

    Column(modifier = Modifier.weight(1f).fillMaxHeight()) {
        // Top bar — shown on main tab routes; on rail nav the search/settings are in the rail.
        if (currentRoute in mainRoutes) {
            FirmiumPageHeader(
                title = routeTitle[currentRoute] ?: routeTitle[currentSection] ?: "",
                onSearchClick = if (useRailNav) null else onSearchClick,
                onSettingsClick = if (useRailNav) null else onSettingsClick,
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
                composable("music") {
                    AlbumListScreen(
                        state = albumListState,
                        coverUrlFor = coverUrl,
                        playlists = playlistsState.playlists,
                        onAlbumClick = { navController.navigate("album/$it") },
                        onLoad = { libraryViewModel.loadAlbums() },
                        onAddAlbumToPlaylist = { pid, albumId ->
                            scope.launch {
                                val tracks = try { app.api.getAlbumDetail(albumId).tracks } catch (_: Exception) { emptyList() }
                                if (tracks.isNotEmpty()) playlistViewModel.addTracks(pid, tracks)
                            }
                        },
                        onCreatePlaylistAndAddAlbum = { name, albumId ->
                            scope.launch {
                                val tracks = try { app.api.getAlbumDetail(albumId).tracks } catch (_: Exception) { emptyList() }
                                if (tracks.isNotEmpty()) playlistViewModel.createAndAdd(name, tracks)
                            }
                        },
                    )
                }
                composable(
                    "album/{albumId}",
                    enterTransition = {
                        slideInHorizontally(initialOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
                            fadeIn(tween(250))
                    },
                    exitTransition = {
                        slideOutHorizontally(targetOffsetX = { -it / 4 }, animationSpec = tween(250)) +
                            fadeOut(tween(200))
                    },
                    popEnterTransition = {
                        slideInHorizontally(initialOffsetX = { -it / 4 }, animationSpec = tween(250)) +
                            fadeIn(tween(200))
                    },
                    popExitTransition = {
                        slideOutHorizontally(targetOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
                            fadeOut(tween(250))
                    },
                ) { back ->
                    val id = back.arguments?.getString("albumId") ?: return@composable
                    AlbumDetailScreen(
                        albumId = id,
                        state = albumDetailState,
                        coverUrlFor = coverUrl,
                        playlists = playlistsState.playlists,
                        onLoad = { libraryViewModel.loadAlbumDetail(it) },
                        onPlayAll = { songs, idx -> playerViewModel.playAt(songs, idx) },
                        onAddToPlaylist = { pid, songs -> playlistViewModel.addTracks(pid, songs) },
                        onCreatePlaylistAndAdd = { name, songs ->
                            playlistViewModel.createAndAdd(name, songs)
                        },
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
                    enterTransition = {
                        slideInHorizontally(initialOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
                            fadeIn(tween(250))
                    },
                    exitTransition = {
                        slideOutHorizontally(targetOffsetX = { -it / 4 }, animationSpec = tween(250)) +
                            fadeOut(tween(200))
                    },
                    popEnterTransition = {
                        slideInHorizontally(initialOffsetX = { -it / 4 }, animationSpec = tween(250)) +
                            fadeIn(tween(200))
                    },
                    popExitTransition = {
                        slideOutHorizontally(targetOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
                            fadeOut(tween(250))
                    },
                ) { back ->
                    val id = back.arguments?.getString("artistId") ?: return@composable
                    ArtistDetailScreen(
                        artistId = id,
                        state = artistDetailState,
                        coverUrlFor = coverUrl,
                        playlists = playlistsState.playlists,
                        onLoad = { libraryViewModel.loadArtistDetail(it) },
                        onAlbumClick = { navController.navigate("album/$it") },
                        onAddAlbum = { albumId -> pendingAddAlbumId = albumId },
                        onBack = { navController.popBackStack() },
                    )
                }
                composable("playlists") {
                    PlaylistsScreen(
                        state = playlistsState,
                        onPlaylistClick = { navController.navigate("playlist/$it") },
                        onCreate = { playlistViewModel.create(it) },
                        onDelete = { playlistViewModel.delete(it) },
                    )
                }
                composable(
                    "playlist/{playlistId}",
                    enterTransition = {
                        slideInHorizontally(initialOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
                            fadeIn(tween(250))
                    },
                    exitTransition = {
                        slideOutHorizontally(targetOffsetX = { -it / 4 }, animationSpec = tween(250)) +
                            fadeOut(tween(200))
                    },
                    popEnterTransition = {
                        slideInHorizontally(initialOffsetX = { -it / 4 }, animationSpec = tween(250)) +
                            fadeIn(tween(200))
                    },
                    popExitTransition = {
                        slideOutHorizontally(targetOffsetX = { it }, animationSpec = tween(300, easing = FastOutSlowInEasing)) +
                            fadeOut(tween(250))
                    },
                ) { back ->
                    val id = back.arguments?.getString("playlistId") ?: return@composable
                    val playlist = playlistsState.playlists.find { it.id == id }
                    if (playlist != null) {
                        PlaylistDetailScreen(
                            playlist = playlist,
                            onPlayAll = { songs, idx -> playerViewModel.playAt(songs, idx) },
                            onRemoveTrack = { trackId -> playlistViewModel.removeTrack(id, trackId) },
                            onBack = { navController.popBackStack() },
                        )
                    }
                }
                composable("search") {
                    SearchScreen(
                        state = searchState,
                        coverUrlFor = coverUrl,
                        playlists = playlistsState.playlists,
                        onBack = { navController.popBackStack() },
                        onQueryChange = { searchViewModel.onQueryChanged(it) },
                        onSearch = { searchViewModel.onQueryChanged(searchState.query) },
                        onPlaySong = { songs, idx -> playerViewModel.playAt(songs, idx) },
                        onAlbumClick = { navController.navigate("album/$it") },
                        onAddSongToPlaylist = { pid, song -> playlistViewModel.addTracks(pid, listOf(song)) },
                        onCreatePlaylistAndAddSong = { name, song -> playlistViewModel.createAndAdd(name, listOf(song)) },
                        onAddAlbum = { albumId -> pendingAddAlbumId = albumId },
                    )
                }
                composable("settings") {
                    SettingsScreen(
                        playerState = playerState,
                        serverUrl = auth.credentials?.server ?: "",
                        username = auth.credentials?.username ?: "",
                        appVersion = BuildConfig.VERSION_NAME,
                        currentThemeId = currentThemeId,
                        lrclibEnabled = lrclibEnabled,
                        lastfmEnabled = lastfmEnabled,
                        lastfmApiKey = lastfmApiKey,
                        lastfmSecret = lastfmSecret,
                        autoLoginEnabled = autoLoginEnabled,
                        onCrossfadeToggle = { playerViewModel.setCrossfadeEnabled(it) },
                        onCrossfadeDurationChange = { playerViewModel.setCrossfadeDuration(it) },
                        onGaplessToggle = { playerViewModel.setGaplessEnabled(it) },
                        onThemeSelected = onThemeSelected,
                        onLrclibToggle = { scope.launch { app.prefs.setLrclibEnabled(it) } },
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
                        onWipeCache = {
                            context.imageLoader.diskCache?.clear()
                            context.imageLoader.memoryCache?.clear()
                        },
                        onDeleteLogs = {
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
            )
        }

        // Bottom bar only on narrow screens; wide screens use the rail nav instead.
        if (!useRailNav) {
            FirmiumBottomBar(
                currentSection = currentSection,
                destinations = bottomDests,
                onNavigate = onNavigate,
            )
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
            playlists = playlistsState.playlists,
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
                // Cycle: none → one (repeat once) → all (repeat forever) → none
                playerViewModel.setRepeatMode(when (playerState.repeatMode) {
                    "none" -> "one"; "one" -> "all"; else -> "none"
                })
            },
            onShuffleToggle = { playerViewModel.toggleShuffle() },
            onQueueOpen = { showQueue = true },
            onLyricsOpen = {
                playerViewModel.openLyrics()
                showLyrics = true
            },
            onAddToPlaylist = { pid ->
                playerState.currentTrack?.let { playlistViewModel.addTracks(pid, listOf(it)) }
            },
            onCreatePlaylistAndAdd = { name ->
                playerState.currentTrack?.let { playlistViewModel.createAndAdd(name, listOf(it)) }
            },
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

    if (showLyrics && playerState.currentTrack != null) {
        LyricsSheet(
            state = lyricsState,
            trackTitle = playerState.currentTrack?.title ?: "",
            onDismiss = {
                showLyrics = false
                playerViewModel.closeLyrics()
            },
        )
    }

    // Album-add-to-playlist dialog — shows once tracks finish loading.
    val tracks = pendingAddAlbumTracks
    if (pendingAddAlbumId != null && tracks != null) {
        AddToPlaylistDialog(
            playlists = playlistsState.playlists,
            onAddTo = { pid ->
                playlistViewModel.addTracks(pid, tracks)
                pendingAddAlbumId = null; pendingAddAlbumTracks = null
            },
            onCreateAndAdd = { name ->
                playlistViewModel.createAndAdd(name, tracks)
                pendingAddAlbumId = null; pendingAddAlbumTracks = null
            },
            onDismiss = { pendingAddAlbumId = null; pendingAddAlbumTracks = null },
        )
    }
}

// Page header matching .mobile-page-header: title left, search + settings icons right, border-bottom.
// Pass null for onSearchClick/onSettingsClick to hide those icons (used on rail-nav wide screens).
@Composable
private fun FirmiumPageHeader(
    title: String,
    onSearchClick: (() -> Unit)?,
    onSettingsClick: (() -> Unit)?,
) {
    val colors = LocalFirmiumColors.current

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
                fontSize = 18.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = FontFamily.Monospace,
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
