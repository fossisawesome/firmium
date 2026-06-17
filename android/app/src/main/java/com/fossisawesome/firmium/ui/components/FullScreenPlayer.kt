package com.fossisawesome.firmium.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.basicMarquee
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.material.icons.automirrored.filled.VolumeDown
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.audio.PlayerState
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.PlaylistListItem
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.roundToInt

// Full-screen now-playing overlay — exact port of MobilePlayer.svelte.
// Dynamic art gradient, monospace font, custom seek bar, swipe/tap gestures on art.
@Composable
fun FullScreenPlayer(
    state: PlayerState,
    coverUrl: String?,
    audioSessionId: Int = 0,
    playlistItems: List<PlaylistListItem>,
    onDismiss: () -> Unit,
    onPlayPause: () -> Unit,
    onNext: () -> Unit,
    onPrevious: () -> Unit,
    onSeek: (Float) -> Unit,
    onSeekStart: () -> Unit,
    onSeekEnd: () -> Unit,
    onVolumeChange: (Float) -> Unit,
    onRepeatCycle: () -> Unit,
    onShuffleToggle: () -> Unit,
    onQueueOpen: () -> Unit,
    onLyricsOpen: () -> Unit,
    onSimilarTracksOpen: (() -> Unit)? = null,
    onAddToPlaylist: (item: PlaylistListItem) -> Unit,
    onCreatePlaylistAndAdd: (name: String) -> Unit,
) {
    val track = state.currentTrack ?: return
    val colors = LocalFirmiumColors.current
    val configuration = LocalConfiguration.current

    // Extract dominant colour from cover art, darken to 22% (same formula as Svelte version).
    val bgColor = rememberDominantColor(coverUrl)

    val orbPalette = rememberOrbPalette(coverUrl)
    var showOrb by remember { mutableStateOf(false) }
    var showAddToPlaylist by remember { mutableStateOf(false) }
    val screenWidth = configuration.screenWidthDp.dp
    // Used to animate the player fully offscreen before removing it from composition.
    val screenHeightDp = configuration.screenHeightDp.dp
    // Landscape = wider than tall. Use a two-column layout instead of stacking vertically.
    val isLandscape = configuration.screenWidthDp > configuration.screenHeightDp
    // art size: landscape=fits the height, portrait=min(72vw, 320dp)
    val artSize: Dp = if (isLandscape)
        minOf(configuration.screenHeightDp.dp * 0.70f, 260.dp)
    else
        minOf(screenWidth * 0.72f, 320.dp)

    // Art shrinks slightly when paused — a common music player UX cue.
    val artScale by animateFloatAsState(
        targetValue = if (state.playbackState == "playing") 1f else 0.88f,
        animationSpec = spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessLow),
        label = "artScale",
    )

    val progress = if (state.trackDuration > 0)
        (state.currentPosition / state.trackDuration).toFloat().coerceIn(0f, 1f)
    else 0f

    // Vertical drag offset — user can pull the screen down by the handle bar to dismiss.
    val dragOffsetY = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()
    var isDismissing by remember { mutableStateOf(false) }

    fun animateDismiss() {
        if (isDismissing) return
        isDismissing = true
        scope.launch {
            dragOffsetY.animateTo(
                screenHeightDp.value * 3,  // slide to far below screen
                tween(durationMillis = 280),
            )
            onDismiss()
        }
    }

    BackHandler { animateDismiss() }

    Box(modifier = Modifier.fillMaxSize().background(colors.bg)
        .offset { IntOffset(0, dragOffsetY.value.roundToInt()) }) {
        // Top gradient fades from darkened art colour into the bg — matches Svelte bgGradient.
        val gradColor = bgColor
        if (gradColor != null) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(360.dp)
                    .background(Brush.verticalGradient(listOf(gradColor, Color.Transparent))),
            )
        }

        if (isLandscape) {
            // Landscape layout: art left, controls right — avoids the "tall stack" problem.
            Row(
                modifier = Modifier
                    .fillMaxSize()
                    .windowInsetsPadding(WindowInsets.systemBars)
                    .padding(top = 36.dp),
            ) {
                // Left column: album art centred vertically
                Box(
                    modifier = Modifier
                        .fillMaxHeight()
                        .weight(0.42f)
                        .padding(start = 20.dp, end = 12.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    ArtOrOrb(
                        showOrb = showOrb,
                        onToggle = { showOrb = !showOrb },
                        coverUrl = coverUrl,
                        albumDescription = track.album,
                        audioSessionId = audioSessionId,
                        palette = orbPalette,
                        isPlaying = state.playbackState == "playing",
                        onTap = onLyricsOpen,
                        onSwipeLeft = onNext,
                        onSwipeRight = onPrevious,
                        modifier = Modifier
                            .size(artSize)
                            .scale(artScale)
                            .then(if (!showOrb) Modifier.shadow(elevation = 24.dp, shape = RoundedCornerShape(16.dp)) else Modifier)
                            .clip(RoundedCornerShape(16.dp)),
                    )
                }

                // Right column: track info + seek + controls + volume, scrollable.
                // Capped so controls don't stretch absurdly wide on ultra-wide screens.
                Column(
                    modifier = Modifier
                        .fillMaxHeight()
                        .weight(0.58f)
                        .verticalScroll(rememberScrollState())
                        .padding(horizontal = 20.dp)
                        .widthIn(max = 480.dp),
                    verticalArrangement = Arrangement.Center,
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    PlayerControls(
                        track = track, state = state, progress = progress,
                        onSeekStart = onSeekStart, onSeek = onSeek, onSeekEnd = onSeekEnd,
                        onPrevious = onPrevious, onPlayPause = onPlayPause, onNext = onNext,
                        onShuffleToggle = onShuffleToggle, onRepeatCycle = onRepeatCycle,
                        onAddToPlaylist = { showAddToPlaylist = true }, onQueueOpen = onQueueOpen,
                        onSimilarTracksOpen = onSimilarTracksOpen,
                        onVolumeChange = onVolumeChange, compact = true,
                    )
                }
            }
        } else {
            // Portrait layout: standard vertical stack.
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 28.dp)
                    .windowInsetsPadding(WindowInsets.systemBars),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Spacer(Modifier.height(40.dp))

                // Album art or orb — tap the toggle icon to switch modes.
                ArtOrOrb(
                    showOrb = showOrb,
                    onToggle = { showOrb = !showOrb },
                    coverUrl = coverUrl,
                    albumDescription = track.album,
                    audioSessionId = audioSessionId,
                    palette = orbPalette,
                    isPlaying = state.playbackState == "playing",
                    onTap = onLyricsOpen,
                    onSwipeLeft = onNext,
                    onSwipeRight = onPrevious,
                    modifier = Modifier
                        .size(artSize)
                        .scale(artScale)
                        .then(if (!showOrb) Modifier.shadow(elevation = 24.dp, shape = RoundedCornerShape(20.dp)) else Modifier)
                        .clip(RoundedCornerShape(20.dp)),
                )

                Spacer(Modifier.height(32.dp))

                PlayerControls(
                    track = track, state = state, progress = progress,
                    onSeekStart = onSeekStart, onSeek = onSeek, onSeekEnd = onSeekEnd,
                    onPrevious = onPrevious, onPlayPause = onPlayPause, onNext = onNext,
                    onShuffleToggle = onShuffleToggle, onRepeatCycle = onRepeatCycle,
                    onAddToPlaylist = { showAddToPlaylist = true }, onQueueOpen = onQueueOpen,
                    onSimilarTracksOpen = onSimilarTracksOpen,
                    onVolumeChange = onVolumeChange, compact = false,
                )

                Spacer(Modifier.height(8.dp))
            }
        }

        // Drag handle drawn last so it sits above the scrollable content in z-order.
        // This ensures its pointerInput wins over the scroll gesture when swiping down from the top.
        Box(
            modifier = Modifier
                .align(Alignment.TopCenter)
                .fillMaxWidth()
                .height(56.dp)
                .windowInsetsPadding(WindowInsets.statusBars)
                .pointerInput(Unit) {
                    awaitEachGesture {
                        val down = awaitFirstDown(requireUnconsumed = false)
                        val startY = down.position.y
                        val startOffset = dragOffsetY.value
                        while (true) {
                            val event = awaitPointerEvent()
                            val change = event.changes.firstOrNull() ?: break
                            val dy = change.position.y - startY
                            if (!change.pressed) {
                                if (dragOffsetY.value > 160f) animateDismiss()
                                else scope.launch {
                                    dragOffsetY.animateTo(0f, spring(
                                        dampingRatio = Spring.DampingRatioMediumBouncy,
                                        stiffness = Spring.StiffnessMedium,
                                    ))
                                }
                                break
                            }
                            scope.launch { dragOffsetY.snapTo((startOffset + dy).coerceAtLeast(0f)) }
                            change.consume()
                        }
                    }
                },
            contentAlignment = Alignment.TopCenter,
        ) {
            Box(
                modifier = Modifier
                    .padding(top = 14.dp)
                    .width(36.dp).height(4.dp)
                    .clip(RoundedCornerShape(2.dp))
                    .background(colors.surface2),
            )
        }
    }

    if (showAddToPlaylist) {
        AddToPlaylistDialog(
            items = playlistItems,
            onAddTo = { item -> onAddToPlaylist(item) },
            onCreateAndAdd = { name -> onCreatePlaylistAndAdd(name) },
            onDismiss = { showAddToPlaylist = false },
        )
    }
}

