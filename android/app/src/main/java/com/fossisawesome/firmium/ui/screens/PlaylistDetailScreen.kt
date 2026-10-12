package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.filled.DownloadDone
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

@Composable
fun PlaylistDetailScreen(
    title: String,
    tracks: List<Song>,
    isServerOnly: Boolean = false,
    serverLoading: Boolean = false,
    onPlayAll: (List<Song>, Int) -> Unit,
    onRemoveTrack: (trackId: String, index: Int) -> Unit,
    onMoveTrack: (from: Int, to: Int) -> Unit,
    onDownloadTrack: ((Song) -> suspend () -> Result<Unit>)? = null,
    downloadedSongIds: Set<String> = emptySet(),
    onAddToQueue: ((Song) -> Unit)? = null,
    onBack: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val allDownloaded = tracks.isNotEmpty() && downloadedSongIds.size >= tracks.size
    var longPressSong by remember { mutableStateOf<Song?>(null) }

    Column(modifier = Modifier.fillMaxSize()) {
        FirmiumDetailHeader(
            title = title,
            onBack = onBack,
            action = if (tracks.isNotEmpty()) {
                {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (allDownloaded) {
                            FirmiumIcon(Icons.Default.DownloadDone, contentDescription = "Downloaded",
                                tint = colors.accent, modifier = Modifier.size(20.dp))
                            Spacer(Modifier.width(4.dp))
                        }
                        FirmiumIconButton(
                            onClick = { onPlayAll(tracks, 0) },
                            modifier = Modifier.size(44.dp),
                        ) {
                            FirmiumIcon(Icons.Default.PlayArrow, contentDescription = "Play all",
                                tint = colors.accent)
                        }
                    }
                }
            } else null,
        )

        if (isServerOnly && serverLoading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("Loading…", fontFamily = LocalAppFontFamily.current, fontSize = 13.sp, color = colors.muted)
            }
        } else if (tracks.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No tracks yet", fontFamily = LocalAppFontFamily.current, fontSize = 13.sp, color = colors.muted)
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 32.dp),
            ) {
                // Server playlists can contain the same song id twice (added via another
                // client with no dedup) — key by index+id so LazyColumn's uniqueness
                // requirement can't be violated.
                itemsIndexed(tracks, key = { index, s -> "$index-${s.id}" }) { index, song ->
                    TrackRow(
                        track = song,
                        index = index + 1,
                        isCurrentlyPlaying = false,
                        onClick = { onPlayAll(tracks, index) },
                        onDownloadClick = onDownloadTrack?.invoke(song),
                        isDownloaded = song.id in downloadedSongIds,
                        onMoveUp = { onMoveTrack(index, index - 1) },
                        onMoveDown = { onMoveTrack(index, index + 1) },
                        canMoveUp = index > 0,
                        canMoveDown = index < tracks.size - 1,
                        onRemove = { onRemoveTrack(song.id, index) },
                        onAddToQueue = onAddToQueue?.let { { it(song) } },
                        onLongPress = if (onAddToQueue != null) { { longPressSong = song } } else null,
                    )
                    FirmiumDivider()
                }
            }
        }
    }

    longPressSong?.let { song ->
        FirmiumBottomSheet(onDismiss = { longPressSong = null }) {
            Text(
                song.title,
                fontSize = 12.sp,
                fontFamily = LocalAppFontFamily.current,
                color = colors.muted,
                maxLines = 1,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 10.dp),
            )
            FirmiumDivider()
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable {
                        onAddToQueue?.invoke(song)
                        longPressSong = null
                    }
                    .padding(horizontal = 20.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                FirmiumIcon(
                    Icons.AutoMirrored.Filled.QueueMusic,
                    contentDescription = null,
                    tint = colors.accent,
                    modifier = Modifier.size(20.dp),
                )
                Text(
                    "Add to queue",
                    fontSize = 14.sp,
                    fontFamily = LocalAppFontFamily.current,
                    color = colors.text,
                )
            }
            Spacer(Modifier.height(8.dp))
        }
    }
}
