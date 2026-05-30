package com.fossisawesome.firmium.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

// Full-screen overlay bottom sheet with smooth drag-to-dismiss.
// Renders directly in the composition tree (no Dialog window) to avoid platform animation
// conflicts that caused jitter. Slides up on enter, drag handle dismisses on downward swipe.
@Composable
fun FirmiumBottomSheet(
    onDismiss: () -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    val colors = LocalFirmiumColors.current
    // Start offscreen so the first frame is invisible; LaunchedEffect slides it in.
    val offsetY = remember { Animatable(2000f) }
    val scope = rememberCoroutineScope()
    var dismissing by remember { mutableStateOf(false) }

    fun animateDismiss() {
        if (dismissing) return
        dismissing = true
        scope.launch {
            offsetY.animateTo(2000f, tween(durationMillis = 280, easing = FastOutSlowInEasing))
            onDismiss()
        }
    }

    LaunchedEffect(Unit) {
        offsetY.animateTo(0f, tween(durationMillis = 320, easing = FastOutSlowInEasing))
    }

    BackHandler(enabled = !dismissing) { animateDismiss() }

    // Full-screen overlay — drawn on top of whatever is already in the composition tree.
    Box(modifier = Modifier.fillMaxSize()) {
        // Scrim — fades in with the sheet; tapping it dismisses.
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black.copy(alpha = 0.5f))
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    onClick = ::animateDismiss,
                ),
        )

        // Sheet panel slides from bottom; offset tracks both the open animation and drag gesture.
        Column(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .offset { IntOffset(0, offsetY.value.roundToInt()) }
                .clip(RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp))
                .background(colors.surface)
                .windowInsetsPadding(WindowInsets.navigationBars)
                // Consume taps so they don't fall through to the scrim.
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    onClick = {},
                ),
        ) {
            // Drag handle — downward drag to dismiss.
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 12.dp)
                    .pointerInput(Unit) {
                        awaitEachGesture {
                            val down = awaitFirstDown(requireUnconsumed = false)
                            val startY = down.position.y
                            val startOffset = offsetY.value
                            while (true) {
                                val event = awaitPointerEvent()
                                val change = event.changes.firstOrNull() ?: break
                                val dy = change.position.y - startY
                                if (!change.pressed) {
                                    if (offsetY.value > 200f) {
                                        animateDismiss()
                                    } else {
                                        scope.launch {
                                            offsetY.animateTo(
                                                0f,
                                                spring(
                                                    dampingRatio = Spring.DampingRatioMediumBouncy,
                                                    stiffness = Spring.StiffnessMedium,
                                                ),
                                            )
                                        }
                                    }
                                    break
                                }
                                scope.launch { offsetY.snapTo((startOffset + dy).coerceAtLeast(0f)) }
                                change.consume()
                            }
                        }
                    },
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .width(36.dp)
                        .height(4.dp)
                        .clip(RoundedCornerShape(2.dp))
                        .background(colors.surface2),
                )
            }

            content()
        }
    }
}
