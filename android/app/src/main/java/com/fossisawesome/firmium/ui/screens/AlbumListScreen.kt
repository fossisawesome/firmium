package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.AlbumListState

// Classifies an album into Single / EP / Album based on songCount.
// Special server-reported types (Compilation, Live, Remix) are preserved.
fun Album.effectiveType(): String {
    if (releaseType in setOf("Compilation", "Live", "Remix")) return releaseType
    return when {
        songCount <= 2 -> "Single"
        songCount <= 6 -> "EP"
        else -> "Album"
    }
}

private val typeOrder = mapOf("Single" to 0, "EP" to 1, "Album" to 2, "Compilation" to 3, "Live" to 4, "Remix" to 5)
fun String.releaseTypeSortOrder(): Int = typeOrder[this] ?: 6

@Composable
fun AlbumListScreen(
    state: AlbumListState,
    coverUrlFor: (String?) -> String?,
    playlists: List<Playlist>,
    onAlbumClick: (String) -> Unit,
    onLoad: () -> Unit,
    onAddAlbumToPlaylist: (playlistId: String, albumId: String) -> Unit,
    onCreatePlaylistAndAddAlbum: (name: String, albumId: String) -> Unit,
    onDownloadAlbum: ((Album) -> suspend () -> Result<Unit>)? = null,
) {
    LaunchedEffect(Unit) { onLoad() }

    val colors = LocalFirmiumColors.current
    var pendingAlbumId by remember { mutableStateOf<String?>(null) }

    when {
        state.isLoading && state.albums.isEmpty() -> Box(
            Modifier.fillMaxSize(), contentAlignment = Alignment.Center,
        ) { FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp)) }

        state.error != null -> Column(
            Modifier.fillMaxSize().padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Text(state.error, color = colors.error, fontFamily = FontFamily.Monospace, fontSize = 13.sp)
            FirmiumTextButton(onClick = onLoad) {
                Text("Retry", fontFamily = FontFamily.Monospace, color = colors.accent, fontSize = 14.sp)
            }
        }

        else -> {
            // Sort into Singles → EPs → Albums → special types, then by year within each group.
            val grouped = state.albums
                .sortedWith(compareBy({ it.effectiveType().releaseTypeSortOrder() }, { -(it.year ?: 0) }))
                .groupBy { it.effectiveType() }

            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp),
            ) {
                grouped.forEach { (type, albums) ->
                    // Section header with item count.
                    item(key = "header_$type") {
                        val label = when (type) {
                            "Single" -> "Singles"
                            "EP" -> "EPs"
                            "Album" -> "Albums"
                            else -> type
                        }
                        Text(
                            "${label.uppercase()} · ${albums.size}",
                            fontSize = 11.sp, fontFamily = FontFamily.Monospace,
                            color = colors.muted, letterSpacing = 1.sp,
                            modifier = Modifier.padding(start = 16.dp, top = 16.dp, bottom = 8.dp),
                        )
                    }
                    items(albums, key = { it.id }) { album ->
                        AlbumRow(
                            album = album,
                            coverUrl = coverUrlFor(album.coverArt),
                            onAlbumClick = onAlbumClick,
                            onAddClick = { pendingAlbumId = album.id },
                            onDownloadClick = onDownloadAlbum?.invoke(album),
                        )
                        FirmiumDivider()
                    }
                }
            }
        }
    }

    val albumId = pendingAlbumId
    if (albumId != null) {
        AddToPlaylistDialog(
            playlists = playlists,
            onAddTo = { pid -> onAddAlbumToPlaylist(pid, albumId); pendingAlbumId = null },
            onCreateAndAdd = { name -> onCreatePlaylistAndAddAlbum(name, albumId); pendingAlbumId = null },
            onDismiss = { pendingAlbumId = null },
        )
    }
}

