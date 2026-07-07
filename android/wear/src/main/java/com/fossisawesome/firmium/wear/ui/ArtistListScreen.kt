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
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.wear.FirmiumWearApplication

@Composable
fun ArtistListScreen(app: FirmiumWearApplication, navController: NavController) {
    val artists by produceState<UiState<List<Artist>>>(UiState.Loading) {
        value = try {
            UiState.Success(app.api.getArtists())
        } catch (e: Exception) {
            UiState.Error(e.message ?: "Failed to load")
        }
    }

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        item { ListHeader { Text("Artists") } }
        when (val state = artists) {
            is UiState.Loading -> item { CircularProgressIndicator() }
            is UiState.Error -> item { Text(state.message) }
            is UiState.Success -> items(state.data) { artist ->
                Chip(
                    onClick = { navController.navigate("artist/${artist.id}") },
                    label = { Text(artist.name, maxLines = 1) },
                    secondaryLabel = { Text("${artist.albumCount} albums") },
                    icon = {
                        WatchCoverImage(
                            url = app.authManager.safeCoverArtUrl(artist.coverArt, 64),
                            contentDescription = artist.name,
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
