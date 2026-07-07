package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.wear.compose.foundation.lazy.ScalingLazyColumn
import androidx.wear.compose.foundation.lazy.items
import androidx.wear.compose.material.ButtonDefaults
import androidx.wear.compose.material.Chip
import androidx.wear.compose.material.ChipDefaults
import androidx.wear.compose.material.CompactButton
import androidx.wear.compose.material.Icon
import androidx.wear.compose.material.ListHeader
import androidx.wear.compose.material.Text
import com.fossisawesome.firmium.wear.FirmiumWearApplication
import kotlinx.coroutines.launch

@Composable
fun DownloadsScreen(app: FirmiumWearApplication) {
    val downloadedSongs by app.downloadManager.downloadedSongs.collectAsState(initial = emptyList())
    var totalBytes by remember { mutableLongStateOf(0L) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(downloadedSongs) {
        totalBytes = app.downloadManager.totalStorageBytes()
    }

    ScalingLazyColumn(modifier = Modifier.fillMaxSize()) {
        item {
            ListHeader { Text("Downloads (${"%.1f".format(totalBytes / 1024.0 / 1024.0)} MB)") }
        }
        if (downloadedSongs.isEmpty()) {
            item { Text("No downloads yet") }
        }
        items(downloadedSongs) { song ->
            Row(modifier = Modifier.fillMaxWidth()) {
                Chip(
                    onClick = { app.playbackController.playAt(listOf(song), 0) },
                    label = { Text(song.title, maxLines = 1) },
                    secondaryLabel = { Text(song.displayArtist ?: song.artist, maxLines = 1) },
                    colors = ChipDefaults.secondaryChipColors(),
                    modifier = Modifier.weight(1f),
                )
                CompactButton(
                    onClick = { scope.launch { app.downloadManager.deleteDownload(song.id) } },
                    colors = ButtonDefaults.secondaryButtonColors(),
                ) {
                    Icon(Icons.Filled.Delete, contentDescription = "Delete download")
                }
            }
        }
    }
}
