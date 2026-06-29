package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

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
import androidx.compose.material.icons.filled.Shuffle
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
    onDownloadAlbum: ((com.fossisawesome.firmium.data.model.Album) -> suspend () -> Result<Unit>)? = null,
    onArtistClick: (String) -> Unit = {},
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
                Text(state.error, color = colors.error, fontFamily = LocalAppFontFamily.current, fontSize = 13.sp)
                FirmiumTextButton(onClick = { onLoad(albumId) }) {
                    Text("Retry", fontFamily = LocalAppFontFamily.current, color = colors.accent, fontSize = 14.sp)
                }
            }
            state.album != null -> {
                val album = state.album
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(bottom = 32.dp),
                ) {
                    item {
                        // Album header: cover top-left, title/artist/count + action icons, then Play + Shuffle.
                        Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
                            Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                                CoverImage(
                                    url = coverUrlFor(album.coverArt),
                                    contentDescription = album.name,
                                    modifier = Modifier.size(130.dp).clip(RoundedCornerShape(10.dp))
                                        .background(colors.surface2),
                                )
                                Column(modifier = Modifier.weight(1f).heightIn(min = 130.dp), verticalArrangement = Arrangement.Center) {
                                    Text(
                                        album.name, fontSize = 20.sp, fontWeight = FontWeight.Bold,
                                        fontFamily = LocalAppFontFamily.current, color = colors.text,
                                        maxLines = 3, overflow = TextOverflow.Ellipsis,
                                    )
                                    Spacer(Modifier.height(6.dp))
                                    Text(
                                        album.artist, fontSize = 13.sp, fontFamily = LocalAppFontFamily.current,
                                        color = colors.accent,
                                        textDecoration = androidx.compose.ui.text.style.TextDecoration.Underline,
                                        modifier = Modifier.clickable(enabled = album.artistId.isNotBlank()) {
                                            onArtistClick(album.artistId)
                                        },
                                    )
                                    Spacer(Modifier.height(4.dp))
                                    val countLabel = "${album.tracks.size} " + if (album.tracks.size == 1) "song" else "songs"
                                    val meta = listOfNotNull(
                                        countLabel,
                                        album.year?.toString(),
                                    ).joinToString(" · ")
                                    Text(meta, fontSize = 12.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)
                                    Spacer(Modifier.height(10.dp))
                                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                        if (onDownloadAlbum != null) {
                                            DownloadButton(
                                                onDownload = onDownloadAlbum.invoke(album),
                                                buttonSize = 36.dp, iconSize = 18.dp,
                                                initiallyDownloaded = state.allDownloaded,
                                            )
                                        }
                                        FirmiumIconButton(onClick = { pendingAllSongs = true }, modifier = Modifier.size(36.dp)) {
                                            FirmiumIcon(Icons.Default.Add, contentDescription = "Add all to playlist",
                                                tint = colors.muted, modifier = Modifier.size(18.dp))
                                        }
                                    }
                                }
                            }
                            if (album.tracks.isNotEmpty()) {
                                Spacer(Modifier.height(16.dp))
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Box(
                                        modifier = Modifier.weight(1f).clip(RoundedCornerShape(999.dp))
                                            .background(colors.accent).clickable { onPlayAll(album.tracks, 0) }
                                            .padding(vertical = 12.dp),
                                        contentAlignment = Alignment.Center,
                                    ) {
                                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                            FirmiumIcon(Icons.Default.PlayArrow, null, tint = colors.bg, modifier = Modifier.size(16.dp))
                                            Text("Play", fontSize = 12.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current, color = colors.bg, letterSpacing = 0.5.sp)
                                        }
                                    }
                                    Box(
                                        modifier = Modifier.weight(1f).clip(RoundedCornerShape(999.dp))
                                            .border(1.dp, colors.border, RoundedCornerShape(999.dp))
                                            .background(colors.surface2).clickable { onPlayAll(album.tracks.shuffled(), 0) }
                                            .padding(vertical = 12.dp),
                                        contentAlignment = Alignment.Center,
                                    ) {
                                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                            FirmiumIcon(Icons.Default.Shuffle, null, tint = colors.text, modifier = Modifier.size(16.dp))
                                            Text("Shuffle", fontSize = 12.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current, color = colors.text, letterSpacing = 0.5.sp)
                                        }
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
                                            fontFamily = LocalAppFontFamily.current, color = fg)
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

                    // Server-side track lists can contain duplicate song ids (bonus track
                    // re-releases, malformed responses) — key by index+id so LazyColumn's
                    // uniqueness requirement can't be violated.
                    itemsIndexed(displayTracks, key = { index, s -> "$index-${s.id}" }) { index, song ->
                        AlbumTrackRow(
                            track = song,
                            index = index + 1,
                            coverUrl = coverUrlFor(song.coverArt),
                            onClick = { onPlayAll(displayTracks, index) },
                            onAddClick = { pendingSong = song },
                            onDownloadClick = onDownloadTrack?.invoke(song),
                            isDownloaded = song.id in state.downloadedSongIds,
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
    isDownloaded: Boolean = false,
) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onClick() }
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text(
            "$index", fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
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
                track.title, fontFamily = LocalAppFontFamily.current, fontSize = 14.sp,
                color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            if (track.displayArtist != null && track.displayArtist != track.artist) {
                Text(
                    track.displayArtist, fontSize = 12.sp, fontFamily = LocalAppFontFamily.current,
                    color = colors.muted, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Text(formatDuration(track.duration), fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)
        if (onDownloadClick != null) {
            DownloadButton(onDownload = onDownloadClick, buttonSize = 32.dp, iconSize = 16.dp,
                initiallyDownloaded = isDownloaded)
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
    isDownloaded: Boolean = false,
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
                "$index", fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
                color = if (isCurrentlyPlaying) colors.accent else colors.muted,
                modifier = Modifier.width(28.dp),
            )
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(
                track.title, fontFamily = LocalAppFontFamily.current, fontSize = 14.sp,
                color = if (isCurrentlyPlaying) colors.accent else colors.text,
                maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            if (track.displayArtist != null && track.displayArtist != track.artist) {
                Text(
                    track.displayArtist, fontSize = 12.sp, fontFamily = LocalAppFontFamily.current,
                    color = colors.muted, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Spacer(Modifier.width(8.dp))
        Text(formatDuration(track.duration), fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)
        if (onDownloadClick != null) {
            DownloadButton(onDownload = onDownloadClick, buttonSize = 32.dp, iconSize = 16.dp,
                initiallyDownloaded = isDownloaded)
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
