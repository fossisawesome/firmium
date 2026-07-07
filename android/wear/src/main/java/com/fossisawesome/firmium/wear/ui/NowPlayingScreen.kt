package com.fossisawesome.firmium.wear.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.VolumeDown
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Repeat
import androidx.compose.material.icons.filled.RepeatOne
import androidx.compose.material.icons.filled.Shuffle
import androidx.compose.material.icons.filled.SkipNext
import androidx.compose.material.icons.filled.SkipPrevious
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.Button
import androidx.wear.compose.material.ButtonDefaults
import androidx.wear.compose.material.CompactButton
import androidx.wear.compose.material.Icon
import androidx.wear.compose.material.MaterialTheme
import androidx.wear.compose.material.Text
import com.fossisawesome.firmium.wear.FirmiumWearApplication

private const val VOLUME_STEP = 0.05f

@Composable
fun NowPlayingScreen(app: FirmiumWearApplication) {
    val state by app.playbackController.state.collectAsState()
    val track = state.currentTrack

    if (track == null) {
        Column(
            modifier = Modifier.fillMaxSize().padding(horizontal = 20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Icon(imageVector = Icons.Filled.MusicNote, contentDescription = null, modifier = Modifier.size(28.dp))
            Spacer(Modifier.height(8.dp))
            Text("Nothing playing", style = MaterialTheme.typography.title3, textAlign = TextAlign.Center)
        }
        return
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        WatchCoverImage(
            url = app.authManager.safeCoverArtUrl(track.coverArt, 128),
            contentDescription = track.title,
            size = 44.dp,
        )
        Spacer(Modifier.height(4.dp))

        Text(
            text = track.title,
            style = MaterialTheme.typography.title3,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
        )
        Text(
            text = track.displayArtist ?: track.artist,
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
            ControlButton(Icons.Filled.SkipPrevious, "Previous") { app.playbackController.skipToPrevious() }
            Button(
                onClick = { app.playbackController.togglePlayPause() },
                modifier = Modifier.size(ButtonDefaults.LargeButtonSize),
            ) {
                Icon(
                    imageVector = if (state.playbackState == "playing") Icons.Filled.Pause else Icons.Filled.PlayArrow,
                    contentDescription = if (state.playbackState == "playing") "Pause" else "Play",
                )
            }
            ControlButton(Icons.Filled.SkipNext, "Next") { app.playbackController.skipToNext() }
        }

        Spacer(Modifier.height(6.dp))

        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            ControlButton(Icons.Filled.Shuffle, "Shuffle") { app.playbackController.toggleShuffle() }
            CompactButton(
                onClick = { app.playbackController.setVolume((state.volume - VOLUME_STEP).coerceIn(0f, 1f)) },
                colors = ButtonDefaults.secondaryButtonColors(),
            ) { Icon(Icons.AutoMirrored.Filled.VolumeDown, contentDescription = "Volume down") }
            Text(text = "${(state.volume * 100).toInt()}%", style = MaterialTheme.typography.caption2)
            CompactButton(
                onClick = { app.playbackController.setVolume((state.volume + VOLUME_STEP).coerceIn(0f, 1f)) },
                colors = ButtonDefaults.secondaryButtonColors(),
            ) { Icon(Icons.AutoMirrored.Filled.VolumeUp, contentDescription = "Volume up") }
            ControlButton(
                icon = if (state.repeatMode == "one") Icons.Filled.RepeatOne else Icons.Filled.Repeat,
                label = "Repeat",
            ) {
                app.playbackController.setRepeatMode(when (state.repeatMode) {
                    "none" -> "all"; "all" -> "one"; else -> "none"
                })
            }
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
