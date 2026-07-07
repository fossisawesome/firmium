package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import android.content.Intent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Radio
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.ArtistDetailState

@Composable
fun ArtistDetailScreen(
    artistId: String,
    state: ArtistDetailState,
    coverUrlFor: (String?) -> String?,
    onLoad: (String) -> Unit,
    onAlbumClick: (String) -> Unit,
    onPlayAlbum: (com.fossisawesome.firmium.data.model.Album) -> Unit,
    onPlaySongs: (List<Song>, Int) -> Unit,
    onBack: () -> Unit,
    recommendations: List<Artist> = emptyList(),
    onArtistClick: (String) -> Unit = {},
    onStartRadio: (() -> Unit)? = null,
) {
    LaunchedEffect(artistId) { onLoad(artistId) }

    val colors = LocalFirmiumColors.current
    val context = LocalContext.current

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
            FirmiumTextButton(onClick = { onLoad(artistId) }) {
                Text("Retry", fontFamily = LocalAppFontFamily.current, color = colors.accent, fontSize = 14.sp)
            }
        }
        state.detail != null -> {
            val detail = state.detail
            var showBio by remember { mutableStateOf(false) }

            val artistImageUrl = detail.imageUrl?.takeIf {
                it.isNotBlank() && !it.contains("2a96cbd8b46e442fc41c2b86b821562f")
            } ?: coverUrlFor(
                detail.albums.maxByOrNull { it.year ?: 0 }?.coverArt
                    ?: detail.albums.firstOrNull()?.coverArt
            )

            // Split into "Albums" (full-lengths + special types) and "Singles & EPs".
            val sorted = detail.albums.sortedByDescending { it.year ?: 0 }
            val albumsGroup = sorted.filter { it.effectiveType() !in setOf("Single", "EP") }
            val singlesEps = sorted.filter { it.effectiveType() in setOf("Single", "EP") }

            Box(modifier = Modifier.fillMaxSize()) {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(bottom = 32.dp),
                ) {
                    // Header: artist image with overlaid controls + large name.
                    item {
                        Box(modifier = Modifier.fillMaxWidth().height(320.dp)) {
                            CoverImage(
                                url = artistImageUrl,
                                contentDescription = detail.artist.name,
                                modifier = Modifier.fillMaxSize(),
                            )
                            // Bottom gradient so the name stays legible over bright art.
                            Box(
                                modifier = Modifier
                                    .fillMaxSize()
                                    .background(Brush.verticalGradient(
                                        0.5f to Color.Transparent, 1f to colors.bg)),
                            )
                            // Top controls.
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .windowInsetsPadding(WindowInsets.statusBars)
                                    .padding(8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                OverlayIconButton(Icons.AutoMirrored.Filled.ArrowBack, "Back", onClick = onBack)
                                Spacer(Modifier.weight(1f))
                                OverlayIconButton(Icons.Default.PlayArrow, "Play all", enabled = state.topSongs.isNotEmpty()) {
                                    onPlaySongs(state.topSongs, 0)
                                }
                                OverlayIconButton(Icons.Default.Share, "Share") {
                                    val intent = Intent(Intent.ACTION_SEND).apply {
                                        type = "text/plain"
                                        putExtra(Intent.EXTRA_TEXT, detail.artist.name)
                                    }
                                    context.startActivity(Intent.createChooser(intent, "Share artist"))
                                }
                            }
                            Text(
                                detail.artist.name,
                                fontSize = 32.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current,
                                color = colors.text, maxLines = 2, overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.align(Alignment.BottomStart).padding(start = 20.dp, end = 20.dp, bottom = 16.dp),
                            )
                        }
                    }

                    // Shuffle + Radio buttons.
                    item {
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 16.dp),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Box(
                                modifier = Modifier.weight(1f).clip(RoundedCornerShape(999.dp))
                                    .background(colors.accent)
                                    .clickable(enabled = state.topSongs.isNotEmpty()) {
                                        onPlaySongs(state.topSongs.shuffled(), 0)
                                    }
                                    .padding(vertical = 12.dp),
                                contentAlignment = Alignment.Center,
                            ) {
                                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    FirmiumIcon(Icons.Default.Shuffle, null, tint = Color.Black, modifier = Modifier.size(18.dp))
                                    Text("Shuffle", fontSize = 14.sp, fontWeight = FontWeight.Bold,
                                        fontFamily = LocalAppFontFamily.current, color = Color.Black)
                                }
                            }
                            if (onStartRadio != null) {
                                Box(
                                    modifier = Modifier.weight(1f).clip(RoundedCornerShape(999.dp))
                                        .background(colors.surface2)
                                        .clickable { onStartRadio() }
                                        .padding(vertical = 12.dp),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                        FirmiumIcon(Icons.Default.Radio, null, tint = colors.text, modifier = Modifier.size(18.dp))
                                        Text("Radio", fontSize = 14.sp, fontWeight = FontWeight.Bold,
                                            fontFamily = LocalAppFontFamily.current, color = colors.text)
                                    }
                                }
                            }
                        }
                    }

                    // Albums carousel.
                    if (albumsGroup.isNotEmpty()) {
                        item { SectionHeader("Albums") }
                        item {
                            LazyRow(
                                contentPadding = PaddingValues(horizontal = 20.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                items(albumsGroup, key = { it.id }) { album ->
                                    AlbumCard(
                                        album = album,
                                        coverUrl = coverUrlFor(album.coverArt),
                                        onClick = onAlbumClick,
                                        onPlay = onPlayAlbum,
                                        modifier = Modifier.width(150.dp),
                                    )
                                }
                            }
                        }
                    }

                    // Singles & EPs carousel.
                    if (singlesEps.isNotEmpty()) {
                        item { SectionHeader("Singles & EPs") }
                        item {
                            LazyRow(
                                contentPadding = PaddingValues(horizontal = 20.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                items(singlesEps, key = { it.id }) { album ->
                                    AlbumCard(
                                        album = album,
                                        coverUrl = coverUrlFor(album.coverArt),
                                        onClick = onAlbumClick,
                                        onPlay = onPlayAlbum,
                                        modifier = Modifier.width(150.dp),
                                    )
                                }
                            }
                        }
                    }

                    // Biography button.
                    if (detail.bio != null) {
                        item {
                            Row(
                                modifier = Modifier.fillMaxWidth().clickable { showBio = true }
                                    .padding(horizontal = 20.dp, vertical = 16.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Text("View Biography", fontSize = 13.sp, fontFamily = LocalAppFontFamily.current, color = colors.accent)
                                FirmiumIcon(Icons.Default.ChevronRight, null, tint = colors.accent, modifier = Modifier.size(18.dp))
                            }
                        }
                    }

                    // Recommendations.
                    if (recommendations.isNotEmpty()) {
                        item {
                            Text(
                                "YOU MIGHT ALSO LIKE · ${recommendations.size}",
                                fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
                                color = colors.muted, letterSpacing = 1.sp,
                                modifier = Modifier.padding(start = 20.dp, top = 16.dp, bottom = 8.dp),
                            )
                        }
                        items(recommendations, key = { "reco_${it.id}" }) { rec ->
                            Row(
                                modifier = Modifier.fillMaxWidth()
                                    .clickable { onArtistClick(rec.id) }
                                    .padding(horizontal = 20.dp, vertical = 12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Text(rec.name, fontSize = 14.sp, fontFamily = LocalAppFontFamily.current, color = colors.text)
                                FirmiumIcon(Icons.Default.ChevronRight, null, tint = colors.muted, modifier = Modifier.size(18.dp))
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

@Composable
private fun SectionHeader(title: String, onArrow: (() -> Unit)? = null) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().padding(start = 20.dp, end = 12.dp, top = 12.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, fontSize = 20.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current,
            color = colors.text, modifier = Modifier.weight(1f))
        if (onArrow != null) {
            FirmiumIconButton(onClick = onArrow, modifier = Modifier.size(36.dp)) {
                FirmiumIcon(Icons.AutoMirrored.Filled.ArrowForward, contentDescription = "See all",
                    tint = colors.accent, modifier = Modifier.size(20.dp))
            }
        }
    }
}

@Composable
private fun OverlayIconButton(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    desc: String,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier.size(40.dp).clip(CircleShape).background(Color.Black.copy(alpha = 0.35f))
            .clickable(enabled = enabled) { onClick() },
        contentAlignment = Alignment.Center,
    ) {
        FirmiumIcon(icon, contentDescription = desc, tint = Color.White.copy(alpha = if (enabled) 1f else 0.4f), modifier = Modifier.size(20.dp))
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
            fontFamily = LocalAppFontFamily.current,
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
                fontFamily = LocalAppFontFamily.current,
                color = colors.muted,
                lineHeight = 20.sp,
            )
        }
    }
}
