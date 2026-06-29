package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Square album/single/EP card with a centered play overlay, title, and year — used in the artist
// page's "Albums" and "Singles & EPs" carousels. Tapping the card opens the album; tapping the
// play overlay starts playback.
@Composable
fun AlbumCard(
    album: Album,
    coverUrl: String?,
    onClick: (String) -> Unit,
    onPlay: ((Album) -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = modifier.clickable { onClick(album.id) },
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Box(
            modifier = Modifier.fillMaxWidth().aspectRatio(1f)
                .clip(RoundedCornerShape(8.dp)).background(colors.surface2),
        ) {
            CoverImage(url = coverUrl, contentDescription = album.name, modifier = Modifier.fillMaxSize())
            if (onPlay != null) {
                Box(
                    modifier = Modifier
                        .align(Alignment.Center)
                        .size(44.dp)
                        .clip(CircleShape)
                        .background(Color.Black.copy(alpha = 0.45f))
                        .clickable { onPlay(album) },
                    contentAlignment = Alignment.Center,
                ) {
                    FirmiumIcon(Icons.Default.PlayArrow, contentDescription = "Play",
                        tint = Color.White, modifier = Modifier.size(26.dp))
                }
            }
        }
        Text(
            album.name, fontSize = 13.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current,
            color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
        )
        album.year?.let { year ->
            Text("$year", fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)
        }
    }
}
