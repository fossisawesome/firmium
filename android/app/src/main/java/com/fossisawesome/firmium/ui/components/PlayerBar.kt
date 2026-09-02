package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.basicMarquee
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.filled.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.audio.PlayerState
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.ui.theme.LocalUiTheme

// Persistent mini-player bar shown above the bottom navigation. Dispatches to the
// default or Spotify layout based on the active UI theme.
@Composable
fun PlayerBar(
    state: PlayerState,
    coverUrl: String?,
    onBarClick: () -> Unit,
    onPlayPause: () -> Unit,
    onNext: () -> Unit,
    onShuffleToggle: () -> Unit,
    onRepeatCycle: () -> Unit,
    onToggleStar: () -> Unit = {},
    onQueueOpen: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    if (LocalUiTheme.current == "spotify") {
        PlayerBarSpotify(state, coverUrl, onBarClick, onPlayPause, onNext, onToggleStar, modifier)
    } else {
        PlayerBarDefault(state, coverUrl, onBarClick, onPlayPause, onNext, onShuffleToggle, onRepeatCycle, onQueueOpen, modifier)
    }
}

@Composable
private fun PlayerBarDefault(
    state: PlayerState,
    coverUrl: String?,
    onBarClick: () -> Unit,
    onPlayPause: () -> Unit,
    onNext: () -> Unit,
    onShuffleToggle: () -> Unit,
    onRepeatCycle: () -> Unit,
    onQueueOpen: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    val track = state.currentTrack ?: return
    val colors = LocalFirmiumColors.current

    Column(
        modifier = modifier
            .fillMaxWidth()
            .background(colors.surface)
            .clickable { onBarClick() },
    ) {
        FirmiumDivider(color = colors.border)
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CoverImage(
                url = coverUrl,
                contentDescription = track.album,
                modifier = Modifier
                    .size(48.dp)
                    .clip(RoundedCornerShape(4.dp)),
            )
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = track.title,
                    fontSize = 14.sp,
                    fontFamily = LocalAppFontFamily.current,
                    color = colors.text,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.basicMarquee(),
                )
                Text(
                    text = track.displayArtist ?: track.artist,
                    fontSize = 12.sp,
                    fontFamily = LocalAppFontFamily.current,
                    color = colors.muted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.basicMarquee(),
                )
                val trackInfo = track.formatTrackInfo()
                if (trackInfo.isNotEmpty()) {
                    Text(
                        text = trackInfo,
                        fontSize = 11.sp,
                        fontFamily = LocalAppFontFamily.current,
                        color = colors.muted,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            Spacer(Modifier.width(4.dp))
            FirmiumIconButton(onClick = onShuffleToggle, modifier = Modifier.size(44.dp)) {
                FirmiumIcon(Icons.Default.Shuffle, contentDescription = "Shuffle",
                    tint = if (state.shuffleEnabled) colors.accent else colors.muted,
                    modifier = Modifier.size(18.dp))
            }
            FirmiumIconButton(onClick = onPlayPause, modifier = Modifier.size(44.dp)) {
                FirmiumIcon(
                    imageVector = when (state.playbackState) {
                        "playing" -> Icons.Default.Pause
                        "loading" -> Icons.Default.HourglassEmpty
                        else -> Icons.Default.PlayArrow
                    },
                    contentDescription = if (state.playbackState == "playing") "Pause" else "Play",
                    tint = colors.text,
                )
            }
            FirmiumIconButton(
                onClick = onNext,
                enabled = state.hasNext,
                modifier = Modifier.size(44.dp),
            ) {
                FirmiumIcon(Icons.Default.SkipNext, contentDescription = "Next",
                    tint = if (state.hasNext) colors.text else colors.muted)
            }
            Box(contentAlignment = Alignment.TopEnd) {
                FirmiumIconButton(onClick = onRepeatCycle, modifier = Modifier.size(44.dp)) {
                    FirmiumIcon(
                        imageVector = if (state.repeatMode == "one") Icons.Default.RepeatOne else Icons.Default.Repeat,
                        contentDescription = "Repeat",
                        tint = if (state.repeatMode != "none") colors.accent else colors.muted,
                        modifier = Modifier.size(18.dp))
                }
                if (state.repeatMode == "one") {
                    Text("1", fontSize = 9.sp, color = colors.accent, fontWeight = FontWeight.Bold,
                        modifier = Modifier.offset(x = (-6).dp, y = 4.dp))
                }
            }
            if (onQueueOpen != null) {
                FirmiumIconButton(onClick = onQueueOpen, modifier = Modifier.size(44.dp)) {
                    FirmiumIcon(Icons.AutoMirrored.Filled.QueueMusic, contentDescription = "Queue",
                        tint = colors.muted, modifier = Modifier.size(18.dp))
                }
            }
        }
    }
}

// Spotify-style mini player: thin progress line across the top edge, larger cover,
// and only play/pause + next (shuffle/repeat live in the full-screen player), matching
// Spotify's mobile mini-player instead of Firmium's full transport-row bar.
@Composable
private fun PlayerBarSpotify(
    state: PlayerState,
    coverUrl: String?,
    onBarClick: () -> Unit,
    onPlayPause: () -> Unit,
    onNext: () -> Unit,
    onToggleStar: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val track = state.currentTrack ?: return
    val colors = LocalFirmiumColors.current
    val progress = if (state.trackDuration > 0) {
        (state.currentPosition / state.trackDuration).toFloat().coerceIn(0f, 1f)
    } else 0f

    Column(
        modifier = modifier
            .fillMaxWidth()
            .background(colors.surface2)
            .clickable { onBarClick() },
    ) {
        Box(modifier = Modifier.fillMaxWidth().height(2.dp).background(colors.border)) {
            Box(modifier = Modifier.fillMaxHeight().fillMaxWidth(progress).background(colors.accent))
        }
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CoverImage(
                url = coverUrl,
                contentDescription = track.album,
                modifier = Modifier
                    .size(52.dp)
                    .clip(RoundedCornerShape(6.dp)),
            )
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = track.title,
                    fontSize = 14.sp,
                    fontFamily = LocalAppFontFamily.current,
                    color = colors.text,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.basicMarquee(),
                )
                Text(
                    text = track.displayArtist ?: track.artist,
                    fontSize = 12.sp,
                    fontFamily = LocalAppFontFamily.current,
                    color = colors.muted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            FavoriteButton(starred = track.starred, onToggle = onToggleStar, size = 16.dp)
            Spacer(Modifier.width(4.dp))
            FirmiumIconButton(onClick = onPlayPause, modifier = Modifier.size(44.dp)) {
                FirmiumIcon(
                    imageVector = when (state.playbackState) {
                        "playing" -> Icons.Default.Pause
                        "loading" -> Icons.Default.HourglassEmpty
                        else -> Icons.Default.PlayArrow
                    },
                    contentDescription = if (state.playbackState == "playing") "Pause" else "Play",
                    tint = colors.text,
                )
            }
            FirmiumIconButton(
                onClick = onNext,
                enabled = state.hasNext,
                modifier = Modifier.size(44.dp),
            ) {
                FirmiumIcon(Icons.Default.SkipNext, contentDescription = "Next",
                    tint = if (state.hasNext) colors.text else colors.muted)
            }
        }
    }
}
