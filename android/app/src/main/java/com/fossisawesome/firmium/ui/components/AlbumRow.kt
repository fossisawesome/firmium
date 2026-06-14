package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.runtime.Composable
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
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

@Composable
fun AlbumRow(
    album: Album,
    coverUrl: String?,
    onAlbumClick: (String) -> Unit,
    onAddClick: () -> Unit,
    onDownloadClick: (suspend () -> Result<Unit>)? = null,
    showArtist: Boolean = true,
    coverSize: Dp = 44.dp,
    coverRadius: Dp = 6.dp,
) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onAlbumClick(album.id) }.padding(10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        CoverImage(
            url = coverUrl,
            contentDescription = album.name,
            modifier = Modifier.size(coverSize).clip(RoundedCornerShape(coverRadius))
                .background(colors.surface2),
        )
        Column(modifier = Modifier.weight(1f)) {
            Text(
                album.name, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace,
                color = colors.text, fontSize = 14.sp, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            val meta = listOfNotNull(
                album.artist.takeIf { showArtist && it.isNotBlank() },
                album.year?.toString(),
            ).joinToString(" · ")
            if (meta.isNotBlank()) {
                Text(meta, fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        if (onDownloadClick != null) {
            DownloadButton(onDownload = onDownloadClick)
        }
        FirmiumIconButton(onClick = onAddClick, modifier = Modifier.size(36.dp)) {
            FirmiumIcon(Icons.Default.Add, contentDescription = "Add to playlist", tint = colors.muted, modifier = Modifier.size(18.dp))
        }
    }
}
