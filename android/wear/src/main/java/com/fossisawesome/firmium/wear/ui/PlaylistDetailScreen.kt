package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.navigation.NavController
import androidx.wear.compose.foundation.lazy.ScalingLazyColumn
import androidx.wear.compose.foundation.lazy.itemsIndexed
import androidx.wear.compose.material.Chip
import androidx.wear.compose.material.ChipDefaults
import androidx.wear.compose.material.CircularProgressIndicator
import androidx.wear.compose.material.ListHeader
import androidx.wear.compose.material.Text
import com.fossisawesome.firmium.data.model.ServerPlaylistTracks
import com.fossisawesome.firmium.wear.FirmiumWearApplication

@Composable
fun PlaylistDetailScreen(app: FirmiumWearApplication, navController: NavController, playlistId: String) {
    val playlist by produceState<UiState<ServerPlaylistTracks>>(UiState.Loading, playlistId) {
        value = try {
            UiState.Success(app.api.getPlaylistTracks(playlistId))
        } catch (e: Exception) {
            UiState.Error(e.message ?: "Failed to load")
        }
    }

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        when (val state = playlist) {
            is UiState.Loading -> item { CircularProgressIndicator() }
            is UiState.Error -> item { Text(state.message) }
            is UiState.Success -> {
                val p = state.data
                item { ListHeader { Text(p.name) } }
                itemsIndexed(p.tracks) { index, track ->
                    Chip(
                        onClick = {
                            app.playbackController.playAt(p.tracks, index)
                            navController.navigate("nowPlaying")
                        },
                        label = { Text(track.title, maxLines = 1) },
                        secondaryLabel = { Text(track.displayArtist ?: track.artist, maxLines = 1) },
                        colors = ChipDefaults.secondaryChipColors(),
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
        }
    }
}
