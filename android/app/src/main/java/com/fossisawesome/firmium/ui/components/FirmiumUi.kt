package com.fossisawesome.firmium.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.QueueMusic
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.rememberVectorPainter
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

// Convenience Text composable backed by BasicText — replaces material3 Text() throughout the app.
// Only supports parameters actually used in this codebase.
@Composable
fun Text(
    text: String,
    modifier: Modifier = Modifier,
    color: Color = Color.Unspecified,
    fontSize: TextUnit = TextUnit.Unspecified,
    fontStyle: FontStyle? = null,
    fontWeight: FontWeight? = null,
    fontFamily: FontFamily? = null,
    letterSpacing: TextUnit = TextUnit.Unspecified,
    textAlign: TextAlign? = null,
    textDecoration: TextDecoration? = null,
    lineHeight: TextUnit = TextUnit.Unspecified,
    overflow: TextOverflow = TextOverflow.Clip,
    softWrap: Boolean = true,
    maxLines: Int = Int.MAX_VALUE,
    minLines: Int = 1,
) {
    BasicText(
        text = text,
        modifier = modifier,
        style = TextStyle(
            color = color,
            fontSize = fontSize,
            fontStyle = fontStyle,
            fontWeight = fontWeight,
            fontFamily = fontFamily,
            letterSpacing = letterSpacing,
            textAlign = textAlign ?: TextAlign.Unspecified,
            textDecoration = textDecoration,
            lineHeight = lineHeight,
        ),
        overflow = overflow,
        softWrap = softWrap,
        maxLines = maxLines,
        minLines = minLines,
    )
}

// Renders an ImageVector with a colour tint — replaces material3 Icon().
@Composable
fun FirmiumIcon(
    imageVector: ImageVector,
    contentDescription: String?,
    tint: Color,
    modifier: Modifier = Modifier,
) {
    Image(
        painter = rememberVectorPainter(imageVector),
        contentDescription = contentDescription,
        colorFilter = ColorFilter.tint(tint),
        modifier = modifier,
    )
}

// Shared press-feedback animation: an overlay alpha and a shrink/spring-back scale,
// driven by an interaction source's pressed state.
// internal (not private): reused by FullScreenPlayer.kt's circle transport buttons so
// press feedback stays consistent across the app.
@Composable
internal fun rememberPressAnimations(
    interactionSource: MutableInteractionSource,
    enabled: Boolean = true,
    pressedAlpha: Float,
    pressedScale: Float,
    label: String,
): Pair<Float, Float> {
    val isPressed by interactionSource.collectIsPressedAsState()
    val overlayAlpha by animateFloatAsState(
        targetValue = if (isPressed && enabled) pressedAlpha else 0f,
        animationSpec = tween(durationMillis = 80),
        label = "${label}Press",
    )
    val scale by animateFloatAsState(
        targetValue = if (isPressed && enabled) pressedScale else 1f,
        animationSpec = if (isPressed && enabled) {
            tween(durationMillis = 80, easing = LinearEasing)
        } else {
            spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessMedium)
        },
        label = "${label}Scale",
    )
    return overlayAlpha to scale
}

// Standard icon-button tap target — matches Material's ~48dp touch-target guideline.
// Use this size for all icon buttons unless a screen is legitimately too dense for it.
val FirmiumIconButtonSize = 44.dp

// Compact variant for genuinely dense screens (e.g. tightly packed rows) where 44dp
// doesn't fit. Prefer FirmiumIconButtonSize by default.
val FirmiumIconButtonCompactSize = 40.dp

// Tap-target Box that wraps icon content — replaces material3 IconButton().
// Shows a subtle press highlight using the text colour at low alpha.
@Composable
fun FirmiumIconButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier.size(FirmiumIconButtonSize),
    enabled: Boolean = true,
    content: @Composable BoxScope.() -> Unit,
) {
    val pressColor = LocalFirmiumColors.current.text
    val interactionSource = remember { MutableInteractionSource() }
    val haptic = LocalHapticFeedback.current
    val (overlayAlpha, scale) = rememberPressAnimations(
        interactionSource = interactionSource,
        enabled = enabled,
        pressedAlpha = 0.12f,
        pressedScale = 0.82f,
        label = "iconBtn",
    )
    Box(
        modifier = modifier
            .scale(scale)
            .then(
                if (enabled) Modifier.clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    onClick = {
                        haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                        onClick()
                    },
                ) else Modifier
            )
            .background(pressColor.copy(alpha = overlayAlpha), shape = CircleShape),
        contentAlignment = Alignment.Center,
        content = content,
    )
}

