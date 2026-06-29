package com.fossisawesome.firmium.ui.components
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.basicMarquee
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.automirrored.filled.QueueMusic
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
import com.fossisawesome.firmium.viewmodel.LyricsState
import com.fossisawesome.firmium.viewmodel.PlaylistListItem
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.roundToInt

// Full-screen now-playing overlay. Tap art for centered lyrics (X to exit), long-press art for the
// star-rating popup, and the 3-dot button opens a grid "more" menu.
@Composable
fun FullScreenPlayer(
    state: PlayerState,
    coverUrl: String?,
    audioSessionId: Int = 0,
    playlistItems: List<PlaylistListItem>,
    lyricsState: LyricsState,
    wordFillEnabled: Boolean,
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
    onLyricsClose: () -> Unit,
    onSimilarTracksOpen: (() -> Unit)? = null,
    onAddToPlaylist: (item: PlaylistListItem) -> Unit,
    onCreatePlaylistAndAdd: (name: String) -> Unit,
    onStartRadio: (() -> Unit)? = null,
    onRate: ((songId: String, rating: Int) -> Unit)? = null,
    onAddToQueue: () -> Unit,
    onViewArtist: () -> Unit,
    onEqualizer: () -> Unit,
    onDownloadTrack: (() -> Unit)? = null,
) {
    val track = state.currentTrack ?: return
    val colors = LocalFirmiumColors.current
    val configuration = LocalConfiguration.current

    // Extract dominant colour from cover art, darken to 22% (same formula as Svelte version).
    val bgColor = rememberDominantColor(coverUrl)

    val orbPalette = rememberOrbPalette(coverUrl)
    var showOrb by remember { mutableStateOf(false) }
    // In-player visualizer type, seeded from settings; tapping the visualizer cycles it live.
    var vizType by remember(state.visualizerType) { mutableStateOf(VisualizerType.fromId(state.visualizerType)) }
    val cycleViz = { vizType = VisualizerType.entries[(vizType.ordinal + 1) % VisualizerType.entries.size] }
    var showAddToPlaylist by remember { mutableStateOf(false) }
    var showMore by remember { mutableStateOf(false) }
    // Lyrics replace the album art in the center; stars pop up over the art on long-press.
    var showLyrics by remember { mutableStateOf(false) }
    var showStars by remember { mutableStateOf(false) }
    var statsExpanded by remember { mutableStateOf(false) }
    // Reset overlays on track change.
    LaunchedEffect(track.id) { showLyrics = false; showStars = false; statsExpanded = false }

    val openLyrics = { showLyrics = true; onLyricsOpen() }
    val closeLyrics = { showLyrics = false; onLyricsClose() }

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

    // Back closes lyrics first, then dismisses the player.
    BackHandler { if (showLyrics) closeLyrics() else animateDismiss() }

    val artBlock: @Composable (Modifier) -> Unit = { artMod ->
        ArtOrLyrics(
            artModifier = artMod,
            showLyrics = showLyrics,
            onCloseLyrics = closeLyrics,
            lyricsState = lyricsState,
            positionSeconds = state.currentPosition,
            isPlaying = state.playbackState == "playing",
            wordFillEnabled = wordFillEnabled,
            showOrb = showOrb,
            onToggleOrb = { showOrb = !showOrb },
            visualizerEnabled = state.visualizerEnabled,
            vizType = vizType,
            onCycleViz = cycleViz,
            coverUrl = coverUrl,
            track = track,
            audioSessionId = audioSessionId,
            orbPalette = orbPalette,
            isPlayingArt = state.playbackState == "playing",
            onTapArt = openLyrics,
            onSwipeLeft = onNext,
            onSwipeRight = onPrevious,
            onLongPressArt = { if (onRate != null) showStars = true },
            showStars = showStars,
            onDismissStars = { showStars = false },
            rating = track.userRating ?: 0,
            onRate = onRate?.let { rate -> { r: Int -> rate(track.id, if (r == track.userRating) 0 else r) } },
        )
    }

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
            Row(
                modifier = Modifier
                    .fillMaxSize()
                    .windowInsetsPadding(WindowInsets.systemBars)
                    .padding(top = 36.dp),
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxHeight()
                        .weight(0.42f)
                        .padding(start = 20.dp, end = 12.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    artBlock(Modifier.size(artSize).scale(artScale))
                }

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
                        onQueueOpen = onQueueOpen,
                        onSimilarTracksOpen = onSimilarTracksOpen,
                        onMoreOpen = { showMore = true },
                        statsExpanded = statsExpanded,
                        onToggleStats = { statsExpanded = !statsExpanded },
                        compact = true,
                    )
                }
            }
        } else {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 28.dp)
                    .windowInsetsPadding(WindowInsets.systemBars),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Spacer(Modifier.height(40.dp))

                artBlock(Modifier.size(artSize).scale(artScale))

                Spacer(Modifier.height(32.dp))

                PlayerControls(
                    track = track, state = state, progress = progress,
                    onSeekStart = onSeekStart, onSeek = onSeek, onSeekEnd = onSeekEnd,
                    onPrevious = onPrevious, onPlayPause = onPlayPause, onNext = onNext,
                    onShuffleToggle = onShuffleToggle, onRepeatCycle = onRepeatCycle,
                    onQueueOpen = onQueueOpen,
                    onSimilarTracksOpen = onSimilarTracksOpen,
                    onMoreOpen = { showMore = true },
                    statsExpanded = statsExpanded,
                    onToggleStats = { statsExpanded = !statsExpanded },
                    compact = false,
                )

                Spacer(Modifier.height(8.dp))
            }
        }

        // Drag handle drawn last so it sits above the scrollable content in z-order.
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
            onStartRadio = onStartRadio,
        )
    }

    if (showMore) {
        PlayerMoreSheet(
            volume = state.volume,
            onVolumeChange = onVolumeChange,
            onAddToPlaylist = { showAddToPlaylist = true },
            onViewArtist = onViewArtist,
            onAddToQueue = onAddToQueue,
            onTrackInfo = { statsExpanded = true },
            onEqualizer = onEqualizer,
            onDownload = { onDownloadTrack?.invoke() },
            onToggleVisualizer = if (state.visualizerEnabled) ({ showOrb = !showOrb }) else null,
            onDismiss = { showMore = false },
        )
    }
}

