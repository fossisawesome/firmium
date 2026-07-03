package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.navigation.NavType
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.navArgument
import androidx.wear.compose.material.CircularProgressIndicator
import androidx.wear.compose.navigation.SwipeDismissableNavHost
import androidx.wear.compose.navigation.composable
import androidx.wear.compose.navigation.rememberSwipeDismissableNavController
import com.fossisawesome.firmium.wear.FirmiumWearApplication
import com.fossisawesome.firmium.wear.RemoteScreen
import com.fossisawesome.firmium.wear.WearPlaybackClient
import kotlinx.coroutines.flow.first

@Composable
fun WearNavGraph(client: WearPlaybackClient) {
    val app = LocalContext.current.applicationContext as FirmiumWearApplication

    // Resolve the persisted mode once before composing the nav host, so the app reopens into
    // whichever mode (standalone browse vs. remote-control) the user was last using.
    val startDestination by produceState<String?>(null) {
        value = if (app.watchPreferences.lastMode.first() == "remote") "remote" else "home"
    }
    val resolvedStart = startDestination
    if (resolvedStart == null) {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
        return
    }

    val navController = rememberSwipeDismissableNavController()

    // Persist the mode on every route change: "remote" route -> remote mode (also pausing any
    // active standalone playback so there's only one audio source); anything else -> standalone.
    val backStackEntry by navController.currentBackStackEntryAsState()
    LaunchedEffect(backStackEntry?.destination?.route) {
        when (val route = backStackEntry?.destination?.route) {
            null -> {}
            "remote" -> {
                app.watchPreferences.setLastMode("remote")
                if (app.playbackController.state.value.playbackState == "playing") {
                    app.playbackController.pause()
                }
            }
            else -> app.watchPreferences.setLastMode("standalone")
        }
    }

    SwipeDismissableNavHost(navController = navController, startDestination = resolvedStart) {
        composable("home") { HomeScreen(app, navController) }
        composable("artists") { ArtistListScreen(app, navController) }
        composable(
            "artist/{artistId}",
            arguments = listOf(navArgument("artistId") { type = NavType.StringType }),
        ) { backStackEntry ->
            val artistId = backStackEntry.arguments?.getString("artistId") ?: return@composable
            ArtistDetailScreen(app, navController, artistId)
        }
        composable(
            "album/{albumId}",
            arguments = listOf(navArgument("albumId") { type = NavType.StringType }),
        ) { backStackEntry ->
            val albumId = backStackEntry.arguments?.getString("albumId") ?: return@composable
            AlbumDetailScreen(app, navController, albumId)
        }
        composable("playlists") { PlaylistListScreen(app, navController) }
        composable(
            "playlist/{playlistId}",
            arguments = listOf(navArgument("playlistId") { type = NavType.StringType }),
        ) { backStackEntry ->
            val playlistId = backStackEntry.arguments?.getString("playlistId") ?: return@composable
            PlaylistDetailScreen(app, navController, playlistId)
        }
        composable("search") { SearchScreen(app, navController) }
        composable("nowPlaying") { NowPlayingScreen(app) }
        composable("remote") { RemoteScreen(client) }
    }
}
