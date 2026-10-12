package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.data.db.PodcastEpisodeEntity
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

@Composable
fun PodcastDetailScreen(
    title: String,
    episodes: List<PodcastEpisodeEntity>,
    playingEpisodeId: String?,
    isPlaying: Boolean,
    onRefresh: () -> Unit,
    onUnsubscribe: () -> Unit,
    onPlayEpisode: (PodcastEpisodeEntity) -> Unit,
    onPlayPause: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val nowPlayingEpisode = episodes.find { it.id == playingEpisodeId }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 8.dp, top = 10.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                title, fontSize = 16.sp, fontWeight = FontWeight.Bold, fontFamily = LocalAppFontFamily.current,
                color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
            FirmiumIconButton(onClick = onRefresh, modifier = Modifier.size(40.dp)) {
                FirmiumIcon(Icons.Default.Refresh, contentDescription = "Refresh", tint = colors.muted)
            }
            Text(
                "Unsubscribe", fontSize = 12.sp, fontFamily = LocalAppFontFamily.current, color = colors.error,
                modifier = Modifier.clickable { onUnsubscribe() }.padding(8.dp),
            )
        }

        if (nowPlayingEpisode != null) {
            NowPlayingRow(episode = nowPlayingEpisode, isPlaying = isPlaying, onPlayPause = onPlayPause)
        }

        if (episodes.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No episodes found.", fontFamily = LocalAppFontFamily.current, fontSize = 14.sp, color = colors.muted)
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp),
            ) {
                items(episodes, key = { it.id }) { episode ->
                    val isCurrent = episode.id == playingEpisodeId
                    PodcastEpisodeRow(
                        episode = episode,
                        isPlaying = isCurrent && isPlaying,
                        onPlay = { if (isCurrent) onPlayPause() else onPlayEpisode(episode) },
                    )
                    FirmiumDivider()
                }
            }
        }
    }
}

// Persistent inline control for the currently-playing episode in this channel —
// satisfies "user can pause/play from within the app" without requiring the
// shared full-screen player to be open. 48dp touch target per Material spec.
@Composable
private fun NowPlayingRow(episode: PodcastEpisodeEntity, isPlaying: Boolean, onPlayPause: () -> Unit) {
    val colors = LocalFirmiumColors.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(colors.surface2)
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("NOW PLAYING", fontSize = 10.sp, fontFamily = LocalAppFontFamily.current, color = colors.accent, letterSpacing = 1.sp)
            Text(
                episode.title, fontFamily = LocalAppFontFamily.current, fontSize = 13.sp,
                color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
        }
        Box(
            modifier = Modifier
                .size(48.dp)
                .clip(CircleShape)
                .background(colors.accent)
                .clickable { onPlayPause() },
            contentAlignment = Alignment.Center,
        ) {
            FirmiumIcon(
                if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                contentDescription = if (isPlaying) "Pause" else "Play",
                tint = colors.bg,
            )
        }
    }
}

@Composable
private fun PodcastEpisodeRow(
    episode: PodcastEpisodeEntity,
    isPlaying: Boolean,
    onPlay: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val durationLabel = episode.durationSeconds?.let { "${it / 60}m${it % 60}s" }.orEmpty()

    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                episode.title, fontFamily = LocalAppFontFamily.current, fontSize = 13.sp,
                color = if (isPlaying) colors.accent else colors.text,
                maxLines = 2, overflow = TextOverflow.Ellipsis,
            )
            Text(durationLabel, fontSize = 11.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)
        }
        FirmiumIconButton(onClick = onPlay, modifier = Modifier.size(48.dp)) {
            FirmiumIcon(
                if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                contentDescription = if (isPlaying) "Pause" else "Play",
                tint = colors.accent,
            )
        }
    }
}
