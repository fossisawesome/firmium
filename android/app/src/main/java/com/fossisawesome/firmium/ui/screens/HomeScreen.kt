package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.HomeState
import com.fossisawesome.firmium.viewmodel.RecentArtist
import java.util.Calendar

@Composable
fun HomeScreen(
    state: HomeState,
    username: String,
    coverUrlFor: (String?) -> String?,
    onAlbumClick: (String) -> Unit,
    onArtistClick: (String) -> Unit,
    onRefresh: () -> Unit,
) {
    LaunchedEffect(Unit) { onRefresh() }

    val colors = LocalFirmiumColors.current

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp)
            .padding(top = 16.dp, bottom = 16.dp),
    ) {
        HomeGreeting(username)

        Spacer(Modifier.height(8.dp))

        when {
            state.isLoading -> Box(
                Modifier.fillMaxWidth().height(200.dp),
                contentAlignment = Alignment.Center,
            ) {
                FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp))
            }
            state.error != null -> ErrorSection(state.error, onRefresh)
            else -> {
                if (state.recentAlbums.isNotEmpty()) {
                    HomeSectionTitle("Recently Played")
                    Spacer(Modifier.height(14.dp))
                    AlbumRow(state.recentAlbums, coverUrlFor, onAlbumClick, cardWidth = 130.dp)
                    Spacer(Modifier.height(28.dp))
                }

                if (state.recentArtists.isNotEmpty()) {
                    HomeSectionTitle("Recently Played Artists")
                    Spacer(Modifier.height(14.dp))
                    ArtistRow(state.recentArtists, coverUrlFor, onArtistClick)
                    Spacer(Modifier.height(28.dp))
                }

                if (state.randomAlbums.isNotEmpty()) {
                    HomeSectionTitle("Random Picks")
                    Spacer(Modifier.height(14.dp))
                    AlbumRow(state.randomAlbums, coverUrlFor, onAlbumClick, cardWidth = 130.dp)
                    Spacer(Modifier.height(16.dp))
                }
            }
        }
    }
}

// "GOOD AFTERNOON, / Username" — matches .home-greeting-label + .home-greeting-name
@Composable
private fun HomeGreeting(username: String) {
    val colors = LocalFirmiumColors.current
    val hour = Calendar.getInstance().get(Calendar.HOUR_OF_DAY)
    val timeOfDay = when {
        hour in 5..11 -> "Morning"
        hour in 12..16 -> "Afternoon"
        hour in 17..20 -> "Evening"
        else -> "Night"
    }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(
            text = "Good $timeOfDay,".uppercase(),
            fontSize = 13.sp,
            fontFamily = LocalAppFontFamily.current,
            color = colors.muted,
            letterSpacing = 1.sp,
        )
        Text(
            text = username,
            fontSize = 22.sp,
            fontWeight = FontWeight.Bold,
            fontFamily = LocalAppFontFamily.current,
            color = colors.accent,
            letterSpacing = (-0.5).sp,
        )
    }
}

// Section header — 11sp uppercase letter-spaced muted monospace, matches .home-section-title
@Composable
private fun HomeSectionTitle(title: String) {
    Text(
        text = title.uppercase(),
        fontSize = 11.sp,
        fontFamily = LocalAppFontFamily.current,
        color = LocalFirmiumColors.current.muted,
        letterSpacing = 1.sp,
    )
}

@Composable
private fun AlbumRow(
    albums: List<Album>,
    coverUrlFor: (String?) -> String?,
    onAlbumClick: (String) -> Unit,
    cardWidth: Dp,
) {
    LazyRow(
        contentPadding = PaddingValues(end = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(albums, key = { it.id }) { album ->
            HomeAlbumCard(album, coverUrlFor(album.coverArt), cardWidth, onAlbumClick)
        }
    }
}

@Composable
private fun ArtistRow(
    artists: List<RecentArtist>,
    coverUrlFor: (String?) -> String?,
    onArtistClick: (String) -> Unit,
) {
    LazyRow(
        contentPadding = PaddingValues(end = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(artists, key = { it.id }) { artist ->
            HomeArtistCard(artist, coverUrlFor(artist.coverArt), onArtistClick)
        }
    }
}

// Album card — 130dp × 130dp art, 12dp radius, 12sp bold monospace title
@Composable
private fun HomeAlbumCard(album: Album, coverUrl: String?, width: Dp, onClick: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier.width(width).clickable { onClick(album.id) },
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        CoverImage(
            url = coverUrl,
            contentDescription = album.name,
            modifier = Modifier.size(width).clip(RoundedCornerShape(12.dp))
                .background(colors.surface2),
        )
        Column(
            modifier = Modifier.padding(horizontal = 2.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                album.name, fontSize = 12.sp, fontWeight = FontWeight.Bold,
                fontFamily = LocalAppFontFamily.current, color = colors.text,
                maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            Text(
                album.artist, fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
                color = colors.muted, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

// Artist card — 110dp × 110dp square art, 12dp radius, 12sp bold monospace name
@Composable
private fun HomeArtistCard(artist: RecentArtist, coverUrl: String?, onClick: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    val size = 110.dp
    Column(
        modifier = Modifier.width(size).clickable { onClick(artist.id) },
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        CoverImage(
            url = coverUrl,
            contentDescription = artist.name,
            modifier = Modifier.size(size).clip(RoundedCornerShape(12.dp))
                .background(colors.surface2),
        )
        Text(
            artist.name, fontSize = 12.sp, fontWeight = FontWeight.Bold,
            fontFamily = LocalAppFontFamily.current, color = colors.text,
            maxLines = 1, overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(horizontal = 2.dp),
        )
    }
}

@Composable
private fun ErrorSection(message: String, onRetry: () -> Unit) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = Modifier.fillMaxWidth().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text("Failed to load: $message", color = colors.error, fontSize = 13.sp,
            fontFamily = LocalAppFontFamily.current)
        FirmiumTextButton(onClick = onRetry) {
            Text("Retry", fontFamily = LocalAppFontFamily.current, color = colors.accent, fontSize = 14.sp)
        }
    }
}
