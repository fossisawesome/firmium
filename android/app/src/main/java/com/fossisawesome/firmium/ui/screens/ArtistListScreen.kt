package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.ArtistListState

@Composable
fun ArtistListScreen(
    state: ArtistListState,
    coverUrlFor: (String?) -> String?,
    onArtistClick: (String) -> Unit,
    onLoad: () -> Unit,
) {
    LaunchedEffect(Unit) { onLoad() }

    val colors = LocalFirmiumColors.current

    when {
        state.isLoading && state.artists.isEmpty() -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp))
        }
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
        else -> LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(bottom = 16.dp),
        ) {
            items(state.artists, key = { it.id }) { artist ->
                ArtistRow(artist, coverUrlFor(artist.coverArt), onArtistClick)
                FirmiumDivider()
            }
        }
    }
}

// .artist-row: flex row, gap 12dp, padding 10dp. Avatar: 44dp circle. Name: bold monospace.
@Composable
private fun ArtistRow(artist: Artist, coverUrl: String?, onArtistClick: (String) -> Unit) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onArtistClick(artist.id) }
            .padding(horizontal = 10.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        // Circular avatar — matches .artist-row-avatar { border-radius: 50% }
        CoverImage(
            url = coverUrl,
            contentDescription = artist.name,
            modifier = Modifier.size(44.dp).clip(CircleShape),
        )
        Column(modifier = Modifier.weight(1f)) {
            Text(
                artist.name, fontSize = 14.sp, fontWeight = FontWeight.Bold,
                fontFamily = LocalAppFontFamily.current, color = colors.text,
            )
            Text(
                "${artist.albumCount} album${if (artist.albumCount != 1) "s" else ""}",
                fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted,
            )
        }
    }
}