// Shared controls section used by both portrait and landscape layouts.
// compact=true tightens vertical spacing for landscape where height is limited.
@Composable
private fun PlayerControls(
    track: Song,
    state: PlayerState,
    progress: Float,
    onSeekStart: () -> Unit,
    onSeek: (Float) -> Unit,
    onSeekEnd: () -> Unit,
    onPrevious: () -> Unit,
    onPlayPause: () -> Unit,
    onNext: () -> Unit,
    onShuffleToggle: () -> Unit,
    onRepeatCycle: () -> Unit,
    onAddToPlaylist: () -> Unit,
    onQueueOpen: () -> Unit,
    onSimilarTracksOpen: (() -> Unit)? = null,
    onVolumeChange: (Float) -> Unit,
    compact: Boolean,
) {
    val colors = LocalFirmiumColors.current
    val gap: Dp = if (compact) 12.dp else 28.dp

    // Track info — title (20sp bold) + artist (14sp muted).
    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = track.title,
            fontFamily = FontFamily.Monospace,
            fontSize = if (compact) 16.sp else 20.sp,
            fontWeight = FontWeight.Bold,
            color = colors.text,
            textAlign = TextAlign.Center,
            maxLines = 1,
            modifier = Modifier.basicMarquee(),
        )
        Spacer(Modifier.height(6.dp))
        Text(
            text = track.displayArtist ?: track.artist,
            fontFamily = FontFamily.Monospace,
            fontSize = if (compact) 12.sp else 14.sp,
            color = colors.muted,
            textAlign = TextAlign.Center,
            maxLines = 1,
            modifier = Modifier.basicMarquee(),
        )
        val trackInfo = track.formatTrackInfo()
        if (trackInfo.isNotEmpty()) {
            Spacer(Modifier.height(4.dp))
            Text(
                text = trackInfo,
                fontFamily = FontFamily.Monospace,
                fontSize = if (compact) 10.sp else 11.sp,
                color = colors.muted,
                textAlign = TextAlign.Center,
                maxLines = 1,
            )
        }
    }

    Spacer(Modifier.height(gap))

    // Seek bar + elapsed/total time labels.
    FirmiumSeekBar(
        progress = progress,
        onSeekStart = onSeekStart,
        onSeekUpdate = onSeek,
        onSeekEnd = onSeekEnd,
        trackColor = colors.surface2,
        fillColor = colors.accent,
    )
    Spacer(Modifier.height(6.dp))
    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(formatSeconds(state.currentPosition), fontSize = 11.sp, color = colors.muted, fontFamily = FontFamily.Monospace)
        Text(formatSeconds(state.trackDuration), fontSize = 11.sp, color = colors.muted, fontFamily = FontFamily.Monospace)
    }

    Spacer(Modifier.height(gap))

    // Primary controls: prev / play / next.
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val btnSize = if (compact) 52.dp else 60.dp
        val playSize = if (compact) 62.dp else 72.dp
        FirmiumCircleButton(size = btnSize, onClick = onPrevious, enabled = state.hasPrev) {
            FirmiumIcon(Icons.Default.SkipPrevious, contentDescription = "Previous",
                tint = if (state.hasPrev) colors.text else colors.muted, modifier = Modifier.size(28.dp))
        }
        Box(
            modifier = Modifier
                .size(playSize).clip(CircleShape).background(colors.accent)
                .pointerInput(Unit) {
                    awaitEachGesture {
                        awaitFirstDown()
                        val up = waitForUpOrCancellation()
                        if (up != null) onPlayPause()
                    }
                },
            contentAlignment = Alignment.Center,
        ) {
            FirmiumIcon(
                imageVector = when (state.playbackState) {
                    "playing" -> Icons.Default.Pause
                    "loading" -> Icons.Default.HourglassEmpty
                    else -> Icons.Default.PlayArrow
                },
                contentDescription = "Play/Pause",
                tint = colors.bg,
                modifier = Modifier.size(28.dp),
            )
        }
        FirmiumCircleButton(size = btnSize, onClick = onNext, enabled = state.hasNext) {
            FirmiumIcon(Icons.Default.SkipNext, contentDescription = "Next",
                tint = if (state.hasNext) colors.text else colors.muted, modifier = Modifier.size(28.dp))
        }
    }

    Spacer(Modifier.height(if (compact) 12.dp else 20.dp))

    // Secondary controls: shuffle / repeat / add / queue.
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(24.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val secSize = if (compact) 42.dp else 48.dp
        FirmiumCircleButton(size = secSize, onClick = onShuffleToggle) {
            FirmiumIcon(Icons.Default.Shuffle, contentDescription = "Shuffle",
                tint = if (state.shuffleEnabled) colors.accent else colors.muted, modifier = Modifier.size(22.dp))
        }
        Box(contentAlignment = Alignment.TopEnd) {
            FirmiumCircleButton(size = secSize, onClick = onRepeatCycle) {
                FirmiumIcon(
                    imageVector = if (state.repeatMode == "one") Icons.Default.RepeatOne else Icons.Default.Repeat,
                    contentDescription = "Repeat",
                    tint = if (state.repeatMode != "none") colors.accent else colors.muted,
                    modifier = Modifier.size(22.dp),
                )
            }
            if (state.repeatMode == "one") {
                Text("1", fontSize = 10.sp, color = colors.accent, fontWeight = FontWeight.Bold,
                    modifier = Modifier.offset(x = (-4).dp, y = 4.dp))
            }
        }
        FirmiumCircleButton(size = secSize, onClick = onAddToPlaylist) {
            FirmiumIcon(Icons.Default.PlaylistAdd, contentDescription = "Add to playlist",
                tint = colors.muted, modifier = Modifier.size(22.dp))
        }
        FirmiumCircleButton(size = secSize, onClick = onQueueOpen) {
            FirmiumIcon(Icons.AutoMirrored.Filled.QueueMusic, contentDescription = "Queue",
                tint = colors.muted, modifier = Modifier.size(22.dp))
        }
        if (onSimilarTracksOpen != null) {
            FirmiumCircleButton(size = secSize, onClick = onSimilarTracksOpen) {
                FirmiumIcon(Icons.Default.Hub, contentDescription = "Similar Tracks",
                    tint = colors.muted, modifier = Modifier.size(22.dp))
            }
        }
    }

    Spacer(Modifier.height(if (compact) 16.dp else 32.dp))

    // Volume slider.
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        FirmiumIcon(Icons.AutoMirrored.Filled.VolumeDown, contentDescription = null,
            tint = colors.muted, modifier = Modifier.size(16.dp))
        FirmiumSlider(
            value = state.volume,
            onValueChange = onVolumeChange,
            modifier = Modifier.weight(1f),
            trackColor = colors.surface2,
            fillColor = colors.accent,
        )
        FirmiumIcon(Icons.AutoMirrored.Filled.VolumeUp, contentDescription = null,
            tint = colors.muted, modifier = Modifier.size(16.dp))
    }
}

