package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.clickable
import androidx.compose.foundation.background
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.FavoritesState

@Composable
fun FavoritesScreen(
    state: FavoritesState,
    coverUrlFor: (String?) -> String?,
    onLoad: () -> Unit,
    onAlbumClick: (String) -> Unit,
    onArtistClick: (String) -> Unit,
    onPlaySong: (Song) -> Unit,
    onToggleSongStar: (Song) -> Unit,
    onAddToQueue: ((Song) -> Unit)? = null,
    onBack: () -> Unit,
) {
    LaunchedEffect(Unit) { onLoad() }
    val colors = LocalFirmiumColors.current
    var longPressSong by remember { mutableStateOf<Song?>(null) }

    Column(modifier = Modifier.fillMaxSize()) {
        FirmiumDetailHeader(title = "Favorites", onBack = onBack)
        when {
            state.isLoading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp))
            }
            state.error != null -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text(state.error, color = colors.error, fontFamily = LocalAppFontFamily.current, fontSize = 13.sp)
            }
            state.artists.isEmpty() && state.albums.isEmpty() && state.songs.isEmpty() -> {
                Box(Modifier.fillMaxSize().padding(32.dp), contentAlignment = Alignment.Center) {
                    Text(
                        "No favorites yet — tap the heart on any song, album, or artist.",
                        color = colors.muted, fontFamily = LocalAppFontFamily.current, fontSize = 13.sp,
                    )
                }
            }
            else -> LazyColumn(contentPadding = PaddingValues(bottom = 32.dp)) {
                if (state.albums.isNotEmpty()) {
                    item {
                        SectionTitle("Albums")
                        LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            items(state.albums, key = { it.id }) { album -> FavoriteAlbumCard(album, coverUrlFor(album.coverArt), onAlbumClick) }
                        }
                        Spacer(Modifier.height(24.dp))
                    }
                }
                if (state.artists.isNotEmpty()) {
                    item {
                        SectionTitle("Artists")
                        LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            items(state.artists, key = { it.id }) { artist -> FavoriteArtistChip(artist, onArtistClick) }
                        }
                        Spacer(Modifier.height(24.dp))
                    }
                }
                if (state.songs.isNotEmpty()) {
                    item { SectionTitle("Songs") }
                    items(state.songs, key = { it.id }) { song ->
                        TrackRow(
                            track = song,
                            index = null,
                            isCurrentlyPlaying = false,
                            onClick = { onPlaySong(song) },
                            onToggleStar = { onToggleSongStar(song) },
                            onAddToQueue = onAddToQueue?.let { { it(song) } },
                            onLongPress = if (onAddToQueue != null) { { longPressSong = song } } else null,
                        )
                        FirmiumDivider()
                    }
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

@Composable
private fun SectionTitle(title: String) {
    Text(
        title, fontSize = 18.sp, fontWeight = FontWeight.Bold,
        fontFamily = LocalAppFontFamily.current, color = LocalFirmiumColors.current.text,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
    )
}

@Composable
private fun FavoriteAlbumCard(album: Album, coverUrl: String?, onClick: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.width(130.dp).clickable { onClick(album.id) }, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        CoverImage(url = coverUrl, contentDescription = album.name, modifier = Modifier.size(130.dp).clip(RoundedCornerShape(10.dp)).background(colors.surface2))
        Text(album.name, fontSize = 12.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current, color = colors.text, maxLines = 1)
        Text(album.artist, fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted, maxLines = 1)
    }
}

@Composable
private fun FavoriteArtistChip(artist: Artist, onClick: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    Box(
        modifier = Modifier.clip(RoundedCornerShape(999.dp)).background(colors.surface2)
            .clickable { onClick(artist.id) }.padding(horizontal = 16.dp, vertical = 10.dp),
    ) {
        Text(artist.name, fontSize = 13.sp, fontFamily = LocalAppFontFamily.current, color = colors.text)
    }
}
