package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.runtime.*
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.graphics.Color
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.AlbumDetailState
import com.fossisawesome.firmium.viewmodel.PlaylistListItem

private data class BpmRange(val label: String, val min: Int, val max: Int)

@Composable
fun AlbumDetailScreen(
    albumId: String,
    state: AlbumDetailState,
    coverUrlFor: (String?) -> String?,
    playlistItems: List<PlaylistListItem>,
    onLoad: (String) -> Unit,
    onPlayAll: (List<Song>, Int) -> Unit,
    onAddToPlaylist: (item: PlaylistListItem, songs: List<Song>) -> Unit,
    onCreatePlaylistAndAdd: (name: String, songs: List<Song>) -> Unit,
    onDownloadTrack: ((Song) -> suspend () -> Result<Unit>)? = null,
    onBack: () -> Unit,
) {
    LaunchedEffect(albumId) { onLoad(albumId) }

    var pendingSong by remember { mutableStateOf<Song?>(null) }
    var pendingAllSongs by remember { mutableStateOf(false) }
    val colors = LocalFirmiumColors.current
    var selectedBpm by remember { mutableIntStateOf(0) }

    val bpmRanges = remember {
        listOf(
            BpmRange("All", 0, Int.MAX_VALUE),
            BpmRange("<80", 0, 79),
            BpmRange("80-120", 80, 120),
            BpmRange("120+", 121, Int.MAX_VALUE),
        )
    }

    Column(modifier = Modifier.fillMaxSize()) {
        FirmiumDetailHeader(title = state.album?.name ?: "", onBack = onBack)

        when {
            state.isLoading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp))
            }
            state.error != null -> Column(
                Modifier.fillMaxSize().padding(32.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                Text(state.error, color = colors.error, fontFamily = FontFamily.Monospace, fontSize = 13.sp)
                FirmiumTextButton(onClick = { onLoad(albumId) }) {
                    Text("Retry", fontFamily = FontFamily.Monospace, color = colors.accent, fontSize = 14.sp)
                }
            }
            state.album != null -> {
                val album = state.album
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(bottom = 32.dp),
                ) {
                    item {
                        // Album header: art + info + Play All
                        Column(
                            modifier = Modifier.fillMaxWidth().padding(16.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            CoverImage(
                                url = coverUrlFor(album.coverArt),
                                contentDescription = album.name,
                                modifier = Modifier.size(220.dp).clip(RoundedCornerShape(12.dp))
                                    .background(colors.surface2),
                            )
                            Spacer(Modifier.height(16.dp))
                            Text(
                                album.name, fontSize = 18.sp, fontWeight = FontWeight.Bold,
                                fontFamily = FontFamily.Monospace, color = colors.text,
                                textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                            )
                            Spacer(Modifier.height(4.dp))
                            Text(album.artist, fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                            if (album.year != null) {
                                val meta = listOfNotNull(
                                    album.year.toString(),
                                    album.releaseType.takeIf { it.isNotEmpty() && it != "Album" }
                                ).joinToString(" · ")
                                Text(meta, fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
                            }
                            if (album.tracks.isNotEmpty()) {
                                Spacer(Modifier.height(16.dp))
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Box(
                                        modifier = Modifier.weight(1f).clip(RoundedCornerShape(2.dp))
                                            .background(colors.accent).clickable { onPlayAll(album.tracks, 0) }
                                            .padding(vertical = 12.dp),
                                        contentAlignment = Alignment.Center,
                                    ) {
                                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                            FirmiumIcon(Icons.Default.PlayArrow, null, tint = colors.bg, modifier = Modifier.size(16.dp))
                                            Text("Play All", fontSize = 12.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, color = colors.bg, letterSpacing = 0.5.sp)
                                        }
                                    }
                                    Box(
                                        modifier = Modifier.clip(RoundedCornerShape(2.dp))
                                            .border(1.dp, colors.border, RoundedCornerShape(2.dp))
                                            .background(colors.surface2).clickable { pendingAllSongs = true }
                                            .padding(horizontal = 16.dp, vertical = 12.dp),
                                        contentAlignment = Alignment.Center,
                                    ) {
                                        FirmiumIcon(Icons.Default.Add, contentDescription = "Add all to playlist", tint = colors.text, modifier = Modifier.size(16.dp))
                                    }
                                }
                            }
                        }
                        FirmiumDivider()
                    }

                    val hasBpm = album.tracks.any { (it.bpm ?: 0) > 0 }
                    if (hasBpm) {
                        item(key = "bpm_filter") {
                            Row(
                                modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp),
                                horizontalArrangement = Arrangement.spacedBy(4.dp),
                            ) {
                                bpmRanges.forEachIndexed { i, range ->
                                    val active = selectedBpm == i
                                    val bg = if (active) colors.accent else colors.surface
                                    val fg = if (active) Color.Black else colors.muted
                                    val borderColor = if (active) colors.accent else colors.border
                                    Box(
                                        modifier = Modifier
                                            .clip(RoundedCornerShape(2.dp))
                                            .background(bg)
                                            .border(1.dp, borderColor, RoundedCornerShape(2.dp))
                                            .clickable { selectedBpm = i }
                                            .padding(horizontal = 10.dp, vertical = 3.dp),
                                    ) {
                                        Text("BPM ${range.label}", fontSize = 11.sp,
                                            fontFamily = FontFamily.Monospace, color = fg)
                                    }
                                }
                            }
                        }
                    }

                    val displayTracks = if (selectedBpm == 0) album.tracks
                    else {
                        val r = bpmRanges[selectedBpm]
                        album.tracks.filter { (it.bpm ?: 0) in r.min..r.max }
                    }

                    itemsIndexed(displayTracks, key = { _, s -> s.id }) { index, song ->
                        AlbumTrackRow(
                            track = song,
                            index = index + 1,
                            coverUrl = coverUrlFor(song.coverArt),
                            onClick = { onPlayAll(displayTracks, index) },
                            onAddClick = { pendingSong = song },
                            onDownloadClick = onDownloadTrack?.invoke(song),
                        )
                        FirmiumDivider()
                    }
                }
            }
        }
    }

    pendingSong?.let { song ->
        AddToPlaylistDialog(
            items = playlistItems,
            onAddTo = { item -> onAddToPlaylist(item, listOf(song)); pendingSong = null },
            onCreateAndAdd = { name -> onCreatePlaylistAndAdd(name, listOf(song)); pendingSong = null },
            onDismiss = { pendingSong = null },
        )
    }

    val album = state.album
    if (pendingAllSongs && album != null) {
        AddToPlaylistDialog(
            items = playlistItems,
            onAddTo = { item -> onAddToPlaylist(item, album.tracks); pendingAllSongs = false },
            onCreateAndAdd = { name -> onCreatePlaylistAndAdd(name, album.tracks); pendingAllSongs = false },
            onDismiss = { pendingAllSongs = false },
        )
    }
}

