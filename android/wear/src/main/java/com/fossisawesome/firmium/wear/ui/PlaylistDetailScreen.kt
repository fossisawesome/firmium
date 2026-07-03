package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Download
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import androidx.wear.compose.foundation.lazy.ScalingLazyColumn
import androidx.wear.compose.foundation.lazy.itemsIndexed
import androidx.wear.compose.material.ButtonDefaults
import androidx.wear.compose.material.Chip
import androidx.wear.compose.material.ChipDefaults
import androidx.wear.compose.material.CircularProgressIndicator
import androidx.wear.compose.material.CompactButton
import androidx.wear.compose.material.Icon
import androidx.wear.compose.material.ListHeader
import androidx.wear.compose.material.Text
import com.fossisawesome.firmium.data.model.ServerPlaylistTracks
import com.fossisawesome.firmium.wear.FirmiumWearApplication
import kotlinx.coroutines.launch

@Composable
fun PlaylistDetailScreen(app: FirmiumWearApplication, navController: NavController, playlistId: String) {
    val playlist by produceState<UiState<ServerPlaylistTracks>>(UiState.Loading, playlistId) {
        value = try {
            UiState.Success(app.api.getPlaylistTracks(playlistId))
        } catch (e: Exception) {
            UiState.Error(e.message ?: "Failed to load")
        }
    }
    val downloadedSongs by app.downloadManager.downloadedSongs.collectAsState(initial = emptyList())
    val downloadedIds = downloadedSongs.map { it.id }.toSet()
    var isDownloadingPlaylist by remember { mutableStateOf(false) }
    var downloadingTrackId by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        when (val state = playlist) {
            is UiState.Loading -> item { CircularProgressIndicator() }
            is UiState.Error -> item { Text(state.message) }
            is UiState.Success -> {
                val p = state.data
                item { ListHeader { Text(p.name) } }
                item {
                    Chip(
                        onClick = {
                            if (!isDownloadingPlaylist) {
                                isDownloadingPlaylist = true
                                scope.launch {
                                    app.downloadManager.downloadTracks(p.tracks)
                                    isDownloadingPlaylist = false
                                }
                            }
                        },
                        label = { Text(if (isDownloadingPlaylist) "Downloading…" else "Download playlist") },
                        icon = {
                            if (isDownloadingPlaylist) CircularProgressIndicator(modifier = Modifier.size(20.dp))
                            else Icon(Icons.Filled.Download, contentDescription = null)
                        },
                        colors = ChipDefaults.secondaryChipColors(),
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
                itemsIndexed(p.tracks) { index, track ->
                    Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Chip(
                            onClick = {
                                app.playbackController.playAt(p.tracks, index)
                                navController.navigate("nowPlaying")
                            },
                            label = { Text(track.title, maxLines = 1) },
                            secondaryLabel = { Text(track.displayArtist ?: track.artist, maxLines = 1) },
                            colors = ChipDefaults.secondaryChipColors(),
                            modifier = Modifier.weight(1f),
                        )
                        CompactButton(
                            onClick = {
                                if (downloadingTrackId == null && track.id !in downloadedIds) {
                                    downloadingTrackId = track.id
                                    scope.launch {
                                        app.downloadManager.downloadTrack(track)
                                        downloadingTrackId = null
                                    }
                                }
                            },
                            colors = ButtonDefaults.secondaryButtonColors(),
                        ) {
                            when {
                                track.id in downloadedIds -> Icon(Icons.Filled.CheckCircle, contentDescription = "Downloaded")
                                downloadingTrackId == track.id -> CircularProgressIndicator(modifier = Modifier.size(16.dp))
                                else -> Icon(Icons.Filled.Download, contentDescription = "Download")
                            }
                        }
                    }
                }
            }
        }
    }
}