// Switches between album art and the NCS orb visualizer.
// The toggle icon sits in the top-right corner of the art box.
@Composable
private fun ArtOrOrb(
    showOrb: Boolean,
    onToggle: () -> Unit,
    coverUrl: String?,
    albumDescription: String,
    audioSessionId: Int,
    palette: OrbPalette,
    isPlaying: Boolean,
    onTap: () -> Unit,
    onSwipeLeft: () -> Unit,
    onSwipeRight: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(modifier = modifier) {
        if (showOrb) {
            MusicOrb(
                audioSessionId = audioSessionId,
                palette = palette,
                isPlaying = isPlaying,
                modifier = Modifier.fillMaxSize(),
            )
        } else {
            ArtWithGestures(
                coverUrl = coverUrl,
                albumDescription = albumDescription,
                onTap = onTap,
                onSwipeLeft = onSwipeLeft,
                onSwipeRight = onSwipeRight,
                modifier = Modifier.fillMaxSize(),
            )
        }
        FirmiumIconButton(
            onClick = onToggle,
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(6.dp)
                .size(32.dp),
        ) {
            FirmiumIcon(
                imageVector = if (showOrb) Icons.Default.Image else Icons.Default.Equalizer,
                contentDescription = if (showOrb) "Show art" else "Show visualizer",
                tint = Color.White.copy(alpha = 0.75f),
                modifier = Modifier.size(18.dp),
            )
        }
    }
}