// 1dp horizontal rule — replaces material3 HorizontalDivider().
@Composable
fun FirmiumDivider(
    modifier: Modifier = Modifier,
    color: Color = LocalFirmiumColors.current.border,
) {
    Box(modifier = modifier.fillMaxWidth().height(1.dp).background(color))
}

// Spinning arc — replaces material3 CircularProgressIndicator().
@Composable
fun FirmiumSpinner(
    color: Color,
    modifier: Modifier = Modifier,
    strokeWidth: Dp = 2.dp,
) {
    val transition = rememberInfiniteTransition(label = "spinner")
    val angle by transition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 900, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "angle",
    )
    Canvas(modifier = modifier) {
        drawArc(
            color = color,
            startAngle = angle,
            sweepAngle = 270f,
            useCenter = false,
            style = Stroke(width = strokeWidth.toPx(), cap = StrokeCap.Round),
        )
    }
}

// Horizontal progress bar — replaces material3 LinearProgressIndicator().
// Progress is animated so the bar glides smoothly rather than jumping.
@Composable
fun FirmiumLinearProgress(
    progress: Float,
    trackColor: Color,
    fillColor: Color,
    modifier: Modifier = Modifier,
) {
    val animatedProgress by animateFloatAsState(
        targetValue = progress.coerceIn(0f, 1f),
        animationSpec = tween(durationMillis = 200),
        label = "linearProgress",
    )
    Box(modifier = modifier.background(trackColor)) {
        Box(
            modifier = Modifier
                .fillMaxWidth(animatedProgress)
                .fillMaxHeight()
                .background(fillColor),
        )
    }
}

// Minimal toggle switch — used in Settings and the login save-password row.
// Animates a thumb between off (left, muted) and on (right, accent).
@Composable
fun FirmiumToggle(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = LocalFirmiumColors.current
    val thumbX by animateFloatAsState(
        targetValue = if (checked) 1f else 0f,
        animationSpec = spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessMedium),
        label = "toggleThumb",
    )
    val trackColor by animateColorAsState(
        targetValue = if (checked) colors.accent.copy(alpha = 0.4f) else colors.border,
        animationSpec = tween(150),
        label = "toggleTrack",
    )
    val thumbColor by animateColorAsState(
        targetValue = if (checked) colors.accent else colors.muted,
        animationSpec = tween(150),
        label = "toggleThumbColor",
    )
    Box(
        modifier = modifier
            .size(width = 40.dp, height = 22.dp)
            .clip(RoundedCornerShape(11.dp))
            .background(trackColor)
            .clickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = null,
            ) { onCheckedChange(!checked) },
    ) {
        Box(
            modifier = Modifier
                .padding(2.dp)
                .size(18.dp)
                .offset(x = (thumbX * 18).dp)
                .clip(CircleShape)
                .background(thumbColor),
        )
    }
}

// Clickable text — replaces material3 TextButton().
// Shows a subtle press highlight using the text colour at low alpha.
@Composable
fun FirmiumTextButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    val pressColor = LocalFirmiumColors.current.text
    val interactionSource = remember { MutableInteractionSource() }
    val (overlayAlpha, scale) = rememberPressAnimations(
        interactionSource = interactionSource,
        pressedAlpha = 0.08f,
        pressedScale = 0.93f,
        label = "textBtn",
    )
    Box(
        modifier = modifier
            .scale(scale)
            .clickable(
                interactionSource = interactionSource,
                indication = null,
                onClick = onClick,
            )
            .background(pressColor.copy(alpha = overlayAlpha), shape = RoundedCornerShape(4.dp))
            .padding(horizontal = 8.dp, vertical = 4.dp),
        contentAlignment = Alignment.Center,
        content = { content() },
    )
}

