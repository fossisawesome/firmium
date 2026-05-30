package com.fossisawesome.firmium.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Animated toggle switch — replaces material3 Switch().
@Composable
fun FirmiumSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = LocalFirmiumColors.current
    val trackColor by animateColorAsState(
        targetValue = if (checked) colors.accent.copy(alpha = 0.4f) else colors.surface2,
        animationSpec = tween(150),
        label = "track",
    )
    val thumbColor by animateColorAsState(
        targetValue = if (checked) colors.accent else colors.muted,
        animationSpec = tween(150),
        label = "thumb",
    )
    val thumbOffset by animateDpAsState(
        targetValue = if (checked) 16.dp else 0.dp,
        animationSpec = tween(150),
        label = "offset",
    )

    Box(
        modifier = modifier
            .width(40.dp)
            .height(24.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(trackColor)
            .clickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = null,
                onClick = { onCheckedChange(!checked) },
            ),
        contentAlignment = Alignment.CenterStart,
    ) {
        Box(
            modifier = Modifier
                .padding(start = thumbOffset + 3.dp)
                .size(18.dp)
                .clip(RoundedCornerShape(9.dp))
                .background(thumbColor),
        )
    }
}
