package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.People
import androidx.compose.material.icons.filled.PlaylistPlay
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Watch
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import androidx.wear.compose.foundation.lazy.ScalingLazyColumn
import androidx.wear.compose.foundation.lazy.items
import androidx.wear.compose.material.Chip
import androidx.wear.compose.material.ChipDefaults
import androidx.wear.compose.material.CircularProgressIndicator
import androidx.wear.compose.material.Icon
import androidx.wear.compose.material.ListHeader
import androidx.wear.compose.material.Text
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.wear.FirmiumWearApplication

@Composable
fun HomeScreen(app: FirmiumWearApplication, navController: NavController) {
    val recentAlbums by produceState<UiState<List<Album>>>(UiState.Loading) {
        value = try {
            UiState.Success(app.api.getRecentAlbums())
        } catch (e: Exception) {
            UiState.Error(e.message ?: "Failed to load")
        }
    }

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        item { ListHeader { Text("Firmium") } }
        item {
            Chip(
                onClick = { navController.navigate("artists") },
                label = { Text("Artists") },
                icon = { Icon(Icons.Filled.People, contentDescription = null) },
                colors = ChipDefaults.secondaryChipColors(),
                modifier = Modifier.fillMaxWidth(),
            )
        }
        item {
            Chip(
                onClick = { navController.navigate("playlists") },
                label = { Text("Playlists") },
                icon = { Icon(Icons.Filled.PlaylistPlay, contentDescription = null) },
                colors = ChipDefaults.secondaryChipColors(),
                modifier = Modifier.fillMaxWidth(),
            )
        }
        item {
            Chip(
                onClick = { navController.navigate("search") },
                label = { Text("Search") },
                icon = { Icon(Icons.Filled.Search, contentDescription = null) },
                colors = ChipDefaults.secondaryChipColors(),
                modifier = Modifier.fillMaxWidth(),
            )
        }
        item {
            Chip(
                onClick = { navController.navigate("downloads") },
                label = { Text("Downloads") },
                icon = { Icon(Icons.Filled.Download, contentDescription = null) },
                colors = ChipDefaults.secondaryChipColors(),
                modifier = Modifier.fillMaxWidth(),
            )
        }
        item {
            Chip(
                onClick = { navController.navigate("remote") },
                label = { Text("Remote (phone playback)") },
                icon = { Icon(Icons.Filled.Watch, contentDescription = null) },
                colors = ChipDefaults.secondaryChipColors(),
                modifier = Modifier.fillMaxWidth(),
            )
        }
        item { ListHeader { Text("Recent Albums") } }
        when (val state = recentAlbums) {
            is UiState.Loading -> item { CircularProgressIndicator() }
            is UiState.Error -> item { Text(state.message) }
            is UiState.Success -> items(state.data) { album ->
                Chip(
                    onClick = { navController.navigate("album/${album.id}") },
                    label = { Text(album.name, maxLines = 1) },
                    secondaryLabel = { Text(album.artist, maxLines = 1) },
                    icon = {
                        WatchCoverImage(
                            url = app.authManager.safeCoverArtUrl(album.coverArt, 64),
                            contentDescription = album.name,
                            size = 24.dp,
                        )
                    },
                    colors = ChipDefaults.secondaryChipColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}
