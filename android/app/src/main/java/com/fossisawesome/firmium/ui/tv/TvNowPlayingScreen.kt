package com.fossisawesome.firmium.ui.tv

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.audio.PlayerState
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.components.CoverImage
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.LyricsState
import com.fossisawesome.firmium.viewmodel.SimilarTracksState
import kotlin.math.roundToInt

private enum class TvNowPlayingPanel { NONE, QUEUE, LYRICS, SIMILAR }

@Composable
fun TvNowPlayingScreen(
    state: PlayerState,
    coverUrl: String?,
    lyricsState: LyricsState,
    similarTracksState: SimilarTracksState,
    onPlayPause: () -> Unit,
    onNext: () -> Unit,
    onPrevious: () -> Unit,
    onSkipToIndex: (Int) -> Unit,
    onLyricsOpen: () -> Unit,
    onLyricsClose: () -> Unit,
    onFetchSimilarTracks: () -> Unit,
    onClearSimilarTracks: () -> Unit,
    onPlaySimilar: (List<Song>, Int) -> Unit,
    onBack: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var panel by remember { mutableStateOf(TvNowPlayingPanel.NONE) }

    BackHandler {
        if (panel != TvNowPlayingPanel.NONE) {
            if (panel == TvNowPlayingPanel.LYRICS) onLyricsClose()
            panel = TvNowPlayingPanel.NONE
        } else {
            onBack()
        }
    }

    val track = state.currentTrack

    Row(modifier = Modifier.fillMaxSize().padding(48.dp)) {
        Column(modifier = Modifier.width(360.dp).padding(end = 48.dp)) {
            CoverImage(url = coverUrl, contentDescription = track?.title, size = 320.dp)
            Text(text = track?.title ?: "Nothing playing", color = colors.text, fontSize = 20.sp, maxLines = 2, modifier = Modifier.padding(top = 24.dp))
            Text(text = track?.artist ?: "", color = colors.muted, fontSize = 15.sp, modifier = Modifier.padding(top = 4.dp))
            Text(text = track?.album ?: "", color = colors.muted, fontSize = 13.sp)

            if (state.trackDuration > 0) {
                val progress = (state.currentPosition / state.trackDuration).toFloat().coerceIn(0f, 1f)
                Box(
                    modifier = Modifier
                        .width(320.dp)
                        .height(4.dp)
                        .padding(top = 24.dp)
                        .background(colors.border),
                ) {
                    Box(modifier = Modifier.fillMaxHeight().fillMaxWidth(progress).background(colors.accent))
                }
            }

            Row(
                modifier = Modifier.padding(top = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                TvActionButton(onClick = onPrevious, colors = colors) {
                    Text(text = "Prev", color = colors.text, fontSize = 13.sp)
                }
                TvActionButton(onClick = onPlayPause, colors = colors) {
                    Text(text = if (state.playbackState == "playing") "Pause" else "Play", color = colors.text, fontSize = 13.sp)
                }
                TvActionButton(onClick = onNext, colors = colors) {
                    Text(text = "Next", color = colors.text, fontSize = 13.sp)
                }
            }
            Row(
                modifier = Modifier.padding(top = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                TvActionButton(onClick = { panel = if (panel == TvNowPlayingPanel.QUEUE) TvNowPlayingPanel.NONE else TvNowPlayingPanel.QUEUE }, colors = colors) {
                    Text(text = "Queue", color = colors.text, fontSize = 13.sp)
                }
                TvActionButton(
                    onClick = {
                        if (panel == TvNowPlayingPanel.LYRICS) {
                            onLyricsClose(); panel = TvNowPlayingPanel.NONE
                        } else {
                            onLyricsOpen(); panel = TvNowPlayingPanel.LYRICS
                        }
                    },
                    colors = colors,
                ) {
                    Text(text = "Lyrics", color = colors.text, fontSize = 13.sp)
                }
                TvActionButton(
                    onClick = {
                        if (panel == TvNowPlayingPanel.SIMILAR) {
                            onClearSimilarTracks(); panel = TvNowPlayingPanel.NONE
                        } else {
                            onFetchSimilarTracks(); panel = TvNowPlayingPanel.SIMILAR
                        }
                    },
                    colors = colors,
                ) {
                    Text(text = "Similar", color = colors.text, fontSize = 13.sp)
                }
            }
        }

        when (panel) {
            TvNowPlayingPanel.QUEUE -> TvQueuePanel(state, colors, onSkipToIndex)
            TvNowPlayingPanel.LYRICS -> TvLyricsPanel(lyricsState, colors)
            TvNowPlayingPanel.SIMILAR -> TvSimilarTracksPanel(similarTracksState, colors, onPlaySimilar)
            TvNowPlayingPanel.NONE -> {}
        }
    }
}

@Composable
private fun TvQueuePanel(state: PlayerState, colors: com.fossisawesome.firmium.ui.theme.FirmiumColors, onSkipToIndex: (Int) -> Unit) {
    LazyColumn(modifier = Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        items(state.queue.size) { index ->
            val song = state.queue[index]
            val isCurrent = index == state.queueIndex
            TvTile(onClick = { onSkipToIndex(index) }, colors = colors, modifier = Modifier.fillMaxWidth()) {
                Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = song.title,
                        color = if (isCurrent) colors.accent else colors.text,
                        fontSize = 13.sp,
                        maxLines = 1,
                        modifier = Modifier.weight(1f),
                    )
                    Text(text = song.artist, color = colors.muted, fontSize = 11.sp, maxLines = 1)
                }
            }
        }
    }
}

