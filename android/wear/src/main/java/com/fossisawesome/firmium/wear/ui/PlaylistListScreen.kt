package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.wear.compose.material.ListHeader
import androidx.wear.compose.material.Text
import com.fossisawesome.firmium.data.model.ServerPlaylist
import com.fossisawesome.firmium.wear.FirmiumWearApplication

@Composable
fun PlaylistListScreen(app: FirmiumWearApplication, navController: NavController) {
    val playlists by produceState<UiState<List<ServerPlaylist>>>(UiState.Loading) {
        value = try {
            UiState.Success(app.api.getPlaylists())
        } catch (e: Exception) {
            UiState.Error(e.message ?: "Failed to load")
        }
    }

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        item { ListHeader { Text("Playlists") } }
        when (val state = playlists) {
            is UiState.Loading -> item { CircularProgressIndicator() }
            is UiState.Error -> item { Text(state.message) }
            is UiState.Success -> items(state.data) { playlist ->
                Chip(
                    onClick = { navController.navigate("playlist/${playlist.id}") },
                    label = { Text(playlist.name, maxLines = 1) },
                    secondaryLabel = { Text("${playlist.songCount} tracks") },
                    icon = {
                        WatchCoverImage(
                            url = app.authManager.safeCoverArtUrl(playlist.coverArt, 64),
                            contentDescription = playlist.name,
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
