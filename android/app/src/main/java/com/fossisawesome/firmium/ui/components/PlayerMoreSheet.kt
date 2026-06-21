package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.PlaylistAdd
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Equalizer
import androidx.compose.material.icons.filled.GraphicEq
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Person
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

private data class MoreAction(val icon: ImageVector, val label: String, val onClick: () -> Unit)

// Now-playing "more" menu — a grid of icon-over-label tiles (screenshot-2 style) plus a volume
// slider. Each handler dismisses the sheet before acting so overlays it opens aren't covered.
@Composable
fun PlayerMoreSheet(
    volume: Float,
    onVolumeChange: (Float) -> Unit,
    onAddToPlaylist: () -> Unit,
    onViewArtist: () -> Unit,
    onAddToQueue: () -> Unit,
    onTrackInfo: () -> Unit,
    onEqualizer: () -> Unit,
    onDownload: () -> Unit,
    onToggleVisualizer: (() -> Unit)?,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current

    val actions = buildList {
        add(MoreAction(Icons.AutoMirrored.Filled.PlaylistAdd, "Add to playlist") { onDismiss(); onAddToPlaylist() })
        add(MoreAction(Icons.AutoMirrored.Filled.QueueMusic, "Add to queue") { onDismiss(); onAddToQueue() })
        add(MoreAction(Icons.Default.Info, "Track info") { onDismiss(); onTrackInfo() })
        add(MoreAction(Icons.Default.Person, "View artist") { onDismiss(); onViewArtist() })
        if (onToggleVisualizer != null) {
            add(MoreAction(Icons.Default.GraphicEq, "Visualizer") { onDismiss(); onToggleVisualizer() })
        }
        add(MoreAction(Icons.Default.Equalizer, "Equalizer") { onDismiss(); onEqualizer() })
        add(MoreAction(Icons.Default.Download, "Download") { onDismiss(); onDownload() })
    }

    FirmiumBottomSheet(onDismiss = onDismiss) {
        // Volume slider at the top of the sheet.
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            FirmiumIcon(Icons.AutoMirrored.Filled.VolumeUp, contentDescription = "Volume",
                tint = colors.muted, modifier = Modifier.size(20.dp))
            FirmiumSlider(
                value = volume,
                onValueChange = onVolumeChange,
                modifier = Modifier.weight(1f),
                trackColor = colors.surface2,
                fillColor = colors.accent,
            )
        }
        FirmiumDivider()

        // Icon-over-label tile grid, 3 per row.
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            actions.chunked(3).forEach { row ->
                Row(modifier = Modifier.fillMaxWidth()) {
                    row.forEach { action ->
                        MoreTile(action, modifier = Modifier.weight(1f))
                    }
                    // Pad incomplete rows so tiles keep a consistent width.
                    repeat(3 - row.size) { Spacer(Modifier.weight(1f)) }
                }
            }
        }
        Spacer(Modifier.height(16.dp))
    }
}

@Composable
private fun MoreTile(action: MoreAction, modifier: Modifier = Modifier) {
    val colors = LocalFirmiumColors.current
    Column(
        modifier = modifier
            .clip(RoundedCornerShape(10.dp))
            .clickable { action.onClick() }
            .padding(vertical = 14.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(
            modifier = Modifier.size(48.dp).clip(RoundedCornerShape(50)).background(colors.surface2),
            contentAlignment = Alignment.Center,
        ) {
            FirmiumIcon(action.icon, contentDescription = action.label,
                tint = colors.text, modifier = Modifier.size(22.dp))
        }
        Text(action.label, fontSize = 11.sp, fontFamily = FontFamily.Monospace,
            color = colors.muted, textAlign = TextAlign.Center, maxLines = 2)
    }
}
