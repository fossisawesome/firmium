package com.fossisawesome.firmium.wear.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.navigation.NavType
import androidx.navigation.navArgument
import androidx.wear.compose.navigation.SwipeDismissableNavHost
import androidx.wear.compose.navigation.composable
import androidx.wear.compose.navigation.rememberSwipeDismissableNavController
import com.fossisawesome.firmium.wear.FirmiumWearApplication
import com.fossisawesome.firmium.wear.RemoteScreen
import com.fossisawesome.firmium.wear.WearPlaybackClient

@Composable
fun WearNavGraph(client: WearPlaybackClient) {
    val app = LocalContext.current.applicationContext as FirmiumWearApplication
    val navController = rememberSwipeDismissableNavController()

    SwipeDismissableNavHost(navController = navController, startDestination = "home") {
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