// Track row with thumbnail, title, optional artist, duration, and a visible + add button.
@Composable
private fun AlbumTrackRow(
    track: Song,
    index: Int,
    coverUrl: String?,
    onClick: () -> Unit,
    onAddClick: () -> Unit,
    onDownloadClick: (suspend () -> Result<Unit>)? = null,
) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onClick() }
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text(
            "$index", fontSize = 11.sp, fontFamily = FontFamily.Monospace,
            color = colors.muted, modifier = Modifier.width(24.dp),
            textAlign = androidx.compose.ui.text.style.TextAlign.End,
        )
        CoverImage(
            url = coverUrl,
            contentDescription = null,
            modifier = Modifier.size(36.dp).clip(RoundedCornerShape(4.dp)),
        )
        Column(modifier = Modifier.weight(1f)) {
            Text(
                track.title, fontFamily = FontFamily.Monospace, fontSize = 14.sp,
                color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            if (track.displayArtist != null && track.displayArtist != track.artist) {
                Text(
                    track.displayArtist, fontSize = 12.sp, fontFamily = FontFamily.Monospace,
                    color = colors.muted, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Text(formatDuration(track.duration), fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        if (onDownloadClick != null) {
            DownloadButton(onDownload = onDownloadClick, buttonSize = 32.dp, iconSize = 16.dp)
        }
        FirmiumIconButton(onClick = onAddClick, modifier = Modifier.size(32.dp)) {
            FirmiumIcon(Icons.Default.Add, contentDescription = "Add to playlist", tint = colors.muted, modifier = Modifier.size(16.dp))
        }
    }
}

// Shared TrackRow used by PlaylistDetailScreen
@Composable
fun TrackRow(
    track: Song,
    index: Int?,
    isCurrentlyPlaying: Boolean,
    onClick: () -> Unit,
    onDownloadClick: (suspend () -> Result<Unit>)? = null,
    onMoveUp: (() -> Unit)? = null,
    onMoveDown: (() -> Unit)? = null,
    canMoveUp: Boolean = false,
    canMoveDown: Boolean = false,
    onRemove: (() -> Unit)? = null,
) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onClick() }.padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (index != null) {
            Text(
                "$index", fontSize = 11.sp, fontFamily = FontFamily.Monospace,
                color = if (isCurrentlyPlaying) colors.accent else colors.muted,
                modifier = Modifier.width(28.dp),
            )
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(
                track.title, fontFamily = FontFamily.Monospace, fontSize = 14.sp,
                color = if (isCurrentlyPlaying) colors.accent else colors.text,
                maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            if (track.displayArtist != null && track.displayArtist != track.artist) {
                Text(
                    track.displayArtist, fontSize = 12.sp, fontFamily = FontFamily.Monospace,
                    color = colors.muted, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Spacer(Modifier.width(8.dp))
        Text(formatDuration(track.duration), fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        if (onDownloadClick != null) {
            DownloadButton(onDownload = onDownloadClick, buttonSize = 32.dp, iconSize = 16.dp)
        }
        if (onMoveUp != null && onMoveDown != null) {
            FirmiumIconButton(onClick = onMoveUp, enabled = canMoveUp, modifier = Modifier.size(32.dp)) {
                FirmiumIcon(Icons.Default.KeyboardArrowUp, contentDescription = "Move up",
                    tint = if (canMoveUp) colors.muted else colors.muted.copy(alpha = 0.3f), modifier = Modifier.size(18.dp))
            }
            FirmiumIconButton(onClick = onMoveDown, enabled = canMoveDown, modifier = Modifier.size(32.dp)) {
                FirmiumIcon(Icons.Default.KeyboardArrowDown, contentDescription = "Move down",
                    tint = if (canMoveDown) colors.muted else colors.muted.copy(alpha = 0.3f), modifier = Modifier.size(18.dp))
            }
        }
        if (onRemove != null) {
            FirmiumIconButton(onClick = onRemove, modifier = Modifier.size(32.dp)) {
                FirmiumIcon(Icons.Default.Close, contentDescription = "Remove from playlist",
                    tint = colors.error, modifier = Modifier.size(18.dp))
            }
        }
    }
}

private fun formatDuration(seconds: Int): String {
    val m = seconds / 60; val s = seconds % 60
    return "%d:%02d".format(m, s)
}
