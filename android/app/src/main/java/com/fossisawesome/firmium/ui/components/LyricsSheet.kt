package com.fossisawesome.firmium.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.LyricsState
import kotlinx.coroutines.launch

// Full-screen lyrics sheet. Auto-scrolls to the active line for synced lyrics.
// Active line is enlarged/centered with a word-by-word karaoke fill (estimated
// timing, gated by wordFillEnabled), inactive lines are blurred, and the
// background is tinted with the cover art's dominant color.
@Composable
fun LyricsSheet(
    state: LyricsState,
    trackTitle: String,
    coverUrl: String?,
    positionSeconds: Double,
    isPlaying: Boolean,
    wordFillEnabled: Boolean,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()

    // Auto-scroll to active line whenever it changes.
    LaunchedEffect(state.activeLine) {
        if (state.activeLine >= 0 && state.synced) {
            scope.launch {
                listState.animateScrollToItem(
                    index = (state.activeLine - 2).coerceAtLeast(0),
                )
            }
        }
    }

    // Interpolate playback position between the periodic position-poll updates so the
    // per-word fill progresses smoothly (mirrors the rAF loop in LyricsPanel.svelte).
    var interpolatedMs by remember { mutableLongStateOf((positionSeconds * 1000).toLong()) }
    LaunchedEffect(positionSeconds, isPlaying, state.activeLine, state.synced) {
        val basePositionMs = (positionSeconds * 1000).toLong()
        interpolatedMs = basePositionMs
        if (!isPlaying || !state.synced || state.activeLine < 0) return@LaunchedEffect
        val startFrameNanos = withFrameNanos { it }
        while (true) {
            val nowNanos = withFrameNanos { it }
            interpolatedMs = basePositionMs + (nowNanos - startFrameNanos) / 1_000_000
        }
    }

    val dominantColor = rememberDominantColor(coverUrl)
    val glowColor by animateColorAsState(dominantColor ?: Color.Transparent, label = "lyricsGlow")

    FirmiumBottomSheet(onDismiss = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .fillMaxHeight(0.9f)
                .background(
                    Brush.radialGradient(
                        colors = listOf(glowColor.copy(alpha = 0.35f), Color.Transparent),
                        radius = 900f,
                    )
                ),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = trackTitle,
                fontSize = 15.sp,
                fontFamily = FontFamily.Monospace,
                color = colors.text,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
                textAlign = TextAlign.Center,
            )

            FirmiumDivider(
                color = colors.border,
                modifier = Modifier.padding(horizontal = 24.dp),
            )

            when {
                state.isLoading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    FirmiumSpinner(color = colors.accent, modifier = Modifier.size(24.dp))
                }
                state.lines.isEmpty() -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        "No lyrics available",
                        fontSize = 14.sp,
                        fontFamily = FontFamily.Monospace,
                        fontStyle = FontStyle.Italic,
                        color = colors.muted,
                    )
                }
                else -> LazyColumn(
                    state = listState,
                    contentPadding = PaddingValues(horizontal = 24.dp, vertical = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(if (state.synced) 12.dp else 4.dp),
                    modifier = Modifier.fillMaxSize(),
                ) {
                    itemsIndexed(state.lines, key = { i, _ -> i }) { index, line ->
                        val isActive = state.synced && index == state.activeLine
                        val isNear = state.synced && (index == state.activeLine - 1 || index == state.activeLine + 1)
                        val blurModifier = if (state.synced && !isActive) Modifier.blur(1.5.dp) else Modifier

                        val words = state.wordTimings.getOrNull(index)
                        if (isActive && wordFillEnabled && !words.isNullOrEmpty()) {
                            val annotated = buildWordFillText(words, interpolatedMs, colors.muted, colors.accent)
                            BasicText(
                                text = annotated,
                                style = TextStyle(
                                    fontSize = 22.sp,
                                    fontFamily = FontFamily.Monospace,
                                    fontWeight = FontWeight.Bold,
                                    textAlign = TextAlign.Center,
                                ),
                                modifier = Modifier.fillMaxWidth().then(blurModifier),
                            )
                        } else {
                            Text(
                                text = line.text.ifBlank { "​" }, // zero-width space preserves blank line height
                                fontSize = if (isActive) 22.sp else 14.sp,
                                fontWeight = if (isActive) FontWeight.Bold else FontWeight.Normal,
                                fontFamily = FontFamily.Monospace,
                                color = when {
                                    isActive -> colors.accent
                                    isNear -> colors.text.copy(alpha = 0.7f)
                                    else -> colors.muted.copy(alpha = 0.7f)
                                },
                                textAlign = TextAlign.Center,
                                modifier = Modifier.fillMaxWidth().then(blurModifier),
                            )
                        }
                    }
                }
            }
        }
    }
}

// Builds an AnnotatedString for the active line where each word's color is interpolated
// between `mutedColor` and `accentColor` based on its estimated karaoke-fill progress.
private fun buildWordFillText(
    words: List<com.fossisawesome.firmium.viewmodel.WordTiming>,
    nowMs: Long,
    mutedColor: Color,
    accentColor: Color,
): AnnotatedString = buildAnnotatedString {
    words.forEachIndexed { i, word ->
        val span = (word.endMs - word.startMs).coerceAtLeast(1)
        val progress = ((nowMs - word.startMs).toFloat() / span).coerceIn(0f, 1f)
        withStyle(SpanStyle(color = lerp(mutedColor, accentColor, progress))) {
            append(word.text)
        }
        if (i < words.size - 1) append(" ")
    }
}