// Synced-line highlighting only — no word-by-word karaoke fill on TV (Phase 2 scope trim).
@Composable
private fun TvLyricsPanel(state: LyricsState, colors: com.fossisawesome.firmium.ui.theme.FirmiumColors) {
    val listState = rememberLazyListState()

    LaunchedEffect(state.activeLine) {
        if (state.activeLine >= 0) listState.animateScrollToItem((state.activeLine - 2).coerceAtLeast(0))
    }

    if (state.isLoading) {
        Text(text = "Loading lyrics…", color = colors.muted, fontSize = 14.sp, modifier = Modifier.fillMaxSize().padding(24.dp))
        return
    }
    if (state.lines.isEmpty()) {
        Text(text = "No lyrics found", color = colors.muted, fontSize = 14.sp, modifier = Modifier.fillMaxSize().padding(24.dp))
        return
    }

    LazyColumn(
        state = listState,
        modifier = Modifier.fillMaxSize().padding(horizontal = 24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(state.lines.size) { index ->
            val line = state.lines[index]
            val isActive = index == state.activeLine
            Text(
                text = line.text,
                color = if (isActive) colors.accent else colors.muted,
                fontSize = if (isActive) 20.sp else 15.sp,
                fontWeight = if (isActive) androidx.compose.ui.text.font.FontWeight.Bold else null,
            )
        }
    }
}

@Composable
private fun TvSimilarTracksPanel(
    state: SimilarTracksState,
    colors: com.fossisawesome.firmium.ui.theme.FirmiumColors,
    onPlaySimilar: (List<Song>, Int) -> Unit,
) {
    if (state.isLoading) {
        Text(text = "Finding similar tracks…", color = colors.muted, fontSize = 14.sp, modifier = Modifier.fillMaxSize().padding(24.dp))
        return
    }
    if (state.error != null || state.matches.isEmpty()) {
        Text(text = state.error ?: "No similar tracks found", color = colors.muted, fontSize = 14.sp, modifier = Modifier.fillMaxSize().padding(24.dp))
        return
    }
    val songs = state.matches.map { it.song }
    LazyColumn(modifier = Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        items(state.matches.size) { index ->
            val match: ApiClient.SimilarMatch = state.matches[index]
            TvTile(onClick = { onPlaySimilar(songs, index) }, colors = colors, modifier = Modifier.fillMaxWidth()) {
                Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(text = match.song.title, color = colors.text, fontSize = 13.sp, maxLines = 1, modifier = Modifier.weight(1f))
                    Text(text = "${(match.similarity * 100).roundToInt()}%", color = colors.muted, fontSize = 11.sp)
                }
            }
        }
    }
}
