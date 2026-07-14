package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import kotlinx.coroutines.launch

// Queue bottom sheet showing the current play queue.
// Active track is highlighted; tapping a row plays it.
@Composable
fun QueueSheet(
    queue: List<Song>,
    currentIndex: Int,
    onDismiss: () -> Unit,
    onPlayAt: (Int) -> Unit,
    onMove: (Int, Int) -> Unit = { _, _ -> },
    onRemove: (Int) -> Unit = {},
) {
    val colors = LocalFirmiumColors.current
    val listState = rememberLazyListState()
    val coroutineScope = rememberCoroutineScope()

    // Scroll to the active track when the sheet opens.
    LaunchedEffect(currentIndex) {
        if (currentIndex >= 0 && currentIndex < queue.size) {
            coroutineScope.launch {
                listState.animateScrollToItem(currentIndex)
            }
        }
    }

    FirmiumBottomSheet(onDismiss = onDismiss) {
        Text(
            text = "Queue",
            fontSize = 16.sp,
            fontFamily = LocalAppFontFamily.current,
            color = colors.text,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        )

        LazyColumn(
            state = listState,
            modifier = Modifier.fillMaxWidth().heightIn(max = 480.dp),
            contentPadding = PaddingValues(bottom = 32.dp),
        ) {
            itemsIndexed(queue) { index, song ->
                QueueItem(
                    index = index,
                    song = song,
                    isActive = index == currentIndex,
                    onClick = { onPlayAt(index) },
                    onMoveUp = if (index > 0) { { onMove(index, index - 1) } } else null,
                    onMoveDown = if (index < queue.size - 1) { { onMove(index, index + 1) } } else null,
                    onRemove = if (queue.size > 1) { { onRemove(index) } } else null,
                )
                FirmiumDivider(color = colors.border)
            }
        }
    }
}

@Composable
private fun QueueItem(
    index: Int,
    song: Song,
    isActive: Boolean,
    onClick: () -> Unit,
    onMoveUp: (() -> Unit)? = null,
    onMoveDown: (() -> Unit)? = null,
    onRemove: (() -> Unit)? = null,
) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(if (isActive) colors.accent.copy(alpha = 0.08f) else Color.Transparent)
            .clickable { onClick() }
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(modifier = Modifier.width(32.dp), contentAlignment = Alignment.Center) {
            if (isActive) {
                FirmiumIcon(
                    Icons.Default.PlayArrow,
                    contentDescription = "Playing",
                    tint = colors.accent,
                    modifier = Modifier.size(20.dp),
                )
            } else {
                Text(
                    text = "${index + 1}",
                    fontSize = 12.sp,
                    fontFamily = LocalAppFontFamily.current,
                    color = colors.muted,
                )
            }
        }

        Spacer(Modifier.width(12.dp))

        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = song.title,
                fontSize = 14.sp,
                fontFamily = LocalAppFontFamily.current,
                color = if (isActive) colors.accent else colors.text,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = song.displayArtist ?: song.artist,
                fontSize = 12.sp,
                fontFamily = LocalAppFontFamily.current,
                color = colors.muted,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }

        Text(
            text = formatDuration(song.duration),
            fontSize = 12.sp,
            fontFamily = LocalAppFontFamily.current,
            color = colors.muted,
            modifier = Modifier.padding(end = 8.dp),
        )

        // Reorder/remove actions — same pattern as AlbumDetailScreen's TrackRow.
        if (onMoveUp != null || onMoveDown != null) {
            FirmiumIconButton(onClick = { onMoveUp?.invoke() }, enabled = onMoveUp != null, modifier = Modifier.size(40.dp)) {
                FirmiumIcon(Icons.Default.KeyboardArrowUp, contentDescription = "Move up",
                    tint = if (onMoveUp != null) colors.muted else colors.muted.copy(alpha = 0.3f), modifier = Modifier.size(18.dp))
            }
            Spacer(Modifier.width(4.dp))
            FirmiumIconButton(onClick = { onMoveDown?.invoke() }, enabled = onMoveDown != null, modifier = Modifier.size(40.dp)) {
                FirmiumIcon(Icons.Default.KeyboardArrowDown, contentDescription = "Move down",
                    tint = if (onMoveDown != null) colors.muted else colors.muted.copy(alpha = 0.3f), modifier = Modifier.size(18.dp))
            }
        }
        if (onRemove != null) {
            Spacer(Modifier.width(4.dp))
            FirmiumIconButton(onClick = onRemove, modifier = Modifier.size(40.dp)) {
                FirmiumIcon(Icons.Default.Close, contentDescription = "Remove from queue",
                    tint = colors.error, modifier = Modifier.size(18.dp))
            }
        }
    }
}

private fun formatDuration(seconds: Int): String {
    val m = seconds / 60
    val s = seconds % 60
    return "%d:%02d".format(m, s)
}
