package com.fossisawesome.firmium.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
    onRefresh: () -> Unit,
    onUnsubscribe: () -> Unit,
    onPlayEpisode: (PodcastEpisodeEntity) -> Unit,
) {
    val colors = LocalFirmiumColors.current

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 8.dp, top = 10.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                title, fontSize = 16.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace,
                color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
            FirmiumIconButton(onClick = onRefresh, modifier = Modifier.size(40.dp)) {
                FirmiumIcon(Icons.Default.Refresh, contentDescription = "Refresh", tint = colors.muted)
            }
            Text(
                "Unsubscribe", fontSize = 12.sp, fontFamily = FontFamily.Monospace, color = colors.error,
                modifier = Modifier.clickable { onUnsubscribe() }.padding(8.dp),
            )
        }

        if (episodes.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No episodes found.", fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = colors.muted)
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 16.dp),
            ) {
                items(episodes, key = { it.id }) { episode ->
                    PodcastEpisodeRow(
                        episode = episode,
                        isPlaying = episode.id == playingEpisodeId,
                        onPlay = { onPlayEpisode(episode) },
                    )
                    FirmiumDivider()
                }
            }
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
                episode.title, fontFamily = FontFamily.Monospace, fontSize = 13.sp,
                color = if (isPlaying) colors.accent else colors.text,
                maxLines = 2, overflow = TextOverflow.Ellipsis,
            )
            Text(durationLabel, fontSize = 11.sp, fontFamily = FontFamily.Monospace, color = colors.muted)
        }
        FirmiumIconButton(onClick = onPlay, modifier = Modifier.size(36.dp)) {
            FirmiumIcon(Icons.Default.PlayArrow, contentDescription = "Play", tint = colors.accent)
        }
    }
}
