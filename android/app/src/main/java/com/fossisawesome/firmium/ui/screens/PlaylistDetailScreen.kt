package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

@Composable
fun PlaylistDetailScreen(
    playlist: Playlist,
    onPlayAll: (List<Song>, Int) -> Unit,
    onRemoveTrack: (trackId: String) -> Unit,
    onDownloadTrack: ((Song) -> suspend () -> Result<Unit>)? = null,
    onBack: () -> Unit,
) {
    val colors = LocalFirmiumColors.current

    Column(modifier = Modifier.fillMaxSize()) {
        FirmiumDetailHeader(
            title = playlist.name,
            onBack = onBack,
            action = if (playlist.tracks.isNotEmpty()) {
                {
                    FirmiumIconButton(
                        onClick = { onPlayAll(playlist.tracks, 0) },
                        modifier = Modifier.size(44.dp),
                    ) {
                        FirmiumIcon(Icons.Default.PlayArrow, contentDescription = "Play all",
                            tint = colors.accent)
                    }
                }
            } else null,
        )

        if (playlist.tracks.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No tracks yet", fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = colors.muted)
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 32.dp),
            ) {
                itemsIndexed(playlist.tracks, key = { _, s -> s.id }) { index, song ->
                    TrackRow(
                        track = song,
                        index = index + 1,
                        isCurrentlyPlaying = false,
                        onClick = { onPlayAll(playlist.tracks, index) },
                        onDownloadClick = onDownloadTrack?.invoke(song),
                    )
                    FirmiumDivider()
                }
            }
        }
    }
}