// Art box with tap-to-lyrics and horizontal swipe for prev/next.
@Composable
private fun ArtWithGestures(
    coverUrl: String?,
    albumDescription: String,
    onTap: () -> Unit,
    onSwipeLeft: () -> Unit,
    onSwipeRight: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier.pointerInput(Unit) {
            awaitEachGesture {
                val down = awaitFirstDown()
                val startX = down.position.x
                val startY = down.position.y
                var moved = false
                while (true) {
                    val event = awaitPointerEvent()
                    val change = event.changes.firstOrNull() ?: break
                    val totalDx = change.position.x - startX
                    val totalDy = change.position.y - startY
                    if (abs(totalDx) > 8f || abs(totalDy) > 8f) moved = true
                    if (!change.pressed) {
                        val threshold = 40.dp.toPx()
                        if (!moved) onTap()
                        else if (abs(totalDx) > threshold && abs(totalDx) > abs(totalDy) * 1.5f) {
                            if (totalDx < 0) onSwipeLeft() else onSwipeRight()
                        }
                        break
                    }
                    change.consume()
                }
            }
        },
    ) {
        CoverImage(url = coverUrl, contentDescription = albumDescription, modifier = Modifier.fillMaxSize())
    }
}

// Invisible circle button — sized clickable Box.
// Fires onClick on pointer UP (not down) to prevent accidental track skips.
@Composable
private fun FirmiumCircleButton(
    size: Dp,
    onClick: () -> Unit,
    enabled: Boolean = true,
    content: @Composable BoxScope.() -> Unit,
) {
    val haptic = LocalHapticFeedback.current
    Box(
        modifier = Modifier
            .size(size)
            .clip(CircleShape)
            .pointerInput(enabled) {
                if (!enabled) return@pointerInput
                awaitEachGesture {
                    awaitFirstDown()
                    val up = waitForUpOrCancellation()
                    if (up != null) {
                        haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                        onClick()
                    }
                }
            },
        contentAlignment = Alignment.Center,
        content = content,
    )
}

private fun formatSeconds(seconds: Double): String {
    val s = seconds.toInt()
    return "%d:%02d".format(s / 60, s % 60)
}
