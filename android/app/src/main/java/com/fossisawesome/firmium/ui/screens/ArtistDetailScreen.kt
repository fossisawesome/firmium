package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.ArtistDetailState

@Composable
fun ArtistDetailScreen(
    artistId: String,
    state: ArtistDetailState,
    coverUrlFor: (String?) -> String?,
    playlists: List<Playlist>,
    onLoad: (String) -> Unit,
    onAlbumClick: (String) -> Unit,
    onAddAlbum: (albumId: String) -> Unit,
    onDownloadAlbum: ((Album) -> suspend () -> Result<Unit>)? = null,
    onBack: () -> Unit,
    recommendations: List<Artist> = emptyList(),
    onArtistClick: (String) -> Unit = {},
    onStartRadio: (() -> Unit)? = null,
) {
    LaunchedEffect(artistId) { onLoad(artistId) }

    val colors = LocalFirmiumColors.current

    Column(modifier = Modifier.fillMaxSize()) {
        FirmiumDetailHeader(title = state.detail?.artist?.name ?: "", onBack = onBack)

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
                FirmiumTextButton(onClick = { onLoad(artistId) }) {
                    Text("Retry", fontFamily = FontFamily.Monospace, color = colors.accent, fontSize = 14.sp)
                }
            }
            state.detail != null -> {
                val detail = state.detail
                var showBio by remember { mutableStateOf(false) }

                // Artist image: server-provided URL first; fallback to most recent album cover.
                val artistImageUrl = detail.imageUrl?.takeIf {
                    it.isNotBlank() && !it.contains("2a96cbd8b46e442fc41c2b86b821562f")
                } ?: coverUrlFor(
                    detail.albums.maxByOrNull { it.year ?: 0 }?.coverArt
                        ?: detail.albums.firstOrNull()?.coverArt
                )

                // Sort albums into sections by type.
                val grouped = detail.albums
                    .sortedWith(compareBy({ it.effectiveType().releaseTypeSortOrder() }, { -(it.year ?: 0) }))
                    .groupBy { it.effectiveType() }

                // Box lets the BiographySheet overlay the list rather than being squashed to 0dp.
                Box(modifier = Modifier.fillMaxSize()) {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(bottom = 32.dp),
                    ) {
                        // Artist image header.
                        if (artistImageUrl != null) {
                            item {
                                CoverImage(
                                    url = artistImageUrl,
                                    contentDescription = detail.artist.name,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(220.dp),
                                )
                            }
                        }

                        // Biography button — opens a full bottom sheet with the untruncated bio.
                        if (detail.bio != null) {
                            item {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .clickable { showBio = true }
                                        .padding(horizontal = 16.dp, vertical = 14.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Text(
                                        "View Biography",
                                        fontSize = 13.sp,
                                        fontFamily = FontFamily.Monospace,
                                        color = colors.accent,
                                    )
                                    FirmiumIcon(
                                        Icons.Default.ChevronRight,
                                        contentDescription = null,
                                        tint = colors.accent,
                                        modifier = Modifier.size(18.dp),
                                    )
                                }
                                FirmiumDivider()
                            }
                        }

                        if (onStartRadio != null) {
                            item(key = "start_radio") {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .clickable { onStartRadio() }
                                        .padding(horizontal = 16.dp, vertical = 14.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Text("Start Radio", fontSize = 13.sp, fontFamily = FontFamily.Monospace, color = colors.accent)
                                    FirmiumIcon(Icons.Default.ChevronRight, contentDescription = null,
                                        tint = colors.accent, modifier = Modifier.size(18.dp))
                                }
                                FirmiumDivider()
                            }
                        }

                        item(key = "sort_label") {
                            Text(
                                "Sorted by type, then year",
                                fontSize = 11.sp, fontFamily = FontFamily.Monospace,
                                color = colors.muted,
                                modifier = Modifier.padding(start = 16.dp, top = 12.dp),
                            )
                        }

                        grouped.forEach { (type, albums) ->
                            val sectionLabel = when (type) {
                                "Single" -> "Singles"
                                "EP" -> "EPs"
                                "Album" -> "Albums"
                                else -> type
                            }
                            item(key = "header_$type") {
                                Text(
                                    "${sectionLabel.uppercase()} · ${albums.size}",
                                    fontSize = 11.sp, fontFamily = FontFamily.Monospace,
                                    color = colors.muted, letterSpacing = 1.sp,
                                    modifier = Modifier.padding(start = 16.dp, top = 12.dp, bottom = 8.dp),
                                )
                            }
                            items(albums, key = { it.id }) { album ->
                                AlbumRow(
                                    album = album,
                                    coverUrl = coverUrlFor(album.coverArt),
                                    onAlbumClick = onAlbumClick,
                                    onAddClick = { onAddAlbum(album.id) },
                                    onDownloadClick = onDownloadAlbum?.invoke(album),
                                    showArtist = false,
                                )
                                FirmiumDivider()
                            }
                        }

                        if (recommendations.isNotEmpty()) {
                            item(key = "reco_header") {
                                Text(
                                    "YOU MIGHT ALSO LIKE · ${recommendations.size}",
                                    fontSize = 11.sp, fontFamily = FontFamily.Monospace,
                                    color = colors.muted, letterSpacing = 1.sp,
                                    modifier = Modifier.padding(start = 16.dp, top = 16.dp, bottom = 8.dp),
                                )
                            }
                            items(recommendations, key = { "reco_${it.id}" }) { rec ->
                                Row(
                                    modifier = Modifier.fillMaxWidth()
                                        .clickable { onArtistClick(rec.id) }
                                        .padding(horizontal = 16.dp, vertical = 12.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Text(rec.name, fontSize = 14.sp, fontFamily = FontFamily.Monospace, color = colors.text)
                                    FirmiumIcon(Icons.Default.ChevronRight, contentDescription = null,
                                        tint = colors.muted, modifier = Modifier.size(18.dp))
                                }
                                FirmiumDivider()
                            }
                        }
                    }

                    if (showBio && detail.bio != null) {
                        BiographySheet(
                            bio = detail.bio,
                            artistName = detail.artist.name,
                            onDismiss = { showBio = false },
                        )
                    }
                }
            }
        }
    }
}

// Full biography bottom sheet — slides up from the bottom when "View Biography" is tapped.
@Composable
private fun BiographySheet(bio: String, artistName: String, onDismiss: () -> Unit) {
    val colors = LocalFirmiumColors.current
    FirmiumBottomSheet(onDismiss = onDismiss) {
        Text(
            text = artistName,
            fontSize = 14.sp,
            fontWeight = FontWeight.Bold,
            fontFamily = FontFamily.Monospace,
            color = colors.text,
            modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 12.dp),
        )
        FirmiumDivider()
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(max = 480.dp)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 16.dp),
        ) {
            Text(
                text = bio,
                fontSize = 13.sp,
                fontFamily = FontFamily.Monospace,
                color = colors.muted,
                lineHeight = 20.sp,
            )
        }
    }
}