// Shared controls section used by both portrait and landscape layouts.
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
    onQueueOpen: () -> Unit,
    onSimilarTracksOpen: (() -> Unit)? = null,
    onMoreOpen: () -> Unit,
    statsExpanded: Boolean,
    onToggleStats: () -> Unit,
    compact: Boolean,
) {
    val colors = LocalFirmiumColors.current
    val gap: Dp = if (compact) 12.dp else 28.dp

    // Track info — title + artist.
    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = track.title,
            fontFamily = LocalAppFontFamily.current,
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
            fontFamily = LocalAppFontFamily.current,
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
                text = trackInfo + "  ▾",
                fontFamily = LocalAppFontFamily.current,
                fontSize = if (compact) 10.sp else 11.sp,
                color = colors.muted,
                textAlign = TextAlign.Center,
                maxLines = 1,
                modifier = Modifier.clickable { onToggleStats() },
            )
        }
        if (statsExpanded) {
            Spacer(Modifier.height(8.dp))
            AudioStats(track = track, compact = compact)
        }
    }

    Spacer(Modifier.height(gap))

    // Seek bar + elapsed/total time labels + 3-dot "more" button.
    FirmiumSeekBar(
        progress = progress,
        onSeekStart = onSeekStart,
        onSeekUpdate = onSeek,
        onSeekEnd = onSeekEnd,
        trackColor = colors.surface2,
        fillColor = colors.accent,
    )
    Spacer(Modifier.height(6.dp))
    Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(formatSeconds(state.currentPosition), fontSize = 11.sp, color = colors.muted, fontFamily = LocalAppFontFamily.current)
        Spacer(Modifier.weight(1f))
        Text(formatSeconds(state.trackDuration), fontSize = 11.sp, color = colors.muted, fontFamily = LocalAppFontFamily.current)
        FirmiumIconButton(onClick = onMoreOpen, modifier = Modifier.size(44.dp)) {
            FirmiumIcon(Icons.Default.MoreVert, contentDescription = "More",
                tint = colors.muted, modifier = Modifier.size(20.dp))
        }
    }

    Spacer(Modifier.height(gap))

    val secSize = if (compact) 44.dp else 48.dp

    // Primary controls: shuffle / prev / play / next / repeat.
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(if (compact) 8.dp else 12.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val btnSize = if (compact) 52.dp else 60.dp
        val playSize = if (compact) 62.dp else 72.dp
        FirmiumCircleButton(size = secSize, onClick = onShuffleToggle) {
            FirmiumIcon(Icons.Default.Shuffle, contentDescription = "Shuffle",
                tint = if (state.shuffleEnabled) colors.accent else colors.muted, modifier = Modifier.size(22.dp))
        }
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
    }

    Spacer(Modifier.height(if (compact) 12.dp else 20.dp))

    // Secondary controls: queue / similar.
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(24.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
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

    Spacer(Modifier.height(if (compact) 16.dp else 24.dp))
}

