package com.fossisawesome.firmium.wear

import androidx.compose.foundation.Image
import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.VolumeDown
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material.icons.filled.SkipPrevious
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.rotary.onRotaryScrollEvent
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.LaunchedEffect
import androidx.wear.compose.material.Button
import androidx.wear.compose.material.ButtonDefaults
import androidx.wear.compose.material.CompactButton
import androidx.wear.compose.material.Icon
import androidx.wear.compose.material.MaterialTheme
import androidx.wear.compose.material.Scaffold
import androidx.wear.compose.material.Text
import androidx.wear.compose.material.TimeText

private const val VOLUME_STEP = 0.05f

@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun RemoteScreen(client: WearPlaybackClient) {
    val state by client.state.collectAsState()
    val focusRequester = remember { FocusRequester() }

    Scaffold(timeText = { TimeText() }) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                // Rotary crown / bezel adjusts phone volume.
                .onRotaryScrollEvent { event ->
                    if (state.hasTrack) {
                        val delta = if (event.verticalScrollPixels > 0f) VOLUME_STEP else -VOLUME_STEP
                        client.setVolume((state.volume + delta).coerceIn(0f, 1f))
                    }
                    true
                }
                .focusRequester(focusRequester)
                .focusable(),
            contentAlignment = Alignment.Center,
        ) {
            if (state.hasTrack) NowPlaying(state, client) else EmptyState()
        }
    }

    LaunchedEffect(Unit) { focusRequester.requestFocus() }
}

@Composable
private fun NowPlaying(state: WatchState, client: WearPlaybackClient) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        val art = state.art
        if (art != null) {
            Image(
                bitmap = art.asImageBitmap(),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier
                    .size(44.dp)
                    .clip(CircleShape),
            )
            Spacer(Modifier.height(4.dp))
        }

        Text(
            text = state.title,
            style = MaterialTheme.typography.title3,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
        )
        Text(
            text = state.artist,
            style = MaterialTheme.typography.caption1,
            color = MaterialTheme.colors.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
        )

        Spacer(Modifier.height(8.dp))

        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            ControlButton(Icons.Filled.SkipPrevious, "Previous") {
                client.sendCommand(WearContract.CMD_PREV)
            }
            Button(
                onClick = { client.sendCommand(WearContract.CMD_PLAY_PAUSE) },
                modifier = Modifier.size(ButtonDefaults.LargeButtonSize),
            ) {
                Icon(
                    imageVector = if (state.isPlaying) Icons.Filled.Pause else Icons.Filled.PlayArrow,
                    contentDescription = if (state.isPlaying) "Pause" else "Play",
                )
            }
            ControlButton(Icons.Filled.SkipNext, "Next") {
                client.sendCommand(WearContract.CMD_NEXT)
            }
        }

        Spacer(Modifier.height(6.dp))

        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            CompactButton(
                onClick = { client.setVolume((state.volume - VOLUME_STEP).coerceIn(0f, 1f)) },
                colors = ButtonDefaults.secondaryButtonColors(),
            ) { Icon(Icons.AutoMirrored.Filled.VolumeDown, contentDescription = "Volume down") }
            Text(
                text = "${(state.volume * 100).toInt()}%",
                style = MaterialTheme.typography.caption2,
            )
            CompactButton(
                onClick = { client.setVolume((state.volume + VOLUME_STEP).coerceIn(0f, 1f)) },
                colors = ButtonDefaults.secondaryButtonColors(),
            ) { Icon(Icons.AutoMirrored.Filled.VolumeUp, contentDescription = "Volume up") }
        }
    }
}

@Composable
private fun ControlButton(icon: ImageVector, label: String, onClick: () -> Unit) {
    Button(
        onClick = onClick,
        colors = ButtonDefaults.secondaryButtonColors(),
        modifier = Modifier.size(ButtonDefaults.SmallButtonSize),
    ) {
        Icon(imageVector = icon, contentDescription = label)
    }
}

@Composable
private fun EmptyState() {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 20.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            imageVector = Icons.Filled.MusicNote,
            contentDescription = null,
            modifier = Modifier.size(28.dp),
            tint = MaterialTheme.colors.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            text = "Nothing playing",
            style = MaterialTheme.typography.title3,
            textAlign = TextAlign.Center,
        )
        Text(
            text = "Open Firmium on your phone and start a track.",
            style = MaterialTheme.typography.caption1,
            color = MaterialTheme.colors.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}
