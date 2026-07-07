package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.fossisawesome.firmium.BuildConfig
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.AuthViewModel
import com.fossisawesome.firmium.viewmodel.LibraryViewModel
import com.fossisawesome.firmium.viewmodel.PlayerViewModel
import com.fossisawesome.firmium.viewmodel.PlaylistViewModel
import com.fossisawesome.firmium.viewmodel.SearchViewModel
import kotlinx.coroutines.launch

private data class TvNavDest(val route: String, val label: String)

private val tvNavDests = listOf(
    TvNavDest("home", "Home"),
    TvNavDest("music", "Albums"),
    TvNavDest("artists", "Artists"),
    TvNavDest("playlists", "Playlists"),
    TvNavDest("mix", "Mix"),
    TvNavDest("search", "Search"),
    TvNavDest("settings", "Settings"),
)

@Composable
fun TvNavGraph(
    auth: AuthManager,
    authViewModel: AuthViewModel,
    playerViewModel: PlayerViewModel,
    libraryViewModel: LibraryViewModel,
    searchViewModel: SearchViewModel,
    playlistViewModel: PlaylistViewModel,
) {
    val colors = LocalFirmiumColors.current
    val navController = rememberNavController()
    val context = LocalContext.current
    val app = context.applicationContext as FirmiumApplication
    val scope = rememberCoroutineScope()

    val playerState by playerViewModel.state.collectAsStateWithLifecycle()
    val lyricsState by playerViewModel.lyricsState.collectAsStateWithLifecycle()
    val similarTracksState by playerViewModel.similarTracksState.collectAsStateWithLifecycle()
    val homeState by libraryViewModel.homeState.collectAsStateWithLifecycle()
    val albumListState by libraryViewModel.albumListState.collectAsStateWithLifecycle()
    val artistListState by libraryViewModel.artistListState.collectAsStateWithLifecycle()
    val albumDetailState by libraryViewModel.albumDetailState.collectAsStateWithLifecycle()
    val artistDetailState by libraryViewModel.artistDetailState.collectAsStateWithLifecycle()
    val playlistsState by playlistViewModel.state.collectAsStateWithLifecycle()
    val serverTracksMap by playlistViewModel.serverTracks.collectAsStateWithLifecycle()
    val searchState by searchViewModel.state.collectAsStateWithLifecycle()
    val authState by authViewModel.state.collectAsStateWithLifecycle()

    val themeId by app.prefs.themeId.collectAsStateWithLifecycle(initialValue = "firmium")
    val fontFamily by app.prefs.fontFamily.collectAsStateWithLifecycle(initialValue = "Liberation Mono")

    val coverUrl: (String?) -> String? = { id ->
        if (!auth.isAuthenticated) null
        else id?.let { if (it.startsWith("file://")) it else auth.coverArtUrl(it, 300) }
    }

    // Genre names for the Mood Mix filter (server only; empty in local-library mode).
    var mixGenres by remember { mutableStateOf<List<String>>(emptyList()) }
    LaunchedEffect(auth.isAuthenticated) {
        mixGenres = if (auth.isAuthenticated) runCatching { app.api.getGenres() }.getOrDefault(emptyList()) else emptyList()
    }

    val currentRoute = navController.currentBackStackEntryAsState().value?.destination?.route
    val showRail = tvNavDests.any { it.route == currentRoute }

    Row(modifier = Modifier.fillMaxSize().background(colors.bg)) {
        if (showRail) {
            Column(modifier = Modifier.fillMaxHeight().width(220.dp).padding(vertical = 48.dp, horizontal = 24.dp)) {
                tvNavDests.forEach { dest ->
                    val selected = currentRoute == dest.route
                    TvActionButton(
                        onClick = {
                            if (!selected) navController.navigate(dest.route) { launchSingleTop = true }
                        },
                        colors = colors,
                        modifier = Modifier.width(180.dp).padding(bottom = 12.dp),
                    ) {
                        Text(text = dest.label, color = if (selected) colors.accent else colors.text, fontSize = 15.sp)
                    }
                }
            }
        }
        Box(modifier = Modifier.fillMaxSize()) {
        NavHost(navController = navController, startDestination = "home") {
            composable("home") {
                TvHomeScreen(
                    state = homeState,
                    coverUrlFor = coverUrl,
                    onLoad = { libraryViewModel.loadHome() },
                    onAlbumClick = { navController.navigate("album/$it") },
                    onArtistClick = { navController.navigate("artist/$it") },
                )
            }
            composable("music") {
                TvAlbumListScreen(
                    state = albumListState,
                    coverUrlFor = coverUrl,
                    onLoad = { libraryViewModel.loadAlbums() },
                    onAlbumClick = { navController.navigate("album/$it") },
                )
            }
            composable("album/{albumId}") { back ->
                val id = back.arguments?.getString("albumId") ?: return@composable
                TvAlbumDetailScreen(
                    albumId = id,
                    state = albumDetailState,
                    coverUrlFor = coverUrl,
                    onLoad = { libraryViewModel.loadAlbumDetail(it) },
                    onPlayAt = { songs, idx ->
                        playerViewModel.playAt(songs, idx)
                        navController.navigate("nowplaying")
                    },
                    onBack = { navController.popBackStack() },
                )
            }
            composable("artists") {
                TvArtistListScreen(
                    state = artistListState,
                    coverUrlFor = coverUrl,
                    onLoad = { libraryViewModel.loadArtists() },
                    onArtistClick = { navController.navigate("artist/$it") },
                )
            }
            composable("artist/{artistId}") { back ->
                val id = back.arguments?.getString("artistId") ?: return@composable
                TvArtistDetailScreen(
                    artistId = id,
                    state = artistDetailState,
                    coverUrlFor = coverUrl,
                    onLoad = { libraryViewModel.loadArtistDetail(it) },
                    onAlbumClick = { navController.navigate("album/$it") },
                    onBack = { navController.popBackStack() },
                )
            }
            composable("playlists") {
                TvPlaylistListScreen(
                    state = playlistsState,
                    onLoad = { playlistViewModel.refreshServerPlaylists() },
                    onPlaylistClick = { navController.navigate("playlist/$it") },
                )
            }
            composable("playlist/{playlistId}") { back ->
                val id = back.arguments?.getString("playlistId") ?: return@composable
                if (id.startsWith("server-")) {
                    val serverId = id.removePrefix("server-")
                    val server = playlistsState.serverPlaylists.find { it.id == serverId }
                    val cached = serverTracksMap[serverId]
                    TvPlaylistDetailScreen(
                        title = server?.name ?: "",
                        tracks = cached?.tracks ?: emptyList(),
                        onLoad = { playlistViewModel.loadServerPlaylistTracks(serverId) },
                        onPlayAll = { songs, idx ->
                            playerViewModel.playAt(songs, idx)
                            navController.navigate("nowplaying")
                        },
                        onBack = { navController.popBackStack() },
                    )
                } else {
                    val playlist = playlistsState.playlists.find { it.id == id }
                    TvPlaylistDetailScreen(
                        title = playlist?.name ?: "",
                        tracks = playlist?.tracks ?: emptyList(),
                        onLoad = {},
                        onPlayAll = { songs, idx ->
                            playerViewModel.playAt(songs, idx)
                            navController.navigate("nowplaying")
                        },
                        onBack = { navController.popBackStack() },
                    )
                }
            }
            composable("search") {
                TvSearchScreen(
                    state = searchState,
                    coverUrlFor = coverUrl,
                    onQueryChange = { searchViewModel.onQueryChanged(it) },
                    onPlaySong = { songs, idx ->
                        playerViewModel.playAt(songs, idx)
                        navController.navigate("nowplaying")
                    },
                    onAlbumClick = { navController.navigate("album/$it") },
                )
            }
            composable("mix") {
                TvMixScreen(
                    genres = mixGenres,
                    onStartMix = { energy, genre ->
                        playerViewModel.playMoodMix(energy, genre) { count ->
                            if (count > 0) navController.navigate("nowplaying")
                        }
                    },
                )
            }
            composable("nowplaying") {
                TvNowPlayingScreen(
                    state = playerState,
                    coverUrl = coverUrl(playerState.currentTrack?.coverArt),
                    lyricsState = lyricsState,
                    similarTracksState = similarTracksState,
                    onPlayPause = { playerViewModel.togglePlayPause() },
                    onNext = { playerViewModel.skipToNext() },
                    onPrevious = { playerViewModel.skipToPrevious() },
                    onSkipToIndex = { playerViewModel.skipToIndex(it) },
                    onLyricsOpen = { playerViewModel.openLyrics() },
                    onLyricsClose = { playerViewModel.closeLyrics() },
                    onFetchSimilarTracks = { playerViewModel.fetchSimilarTracks() },
                    onClearSimilarTracks = { playerViewModel.clearSimilarTracks() },
                    onPlaySimilar = { songs, idx ->
                        playerViewModel.playAt(songs, idx)
                        playerViewModel.clearSimilarTracks()
                    },
                    onBack = { navController.popBackStack() },
                )
            }
            composable("settings") {
                TvSettingsScreen(
                    playerState = playerState,
                    serverUrl = auth.credentials?.server ?: "",
                    username = auth.credentials?.username ?: "",
                    appVersion = BuildConfig.VERSION_NAME,
                    currentThemeId = themeId,
                    currentFontFamily = fontFamily,
                    onThemeSelected = { id -> scope.launch { app.prefs.setThemeId(id) } },
                    onFontSelected = { name -> scope.launch { app.prefs.setFontFamily(name) } },
                    onCrossfadeToggle = { playerViewModel.setCrossfadeEnabled(it) },
                    onCrossfadeDurationChange = { playerViewModel.setCrossfadeDuration(it) },
                    onCrossfadeCurveChange = { playerViewModel.setCrossfadeCurve(it) },
                    onGaplessToggle = { playerViewModel.setGaplessEnabled(it) },
                    onReplayGainToggle = { playerViewModel.setReplayGainEnabled(it) },
                    onVisualizerToggle = { playerViewModel.setVisualizerEnabled(it) },
                    onVisualizerTypeSelected = { playerViewModel.setVisualizerType(it) },
                    onLogout = {
                        authViewModel.logout()
                        libraryViewModel.invalidateAll()
                    },
                    onOpenEqualizer = { navController.navigate("equalizer") },
                    onViewRecap = { navController.navigate("recap") },
                )
            }
            composable("equalizer") {
                TvEqualizerScreen(onBack = { navController.popBackStack() })
            }
            composable("recap") {
                TvRecapScreen(
                    repository = app.playHistory,
                    coverUrlFor = coverUrl,
                    onBack = { navController.popBackStack() },
                )
            }
        }
        }
    }
}