// Expandable audio stats: BPM and ReplayGain track/album gain + peak (display only).
@Composable
private fun AudioStats(track: Song, compact: Boolean) {
    val colors = LocalFirmiumColors.current
    val size = if (compact) 10.sp else 11.sp

    fun db(v: Double?): String = if (v == null) "—" else "%+.2f dB".format(v)
    fun peak(v: Double?): String = if (v == null) "—" else "%.4f".format(v)

    val rows = buildList {
        track.bpm?.let { add("Tempo" to "$it BPM") }
        if (track.replayGainTrack != null || track.replayGainTrackPeak != null) {
            add("Track gain" to db(track.replayGainTrack))
            add("Track peak" to peak(track.replayGainTrackPeak))
        }
        if (track.replayGainAlbum != null || track.replayGainAlbumPeak != null) {
            add("Album gain" to db(track.replayGainAlbum))
            add("Album peak" to peak(track.replayGainAlbumPeak))
        }
    }

    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
        verticalArrangement = Arrangement.spacedBy(3.dp),
    ) {
        if (rows.isEmpty()) {
            Text("No extra stats provided by server", fontFamily = LocalAppFontFamily.current,
                fontSize = size, color = colors.muted, textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth())
        } else {
            rows.forEach { (label, value) ->
                Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(label, fontFamily = LocalAppFontFamily.current, fontSize = size, color = colors.muted)
                    Text(value, fontFamily = LocalAppFontFamily.current, fontSize = size, color = colors.text)
                }
            }
        }
    }
}

// Center area of the player: either the album art (with orb/visualizer toggle and a long-press
// star popup) or, when toggled, the lyrics in place of the art with an X to return.
@Composable
private fun ArtOrLyrics(
    artModifier: Modifier,
    showLyrics: Boolean,
    onCloseLyrics: () -> Unit,
    lyricsState: LyricsState,
    positionSeconds: Double,
    isPlaying: Boolean,
    wordFillEnabled: Boolean,
    showOrb: Boolean,
    onToggleOrb: () -> Unit,
    visualizerEnabled: Boolean,
    vizType: VisualizerType,
    onCycleViz: () -> Unit,
    coverUrl: String?,
    track: Song,
    audioSessionId: Int,
    orbPalette: OrbPalette,
    isPlayingArt: Boolean,
    onTapArt: () -> Unit,
    onSwipeLeft: () -> Unit,
    onSwipeRight: () -> Unit,
    onLongPressArt: () -> Unit,
    showStars: Boolean,
    onDismissStars: () -> Unit,
    rating: Int,
    onRate: ((Int) -> Unit)?,
) {
    val colors = LocalFirmiumColors.current
    Box(modifier = artModifier.clip(RoundedCornerShape(20.dp))) {
        if (showLyrics) {
            LyricsLines(
                state = lyricsState,
                positionSeconds = positionSeconds,
                isPlaying = isPlaying,
                wordFillEnabled = wordFillEnabled,
                modifier = Modifier.fillMaxSize(),
                activeFontSize = 18.sp,
                inactiveFontSize = 13.sp,
            )
            // X to leave lyrics and return to the album art.
            Box(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(6.dp)
                    .size(32.dp)
                    .clip(CircleShape)
                    .background(colors.surface2)
                    .clickable { onCloseLyrics() },
                contentAlignment = Alignment.Center,
            ) {
                FirmiumIcon(Icons.Default.Close, contentDescription = "Close lyrics",
                    tint = colors.text, modifier = Modifier.size(18.dp))
            }
        } else {
            ArtOrOrb(
                showOrb = showOrb,
                onToggle = onToggleOrb,
                visualizerEnabled = visualizerEnabled,
                vizType = vizType,
                onCycleType = onCycleViz,
                coverUrl = coverUrl,
                albumDescription = track.album,
                audioSessionId = audioSessionId,
                palette = orbPalette,
                isPlaying = isPlayingArt,
                onTap = onTapArt,
                onSwipeLeft = onSwipeLeft,
                onSwipeRight = onSwipeRight,
                onLongPress = onLongPressArt,
                modifier = Modifier.fillMaxSize()
                    .then(if (!showOrb) Modifier.shadow(elevation = 24.dp, shape = RoundedCornerShape(20.dp)) else Modifier),
            )

            // Star-rating popup, animated in over the art on long-press.
            if (onRate != null) {
                AnimatedVisibility(
                    visible = showStars,
                    enter = scaleIn(initialScale = 0.7f) + fadeIn(),
                    exit = scaleOut(targetScale = 0.7f) + fadeOut(),
                    modifier = Modifier.align(Alignment.BottomCenter).padding(bottom = 16.dp),
                ) {
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(999.dp))
                            .background(colors.surface2.copy(alpha = 0.95f))
                            .padding(horizontal = 16.dp, vertical = 10.dp),
                    ) {
                        StarRating(
                            rating = rating,
                            onRate = { r -> onRate(r); onDismissStars() },
                            starSize = 26.dp,
                            accentColor = colors.accent,
                            mutedColor = colors.muted,
                        )
                    }
                }
                // Tap anywhere over the art (outside the stars) dismisses the popup.
                if (showStars) {
                    Box(
                        modifier = Modifier
                            .matchParentSize()
                            .clickable(
                                interactionSource = remember { MutableInteractionSource() },
                                indication = null,
                                onClick = onDismissStars,
                            ),
                    )
                }
            }
        }
    }
}