// Swipe-right to add a song to the play queue.
// As the user drags right, the theme accent colour is revealed behind the row and a QueueMusic
// icon fades in. Releasing past 35% of the row width triggers the action; releasing earlier
// springs the row back without doing anything.
@Composable
fun SwipeToQueueBox(
    onAddToQueue: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    val colors = LocalFirmiumColors.current
    val haptic = LocalHapticFeedback.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val offsetX = remember { Animatable(0f) }
    var widthPx by remember { mutableFloatStateOf(1f) }
    val threshold = 0.35f

    val accentColor = colors.accent
    Box(modifier = modifier.onSizeChanged { widthPx = it.width.toFloat().coerceAtLeast(1f) }) {
        val revealFraction = (offsetX.value / widthPx).coerceIn(0f, 1f)
        // Canvas draws only the revealed strip. Using a nested Box with matchParentSize() +
        // fillMaxWidth(fraction) doesn't work: matchParentSize() sets min == parent width so
        // fillMaxWidth(0f) can't shrink it — the background was always full-width. Canvas draws
        // exactly what we ask.
        Canvas(modifier = Modifier.matchParentSize()) {
            drawRect(
                color = accentColor.copy(alpha = 0.35f),
                size = androidx.compose.ui.geometry.Size(size.width * revealFraction, size.height),
            )
        }
        if (revealFraction > 0.12f) {
            Box(
                modifier = Modifier.matchParentSize(),
                contentAlignment = Alignment.CenterStart,
            ) {
                FirmiumIcon(
                    Icons.AutoMirrored.Filled.QueueMusic,
                    contentDescription = null,
                    tint = colors.bg.copy(alpha = ((revealFraction - 0.12f) * 5f).coerceIn(0f, 1f)),
                    modifier = Modifier.padding(start = 16.dp).size(22.dp),
                )
            }
        }

        // Content offset by the drag amount.
        Box(
            modifier = Modifier
                .offset { IntOffset(offsetX.value.roundToInt(), 0) }
                .pointerInput(Unit) {
                    detectHorizontalDragGestures(
                        onDragEnd = {
                            scope.launch {
                                if (offsetX.value >= widthPx * threshold) {
                                    haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                                    onAddToQueue()
                                    android.widget.Toast.makeText(
                                        context,
                                        "Added to queue",
                                        android.widget.Toast.LENGTH_SHORT,
                                    ).show()
                                }
                                offsetX.animateTo(
                                    0f,
                                    spring(
                                        dampingRatio = Spring.DampingRatioMediumBouncy,
                                        stiffness = Spring.StiffnessMedium,
                                    ),
                                )
                            }
                        },
                        onDragCancel = {
                            scope.launch {
                                offsetX.animateTo(
                                    0f,
                                    spring(
                                        dampingRatio = Spring.DampingRatioMediumBouncy,
                                        stiffness = Spring.StiffnessMedium,
                                    ),
                                )
                            }
                        },
                        onHorizontalDrag = { change, dragAmount ->
                            change.consume()
                            val capped = (offsetX.value + dragAmount)
                                .coerceAtLeast(0f)
                                .coerceAtMost(widthPx * 0.6f)
                            scope.launch { offsetX.snapTo(capped) }
                        },
                    )
                },
        ) {
            content()
        }
    }
}

// Thin vertical scroll-position indicator for verticalScroll() containers (no built-in scrollbar).
@Composable
fun FirmiumVerticalScrollbar(
    scrollState: ScrollState,
    modifier: Modifier = Modifier,
) {
    val colors = LocalFirmiumColors.current
    if (scrollState.maxValue <= 0) return

    val viewport = scrollState.viewportSize.toFloat()
    val totalHeight = viewport + scrollState.maxValue
    val thumbFraction = (viewport / totalHeight).coerceIn(0.05f, 1f)
    val scrollFraction = scrollState.value.toFloat() / scrollState.maxValue

    Canvas(
        modifier = modifier
            .fillMaxHeight()
            .width(3.dp)
            .padding(vertical = 2.dp),
    ) {
        val thumbHeight = size.height * thumbFraction
        val thumbY = (size.height - thumbHeight) * scrollFraction
        drawRoundRect(
            color = colors.muted.copy(alpha = 0.4f),
            topLeft = androidx.compose.ui.geometry.Offset(0f, thumbY),
            size = androidx.compose.ui.geometry.Size(size.width, thumbHeight),
            cornerRadius = androidx.compose.ui.geometry.CornerRadius(size.width / 2, size.width / 2),
        )
    }
}
