package com.fossisawesome.firmium.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Pointer-input slider — replaces material3 Slider().
// Uses BoxWithConstraints so the thumb is positioned with an absolute offset,
// preventing it from being clipped or hidden at low/zero values.
@Composable
fun FirmiumSlider(
    value: Float,
    onValueChange: (Float) -> Unit,
    modifier: Modifier = Modifier,
    valueRange: ClosedFloatingPointRange<Float> = 0f..1f,
    trackColor: Color = LocalFirmiumColors.current.surface2,
    fillColor: Color = LocalFirmiumColors.current.accent,
    onValueChangeFinished: (() -> Unit)? = null,
) {
    val fraction = ((value - valueRange.start) / (valueRange.endInclusive - valueRange.start)).coerceIn(0f, 1f)
    val thumbDiameter = 14.dp
    val thumbRadius = thumbDiameter / 2

    BoxWithConstraints(
        modifier = modifier
            .height(20.dp)
            .pointerInput(valueRange) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    val w = size.width.toFloat()
                    fun posToValue(x: Float): Float {
                        val f = (x / w).coerceIn(0f, 1f)
                        return valueRange.start + f * (valueRange.endInclusive - valueRange.start)
                    }
                    onValueChange(posToValue(down.position.x))
                    while (true) {
                        val event = awaitPointerEvent()
                        val change = event.changes.firstOrNull() ?: break
                        if (!change.pressed) {
                            onValueChangeFinished?.invoke()
                            break
                        }
                        onValueChange(posToValue(change.position.x))
                        change.consume()
                    }
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        // Track + fill
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(4.dp)
                .clip(RoundedCornerShape(2.dp))
                .background(trackColor),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxWidth(fraction)
                    .fillMaxHeight()
                    .background(fillColor),
            )
        }
        // Thumb — absolute x so it's always fully visible, even at 0 or 1
        val thumbOffsetX = (maxWidth * fraction - thumbRadius).coerceIn(0.dp, maxWidth - thumbDiameter)
        Box(
            modifier = Modifier
                .align(Alignment.CenterStart)
                .offset(x = thumbOffsetX)
                .size(thumbDiameter)
                .clip(RoundedCornerShape(thumbRadius))
                .background(fillColor),
        )
    }
}

// Seek bar used by FullScreenPlayer. Same thumb-fix as FirmiumSlider.
@Composable
fun FirmiumSeekBar(
    progress: Float,
    onSeekStart: () -> Unit,
    onSeekUpdate: (Float) -> Unit,
    onSeekEnd: () -> Unit,
    trackColor: Color,
    fillColor: Color,
) {
    val fraction = progress.coerceIn(0f, 1f)
    val thumbDiameter = 12.dp
    val thumbRadius = thumbDiameter / 2

    BoxWithConstraints(
        modifier = Modifier
            .fillMaxWidth()
            .height(20.dp)
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    val w = size.width.toFloat()
                    onSeekStart()
                    onSeekUpdate((down.position.x / w).coerceIn(0f, 1f))
                    while (true) {
                        val event = awaitPointerEvent()
                        val change = event.changes.firstOrNull() ?: break
                        if (!change.pressed) { onSeekEnd(); break }
                        onSeekUpdate((change.position.x / w).coerceIn(0f, 1f))
                        change.consume()
                    }
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        // Track + fill
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(4.dp)
                .clip(RoundedCornerShape(2.dp))
                .background(trackColor),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxWidth(fraction)
                    .fillMaxHeight()
                    .background(fillColor),
            )
        }
        // Thumb — absolute x
        val thumbOffsetX = (maxWidth * fraction - thumbRadius).coerceIn(0.dp, maxWidth - thumbDiameter)
        Box(
            modifier = Modifier
                .align(Alignment.CenterStart)
                .offset(x = thumbOffsetX)
                .size(thumbDiameter)
                .clip(RoundedCornerShape(thumbRadius))
                .background(fillColor),
        )
    }
}