// Switches between album art and the NCS orb visualizer.
@Composable
private fun ArtOrOrb(
    showOrb: Boolean,
    onToggle: () -> Unit,
    visualizerEnabled: Boolean,
    vizType: VisualizerType,
    onCycleType: () -> Unit,
    coverUrl: String?,
    albumDescription: String,
    audioSessionId: Int,
    palette: OrbPalette,
    isPlaying: Boolean,
    onTap: () -> Unit,
    onSwipeLeft: () -> Unit,
    onSwipeRight: () -> Unit,
    onLongPress: () -> Unit,
    modifier: Modifier = Modifier,
) {
    // The visualizer is only available when enabled in Settings.
    val showViz = showOrb && visualizerEnabled
    Box(modifier = modifier) {
        if (showViz) {
            Box(
                modifier = Modifier.fillMaxSize().pointerInput(Unit) {
                    awaitEachGesture {
                        awaitFirstDown()
                        val up = waitForUpOrCancellation()
                        if (up != null) onCycleType()  // tap visualizer = next type
                    }
                },
            ) {
                VisualizerView(
                    type = vizType,
                    audioSessionId = audioSessionId,
                    palette = palette,
                    isPlaying = isPlaying,
                    modifier = Modifier.fillMaxSize(),
                )
                Text(
                    vizType.label,
                    fontSize = 10.sp,
                    fontFamily = LocalAppFontFamily.current,
                    color = Color.White.copy(alpha = 0.65f),
                    modifier = Modifier.align(Alignment.BottomCenter).padding(bottom = 8.dp),
                )
            }
        } else {
            ArtWithGestures(
                coverUrl = coverUrl,
                albumDescription = albumDescription,
                onTap = onTap,
                onSwipeLeft = onSwipeLeft,
                onSwipeRight = onSwipeRight,
                onLongPress = onLongPress,
                modifier = Modifier.fillMaxSize(),
            )
        }
        if (visualizerEnabled) {
            FirmiumIconButton(
                onClick = onToggle,
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(6.dp)
                    .size(32.dp),
            ) {
                FirmiumIcon(
                    imageVector = if (showViz) Icons.Default.Image else Icons.Default.Equalizer,
                    contentDescription = if (showViz) "Show art" else "Show visualizer",
                    tint = Color.White.copy(alpha = 0.75f),
                    modifier = Modifier.size(18.dp),
                )
            }
        }
    }
}

// Art box with tap-to-lyrics, long-press-for-stars, and horizontal swipe for prev/next.
@Composable
private fun ArtWithGestures(
    coverUrl: String?,
    albumDescription: String,
    onTap: () -> Unit,
    onSwipeLeft: () -> Unit,
    onSwipeRight: () -> Unit,
    onLongPress: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .pointerInput(Unit) {
                detectTapGestures(onTap = { onTap() }, onLongPress = { onLongPress() })
            }
            .pointerInput(Unit) {
                var total = 0f
                detectHorizontalDragGestures(
                    onDragStart = { total = 0f },
                    onDragEnd = {
                        val threshold = 40.dp.toPx()
                        if (abs(total) > threshold) {
                            if (total < 0) onSwipeLeft() else onSwipeRight()
                        }
                    },
                    onHorizontalDrag = { _, amount -> total += amount },
                )
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
