package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.ui.theme.LocalUiTheme
import com.fossisawesome.firmium.viewmodel.AlbumListState
import com.fossisawesome.firmium.viewmodel.PlaylistListItem

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
    playlistItems: List<PlaylistListItem>,
    onAlbumClick: (String) -> Unit,
    onLoad: () -> Unit,
    onAddAlbumToPlaylist: (item: PlaylistListItem, albumId: String) -> Unit,
    onCreatePlaylistAndAddAlbum: (name: String, albumId: String) -> Unit,
    onDownloadAlbum: ((Album) -> suspend () -> Result<Unit>)? = null,
) {
    LaunchedEffect(Unit) { onLoad() }

    val colors = LocalFirmiumColors.current
    val spotify = LocalUiTheme.current == "spotify"
    var pendingAlbumId by remember { mutableStateOf<String?>(null) }
    var selectedGenres by remember { mutableStateOf(emptySet<String>()) }
    var selectedDecades by remember { mutableStateOf(emptySet<String>()) }

    val allGenres = remember(state.albums) {
        val counts = mutableMapOf<String, Int>()
        state.albums.forEach { a -> a.genres.forEach { g -> counts[g] = (counts[g] ?: 0) + 1 } }
        counts.entries.sortedByDescending { it.value }.map { it.key }
    }

    val allDecades = remember(state.albums) {
        state.albums.mapNotNull { a ->
            val y = a.year ?: return@mapNotNull null
            if (y < 1900) return@mapNotNull null
            "${(y / 10) * 10}s"
        }.distinct().sorted()
    }

    val filteredAlbums = remember(state.albums, selectedGenres, selectedDecades) {
        var list = state.albums
        if (selectedGenres.isNotEmpty()) {
            list = list.filter { a -> a.genres.any { it in selectedGenres } }
        }
        if (selectedDecades.isNotEmpty()) {
            list = list.filter { a ->
                val y = a.year ?: return@filter false
                "${(y / 10) * 10}s" in selectedDecades
            }
        }
        list
    }

    when {
        state.isLoading && state.albums.isEmpty() -> Box(
            Modifier.fillMaxSize(), contentAlignment = Alignment.Center,
        ) { FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp)) }

        state.error != null -> Column(
            Modifier.fillMaxSize().padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Text(state.error, color = colors.error, fontFamily = LocalAppFontFamily.current, fontSize = 13.sp)
            FirmiumTextButton(onClick = onLoad) {
                Text("Retry", fontFamily = LocalAppFontFamily.current, color = colors.accent, fontSize = 14.sp)
            }
        }

        else -> {
            // Sort + group only when the filtered album set changes, not on every
            // recomposition (e.g. when pendingAlbumId toggles).
            val grouped = remember(filteredAlbums) {
                filteredAlbums
                    .sortedWith(compareBy({ it.effectiveType().releaseTypeSortOrder() }, { -(it.year ?: 0) }))
                    .groupBy { it.effectiveType() }
            }

            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp),
            ) {
                if (allDecades.isNotEmpty() || allGenres.isNotEmpty()) {
                    item(key = "filters") {
                        FilterChipsRow(
                            decades = allDecades,
                            genres = allGenres.take(20),
                            selectedDecades = selectedDecades,
                            selectedGenres = selectedGenres,
                            onToggleDecade = { d ->
                                selectedDecades = if (d in selectedDecades) selectedDecades - d else selectedDecades + d
                            },
                            onToggleGenre = { g ->
                                selectedGenres = if (g in selectedGenres) selectedGenres - g else selectedGenres + g
                            },
                            onClear = { selectedGenres = emptySet(); selectedDecades = emptySet() },
                        )
                    }
                }
                item(key = "sort_label") {
                    Text(
                        "Sorted by type, then year",
                        fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
                        color = colors.muted,
                        modifier = Modifier.padding(start = 16.dp, top = 12.dp),
                    )
                }
                grouped.forEach { (type, albums) ->
                    item(key = "header_$type") {
                        val label = when (type) {
                            "Single" -> "Singles"
                            "EP" -> "EPs"
                            "Album" -> "Albums"
                            else -> type
                        }
                        Text(
                            "${label.uppercase()} · ${albums.size}",
                            fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
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
                            coverSize = if (spotify) 56.dp else 44.dp,
                            coverRadius = if (spotify) 8.dp else 6.dp,
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
            items = playlistItems,
            onAddTo = { item -> onAddAlbumToPlaylist(item, albumId); pendingAlbumId = null },
            onCreateAndAdd = { name -> onCreatePlaylistAndAddAlbum(name, albumId); pendingAlbumId = null },
            onDismiss = { pendingAlbumId = null },
        )
    }
}

@Composable
private fun FilterChipsRow(
    decades: List<String>,
    genres: List<String>,
    selectedDecades: Set<String>,
    selectedGenres: Set<String>,
    onToggleDecade: (String) -> Unit,
    onToggleGenre: (String) -> Unit,
    onClear: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val hasActive = selectedDecades.isNotEmpty() || selectedGenres.isNotEmpty()

    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp),
    ) {
        @Composable
        fun Chip(label: String, active: Boolean, onClick: () -> Unit) {
            val bg = if (active) colors.accent else colors.surface
            val fg = if (active) Color.Black else colors.muted
            val borderColor = if (active) colors.accent else colors.border
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(2.dp))
                    .background(bg)
                    .border(1.dp, borderColor, RoundedCornerShape(2.dp))
                    .clickable { onClick() }
                    .padding(horizontal = 10.dp, vertical = 3.dp),
            ) {
                Text(label, fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = fg)
            }
        }

        FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            decades.forEach { d -> Chip(d, d in selectedDecades) { onToggleDecade(d) } }
            genres.forEach { g -> Chip(g, g in selectedGenres) { onToggleGenre(g) } }
            if (hasActive) {
                Box(
                    modifier = Modifier
                        .clickable { onClear() }
                        .padding(horizontal = 10.dp, vertical = 3.dp),
                ) {
                    Text("Clear", fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
                        color = colors.muted, fontWeight = FontWeight.Normal)
                }
            }
        }
    }
}


