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
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.wear.FirmiumWearApplication

@Composable
fun ArtistDetailScreen(app: FirmiumWearApplication, navController: NavController, artistId: String) {
    val detail by produceState<UiState<ApiClient.ArtistDetailResult>>(UiState.Loading, artistId) {
        value = try {
            val d = app.api.getArtistDetail(artistId)
            UiState.Success(ApiClient.ArtistDetailResult(d.artist.name, d.albums))
        } catch (e: Exception) {
            UiState.Error(e.message ?: "Failed to load")
        }
    }

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        when (val state = detail) {
            is UiState.Loading -> item { CircularProgressIndicator() }
            is UiState.Error -> item { Text(state.message) }
            is UiState.Success -> {
                item { ListHeader { Text(state.data.artistName) } }
                items(state.data.albums) { album ->
                    Chip(
                        onClick = { navController.navigate("album/${album.id}") },
                        label = { Text(album.name, maxLines = 1) },
                        secondaryLabel = { Text(album.year?.toString() ?: album.releaseType) },
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
}
